//! API Error Types

use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// API Result type
pub type ApiResult<T> = Result<T, ApiError>;

/// API error types
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Message not found: {0}")]
    MessageNotFound(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            ApiError::NotFound(msg) => (axum::http::StatusCode::NOT_FOUND, msg.clone()),
            ApiError::BadRequest(msg) => (axum::http::StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::Internal(msg) => {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
            }
            ApiError::Unauthorized => {
                (axum::http::StatusCode::UNAUTHORIZED, "Unauthorized".to_string())
            }
            ApiError::Forbidden => {
                (axum::http::StatusCode::FORBIDDEN, "Forbidden".to_string())
            }
            ApiError::SessionNotFound(msg) => {
                (axum::http::StatusCode::NOT_FOUND, msg.clone())
            }
            ApiError::MessageNotFound(msg) => {
                (axum::http::StatusCode::NOT_FOUND, msg.clone())
            }
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}