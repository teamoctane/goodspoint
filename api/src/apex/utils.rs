use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Serialize)]
pub struct ErrorMessage {
    pub status: u16,
    pub error: String,
}

impl ErrorMessage {
    pub fn new(status: StatusCode, error: &str) -> Self {
        ErrorMessage {
            status: status.as_u16(),
            error: error.to_string(),
        }
    }
}

#[derive(Debug)]
pub enum VerboseHTTPError {
    Standard(StatusCode, String),
}

impl IntoResponse for VerboseHTTPError {
    fn into_response(self) -> Response {
        match self {
            VerboseHTTPError::Standard(status, message) => {
                let error_message = ErrorMessage::new(status, &message);
                let body = axum::Json(error_message);
                (status, body).into_response()
            }
        }
    }
}
