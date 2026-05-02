//! 健康检查路由

use crate::response::{ok, ApiOk};
use serde_json::json;

/// 健康检查处理器
pub async fn health_check() -> ApiOk<serde_json::Value> {
    ok(json!({
        "status": "ok",
        "service": "nexusflow-api",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
