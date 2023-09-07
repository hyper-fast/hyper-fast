#[allow(unused_imports)]
use log::{debug, error, info, warn};
use thiserror::Error;

use crate::server::{HttpResponse, HttpResult};

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Internal Server Error: {0}")]
    InternalServerError(anyhow::Error),

    #[error("Not Found Error: {0}")]
    NotFound(String),

    #[error("Forbidden Error: {0}")]
    Forbidden(String),

    #[error("Bad Request Error: {0}")]
    BadRequest(#[from] anyhow::Error),

    #[error("Not Content: {0}")]
    NoContent(String),
}

impl Into<HttpResult> for ApiError {
    fn into(self) -> HttpResult {
        match self {
            ApiError::InternalServerError(error) => HttpResponse::internal_server_error(error),
            ApiError::NotFound(reason) => HttpResponse::not_found(&reason),
            ApiError::Forbidden(reason) => HttpResponse::forbidden(&reason),
            ApiError::BadRequest(error) => HttpResponse::bad_request(error),
            ApiError::NoContent(reason) => HttpResponse::no_content(&reason),
        }
    }
}
