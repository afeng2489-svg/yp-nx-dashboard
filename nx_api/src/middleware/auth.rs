//! API 密钥认证中间件

use axum::body::Body;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use crate::routes::AppState;

/// API 密钥认证中间件
#[derive(Clone)]
pub struct ApiKeyAuth;

impl ApiKeyAuth {
    /// 认证中间件处理函数（使用 Arc<AppState>）
    pub async fn middleware(
        State(state): State<Arc<AppState>>,
        request: Request<Body>,
        next: Next,
    ) -> Result<Response, StatusCode> {
        // 如果没有配置 API 密钥，则跳过认证
        let api_key = match &state.api_key_config {
            Some(key) => key,
            None => return Ok(next.run(request).await),
        };

        // 从 Authorization 头获取 API 密钥
        let auth_header = request
            .headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok());

        let provided_key = auth_header
            .and_then(|h| h.strip_prefix("Bearer "))
            .or_else(|| auth_header);

        match provided_key {
            Some(key) if key == api_key => Ok(next.run(request).await),
            _ => Err(StatusCode::UNAUTHORIZED),
        }
    }
}
