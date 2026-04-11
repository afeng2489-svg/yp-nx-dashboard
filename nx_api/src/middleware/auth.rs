//! API 密钥认证中间件

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use axum::body::Body;

use crate::config::ApiConfig;

/// API 密钥认证中间件
#[derive(Clone)]
pub struct ApiKeyAuth;

impl ApiKeyAuth {
    /// 认证中间件处理函数
    pub async fn middleware(
        State(config): State<ApiConfig>,
        mut request: Request<Body>,
        next: Next,
    ) -> Result<Response, StatusCode> {
        // 如果没有配置 API 密钥，则跳过认证
        let api_key = config.api_key.as_ref().ok_or(StatusCode::OK)?;

        // 从 Authorization 头获取 API 密钥
        let auth_header = request
            .headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok());

        let provided_key = auth_header
            .and_then(|h| h.strip_prefix("Bearer "))
            .or_else(|| auth_header);

        match provided_key {
            Some(key) if key == api_key => {
                Ok(next.run(request).await)
            }
            _ => Err(StatusCode::UNAUTHORIZED),
        }
    }
}