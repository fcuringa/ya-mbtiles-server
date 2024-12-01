use std::ffi::CString;
use std::fs;
use std::ops::{Add, Not};
use std::time::{Duration, Instant};
use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{Error, HttpResponse};
use actix_web::error::HttpError;
use actix_web::middleware::{Logger, Next};
use actix_web::web::{head, Data};
use pyo3::impl_::wrap::SomeWrap;
use pyo3::prelude::{PyAnyMethods, PyModule};
use pyo3::Python;
use pyo3::types::IntoPyDict;
use pyo3_ffi::c_str;
use log::{debug, log};
use crate::app_conf::{AppState, AuthState};
use crate::error::UnauthorizedAccess;

pub(crate) async fn auth_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let mut is_allowed = false;
    {
        let app_data = req.app_data::<Data<AppState>>().unwrap();
        let script_text = CString::new(fs::read_to_string(app_data.auth_script.as_str()).unwrap());
        let req_path = req.path();

        // Acquire lock on cache
        let mut auth_cache = app_data.auth_cache.lock().unwrap();
        let now = Instant::now();
        // Remove expired entries
        auth_cache.retain(|k, v| v.exp_time > now );


        let mut user_session_hash = "".to_string();
        let mut headers = Vec::<(String, String)>::new();

        for header in app_data.auth_headers.iter() {
            let value = req.headers().get(header);
            if value.is_none() {
                // One of the auth headers is not found so exit here with 401
                return Err(Error::from(UnauthorizedAccess { name: "Unauthorized" }));
            } else {
                // Header found so append to session hash
                user_session_hash = String::new()
                    .add(user_session_hash.as_str())
                    .add(value.unwrap().to_str().unwrap());
                headers.push((header.to_string(), value.unwrap().to_str().unwrap().to_string()));
            }
        }

        let mut is_check_required = true;

        // No auth required
        if user_session_hash == "" {
            is_check_required = false;
            is_allowed = true;
        }

        // Try to get the session from the cache
        let user_auth_state = auth_cache.get(user_session_hash.as_str());

        if user_auth_state.is_some() {
            // Found session hash in the cache
            let auth_state = user_auth_state.unwrap();
            if now < auth_state.exp_time {
                debug!("Cache hit for {}", user_session_hash);
                is_check_required = false;
                is_allowed = auth_state.is_allowed;
            } else {
                debug!("Cache hit but expired for {}", user_session_hash);
            }
        }

        // Check if calling the script is required
        if is_check_required {
            Python::with_gil(|py| {
                let callable = PyModule::from_code(
                    py,
                    script_text.unwrap().as_c_str(),
                    c_str!("g_auth.py"),
                    c_str!("g_auth")
                );
                let kwargs = headers.into_py_dict(py).unwrap();
                let auth_result: bool = callable.unwrap().getattr("auth")
                    .unwrap().call((req_path,), Some(&kwargs))
                    .unwrap().extract().unwrap();

                // Print and save result to cache
                debug!("Result from auth script: {:?}", auth_result);
                auth_cache.insert(user_session_hash.to_string(), AuthState { is_allowed: auth_result, exp_time: Instant::now().add(Duration::from_secs(60)) });
                is_allowed = auth_result;
            });
        }
    }

    if !is_allowed {
        return Err(Error::from(UnauthorizedAccess { name: "Unauthorized" }));
    }

    // Proceed to serving
    next.call(req).await
}