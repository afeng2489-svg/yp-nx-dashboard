//! 健康检查路由

use crate::response::{ok, ApiOk};
use crate::routes::AppState;
use axum::{routing::get, Router};
use serde_json::json;
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/health", get(health_check))
}

pub async fn health_check() -> ApiOk<serde_json::Value> {
    ok(json!({
        "status": "ok",
        "service": "nexusflow-api",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}
