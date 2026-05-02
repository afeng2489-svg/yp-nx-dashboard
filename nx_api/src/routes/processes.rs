//! Process monitoring routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::routes::AppState;

/// Process info response
#[derive(Debug, serde::Serialize)]
pub struct ProcessInfo {
    pub execution_id: String,
    pub pid: Option<u32>,
    pub role_id: String,
    pub role_name: String,
    pub team_id: String,
    pub task: String,
    pub start_time: String,
    pub elapsed_secs: u64,
    pub status: String,
    pub output: String,
}

/// List all running processes
pub async fn list_processes(State(state): State<Arc<AppState>>) -> Json<Vec<ProcessInfo>> {
    let processes = state.teams_state.agent_team_service.get_processes();

    let infos: Vec<ProcessInfo> = processes
        .into_iter()
        .map(|p| {
            let elapsed = p.start_time.elapsed().as_secs();
            ProcessInfo {
                execution_id: p.role_id.clone(), // Use role_id as execution_id for now
                pid: p.pid,
                role_id: p.role_id,
                role_name: p.role_name,
                team_id: p.team_id,
                task: p.task,
                start_time: format!("{:?}", p.start_time),
                elapsed_secs: elapsed,
                status: format!("{:?}", p.status).to_lowercase(),
                output: p.output,
            }
        })
        .collect();

    Json(infos)
}

/// Kill a running process
pub async fn kill_process(
    State(state): State<Arc<AppState>>,
    Path(execution_id): Path<String>,
) -> Result<Json<KillResponse>, AppError> {
    // Try to kill the process
    if let Err(e) = state
        .teams_state
        .agent_team_service
        .kill_process(&execution_id)
    {
        return Err(AppError {
            status: StatusCode::NOT_FOUND,
            message: e,
        });
    }

    Ok(Json(KillResponse {
        success: true,
        message: format!("Process {} killed", execution_id),
    }))
}

#[derive(Debug, serde::Serialize)]
pub struct KillResponse {
    pub success: bool,
    pub message: String,
}

/// App error for process routes
#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let body = serde_json::json!({ "ok": false, "error": self.message });
        (self.status, Json(body)).into_response()
    }
}
