use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct AppState {
    pub(crate) auth_script: String,
    pub(crate) webroot: String,
    pub(crate) auth_headers: Vec<String>,
    pub(crate) cache_validity_seconds: u64,
    pub(crate) cache_control_header: String,
    pub(crate) auth_cache: Arc<Mutex<HashMap<String, AuthState>>>
}

#[derive(Debug)]
pub struct AuthState {
    pub is_allowed: bool,
    pub exp_time: Instant
}