//! 进程管理 API 路由

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::routes::{AppState, resolve_project_id};
use crate::services::team_evolution::process_lifecycle::{ProcessStats, ProcessLifecycleEvent};
use crate::services::team_evolution::error::TeamEvolutionError;

fn map_tev_error(err: TeamEvolutionError) -> (StatusCode, Json<serde_json::Value>) {
    let msg = err.to_string();
    let status = match &err {
        TeamEvolutionError::ResourceLimitReached { .. } => StatusCode::CONFLICT,
        TeamEvolutionError::FeatureDisabled(_) => StatusCode::FORBIDDEN,
        TeamEvolutionError::ProcessNotFound(_) => StatusCode::NOT_FOUND,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(serde_json::json!({ "error": msg })))
}

/// GET /api/v1/processes/stats
pub async fn get_process_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ProcessStats>, (StatusCode, Json<serde_json::Value>)> {
    let manager = state.process_lifecycle.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "Process lifecycle not available" }))))?;
    Ok(Json(manager.get_stats()))
}

/// POST /api/v1/projects/:id/processes/cleanup
pub async fn cleanup_project_processes(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let manager = state.process_lifecycle.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "Process lifecycle not available" }))))?;

    let resolved_id = resolve_project_id(&state, &project_id);
    let terminated = manager.cleanup_project_processes(&resolved_id);
    Ok(Json(serde_json::json!({
        "project_id": project_id,
        "terminated_count": terminated.len(),
        "terminated_executions": terminated,
    })))
}

/// POST /api/v1/processes/:execution_id/hibernate
pub async fn hibernate_process(
    State(state): State<Arc<AppState>>,
    Path(execution_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let manager = state.process_lifecycle.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "Process lifecycle not available" }))))?;

    // 发送 Hibernated 事件
    let event_tx = state.agent_execution_manager.event_sender();
    let _ = event_tx.send(crate::ws::agent_execution::AgentExecutionEvent::Hibernated {
        execution_id: execution_id.clone(),
        idle_secs: 0,
    });

    manager.unregister_process(&execution_id);
    Ok(Json(serde_json::json!({ "hibernated": execution_id })))
}

/// POST /api/v1/processes/:execution_id/wake
pub async fn wake_process(
    State(state): State<Arc<AppState>>,
    Path(execution_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let manager = state.process_lifecycle.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "Process lifecycle not available" }))))?;

    // Touch to reset idle timer
    manager.touch(&execution_id);
    Ok(Json(serde_json::json!({ "woken": execution_id })))
}
