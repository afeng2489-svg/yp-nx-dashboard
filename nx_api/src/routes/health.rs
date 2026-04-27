//! 健康检查路由

use axum::Json;
use serde_json::{json, Value};

/// 健康检查处理器
pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "nexusflow-api",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
