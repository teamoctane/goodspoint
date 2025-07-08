use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Serialize)]
pub struct ErrorMessage {
    pub status: &'static str,
    pub message: String,
}

impl ErrorMessage {
    #[inline]
    pub fn new(_status: StatusCode, message: String) -> Self {
        Self {
            status: "error",
            message,
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
            Self::Standard(status, message) => {
                let error_message = ErrorMessage::new(status, message);
                (status, axum::Json(error_message)).into_response()
            }
        }
    }
}
