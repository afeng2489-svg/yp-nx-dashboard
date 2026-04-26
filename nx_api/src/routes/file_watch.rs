//! File Watch API 路由

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::routes::{AppState, resolve_project_id};

/// GET /api/v1/projects/:id/file-changes
pub async fn get_file_changes(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let watcher = state.file_watcher.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "File watcher not available" }))))?;

    if !watcher.is_enabled() {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({ "error": "File watcher is disabled" }))));
    }

    let resolved_id = resolve_project_id(&state, &project_id);
    let changes = watcher.get_recent_changes(&resolved_id);
    Ok(Json(serde_json::json!({
        "project_id": project_id,
        "changes": changes,
        "count": changes.len(),
    })))
}

/// POST /api/v1/projects/:id/file-watch/start
pub async fn start_file_watch(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let watcher = state.file_watcher.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "File watcher not available" }))))?;

    if !watcher.is_enabled() {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({ "error": "File watcher is disabled" }))));
    }

    let working_dir = state.current_workspace_path.read().clone();
    let workspace = working_dir.unwrap_or_default();

    if workspace.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "No workspace path set" }))));
    }

    // The watcher's start_watching uses an internal callback that records changes
    // into recent_changes automatically, so we pass a no-op callback here.
    // The notify watcher inside FileWatcher already records to recent_changes.
    let resolved_id = resolve_project_id(&state, &project_id);
    match watcher.start_watching(&resolved_id, &workspace, Box::new(|_change| {
        // Changes are already recorded internally by the watcher
    })) {
        Ok(()) => Ok(Json(serde_json::json!({
            "project_id": project_id,
            "status": "watching",
            "workspace": workspace,
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() })))),
    }
}

/// POST /api/v1/projects/:id/file-watch/stop
pub async fn stop_file_watch(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let watcher = state.file_watcher.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "File watcher not available" }))))?;

    let resolved_id = resolve_project_id(&state, &project_id);
    watcher.stop_watching(&resolved_id);
    Ok(Json(serde_json::json!({
        "project_id": project_id,
        "status": "stopped",
    })))
}
