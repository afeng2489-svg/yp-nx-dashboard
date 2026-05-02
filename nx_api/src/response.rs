//! Unified API response types.
//!
//! All endpoints return `{ ok, data?, error?, meta? }`.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// Standard API response envelope.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
            meta: None,
        }
    }

    pub fn ok_with_meta(data: T, meta: serde_json::Value) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
            meta: Some(meta),
        }
    }
}

/// Successful response alias: `Json<ApiResponse<T>>`.
pub type ApiOk<T> = Json<ApiResponse<T>>;

/// Build a success response.
pub fn ok<T: Serialize>(data: T) -> ApiOk<T> {
    Json(ApiResponse::ok(data))
}

/// Build a success response with metadata.
pub fn ok_with_meta<T: Serialize>(data: T, meta: serde_json::Value) -> ApiOk<T> {
    Json(ApiResponse::ok_with_meta(data, meta))
}

/// Error response that implements IntoResponse.
#[derive(Debug)]
pub struct ApiErrorResponse {
    pub status: StatusCode,
    pub message: String,
}

impl ApiErrorResponse {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiErrorResponse {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "ok": false,
            "error": self.message,
        });
        (self.status, Json(body)).into_response()
    }
}

impl From<crate::error::ApiError> for ApiErrorResponse {
    fn from(err: crate::error::ApiError) -> Self {
        use crate::error::ApiError;
        let status = match &err {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden => StatusCode::FORBIDDEN,
            ApiError::SessionNotFound(_) => StatusCode::NOT_FOUND,
            ApiError::MessageNotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        ApiErrorResponse::new(status, err.to_string())
    }
}

/// Result type using the envelope error.
pub type ApiResult<T> = Result<T, ApiErrorResponse>;
