//! 快照 API 路由

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::routes::{AppState, resolve_project_id};
use crate::services::team_evolution::error::TeamEvolutionError;
use crate::services::team_evolution::snapshot_repository::{
    ProjectProgress, RoleSnapshot, RoleSnapshotHistory,
};

fn map_tev_error(err: TeamEvolutionError) -> (StatusCode, Json<serde_json::Value>) {
    let msg = err.to_string();
    let status = match &err {
        TeamEvolutionError::SnapshotNotFound(_) => StatusCode::NOT_FOUND,
        TeamEvolutionError::FeatureDisabled(_) => StatusCode::FORBIDDEN,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(serde_json::json!({ "error": msg })))
}

/// GET /api/v1/projects/:id/progress
pub async fn get_project_progress(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
) -> Result<Json<Option<ProjectProgress>>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.snapshot_service.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "Snapshot service not available" }))))?;
    let resolved_id = resolve_project_id(&state, &project_id);
    let progress = service.get_project_progress(&resolved_id).map_err(map_tev_error)?;
    Ok(Json(progress))
}

/// GET /api/v1/projects/:id/role-snapshots
pub async fn get_role_snapshots(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<RoleSnapshot>>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.snapshot_service.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "Snapshot service not available" }))))?;
    let resolved_id = resolve_project_id(&state, &project_id);
    let snaps = service.get_role_snapshots(&resolved_id).map_err(map_tev_error)?;
    Ok(Json(snaps))
}

/// GET /api/v1/projects/:id/role-snapshots/:role_id
pub async fn get_role_snapshot(
    State(state): State<Arc<AppState>>,
    Path((project_id, role_id)): Path<(String, String)>,
) -> Result<Json<Option<RoleSnapshot>>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.snapshot_service.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "Snapshot service not available" }))))?;
    let resolved_id = resolve_project_id(&state, &project_id);
    let snap = service.get_role_snapshot(&resolved_id, &role_id).map_err(map_tev_error)?;
    Ok(Json(snap))
}

/// GET /api/v1/projects/:id/role-snapshots/:role_id/history
pub async fn get_role_snapshot_history(
    State(state): State<Arc<AppState>>,
    Path((project_id, role_id)): Path<(String, String)>,
) -> Result<Json<Vec<RoleSnapshotHistory>>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.snapshot_service.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "Snapshot service not available" }))))?;
    let history = service.get_role_history(&resolve_project_id(&state, &project_id), &role_id).map_err(map_tev_error)?;
    Ok(Json(history))
}

/// POST /api/v1/projects/:id/snapshot-all
pub async fn snapshot_all_active(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.snapshot_service.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "Snapshot service not available" }))))?;
    let resolved_id = resolve_project_id(&state, &project_id);
    let count = service.snapshot_all_active(&resolved_id).map_err(map_tev_error)?;
    Ok(Json(serde_json::json!({ "saved": count })))
}
