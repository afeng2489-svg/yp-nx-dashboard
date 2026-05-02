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
        let status = match &self {
            ApiError::NotFound(_) | ApiError::SessionNotFound(_) | ApiError::MessageNotFound(_) => {
                axum::http::StatusCode::NOT_FOUND
            }
            ApiError::BadRequest(_) => axum::http::StatusCode::BAD_REQUEST,
            ApiError::Internal(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Unauthorized => axum::http::StatusCode::UNAUTHORIZED,
            ApiError::Forbidden => axum::http::StatusCode::FORBIDDEN,
        };

        let body = Json(json!({
            "ok": false,
            "error": self.to_string()
        }));

        (status, body).into_response()
    }
}
