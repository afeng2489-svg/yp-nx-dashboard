use crate::routes::AppState;
use crate::services::execution_log_service::ExecutionLog;
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/api/v1/executions/:id/logs", get(list_logs))
}

async fn list_logs(
    State(state): State<Arc<AppState>>,
    Path(execution_id): Path<String>,
) -> Json<Vec<ExecutionLog>> {
    Json(state.execution_log_service.list_by_execution(&execution_id))
}
