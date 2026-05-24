//! 预览服务器路由

use axum::extract::{Query, State};
use axum::{Json, Router};
use std::sync::Arc;

use crate::routes::AppState;
use crate::services::preview_server::{PreviewServerManager, PreviewStatus};

use serde::{Deserialize, Serialize};

/// 启动预览请求
#[derive(Debug, Deserialize)]
pub struct StartPreviewRequest {
    pub project_id: String,
    pub project_path: String,
}

/// 停止预览请求
#[derive(Debug, Deserialize)]
pub struct StopPreviewRequest {
    pub session_id: String,
}

/// 预览状态查询参数
#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    pub session_id: String,
}

/// 预览状态响应
#[derive(Debug, Serialize)]
pub struct PreviewStatusResponse {
    pub status: PreviewStatus,
    pub port: Option<u16>,
    pub url: Option<String>,
    pub started_at: Option<String>,
}

/// 启动预览服务器
async fn start_preview(
    State(state): State<Arc<AppState>>,
    Json(body): Json<StartPreviewRequest>,
) -> Json<serde_json::Value> {
    match state.preview_manager.start(&body.project_id, &body.project_path).await {
        Ok(info) => Json(serde_json::json!({
            "session_id": info.session_id,
            "port": info.port,
            "url": info.preview_url,
        })),
        Err(e) => Json(serde_json::json!({
            "error": e.to_string(),
        })),
    }
}

/// 停止预览服务器
async fn stop_preview(
    State(state): State<Arc<AppState>>,
    Json(body): Json<StopPreviewRequest>,
) -> Json<serde_json::Value> {
    let stopped = state.preview_manager.stop(&body.session_id).await;
    Json(serde_json::json!({ "stopped": stopped }))
}

/// 查询预览状态
async fn preview_status(
    State(state): State<Arc<AppState>>,
    Query(query): Query<StatusQuery>,
) -> Json<serde_json::Value> {
    match state.preview_manager.status(&query.session_id) {
        Some(info) => Json(serde_json::json!({
            "status": info.status,
            "port": info.port,
            "url": info.preview_url,
            "session_id": info.session_id,
        })),
        None => Json(serde_json::json!({
            "status": "not_found",
            "port": null,
            "url": null,
        })),
    }
}

/// 创建预览路由
pub fn preview_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/preview/start", axum::routing::post(start_preview))
        .route("/api/v1/preview/stop", axum::routing::post(stop_preview))
        .route("/api/v1/preview/status", axum::routing::get(preview_status))
}
