use std::fmt::{Display};
use actix_web::{error};
use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
pub struct UnauthorizedAccess {
    pub(crate) name: &'static str,
}

// Use default implementation for `error_response()` method
impl error::ResponseError for UnauthorizedAccess {}