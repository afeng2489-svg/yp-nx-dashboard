//! Scheduler API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::{Arc, RwLock};

use crate::routes::AppState;
use crate::scheduler::{CreateTaskRequest, QueueStats, SchedulerService, TaskResponse};
use crate::services::ExecutionService;

/// Scheduler state wrapper for route handlers
pub struct SchedulerState {
    pub scheduler: RwLock<Option<SchedulerService>>,
}

impl SchedulerState {
    pub fn new() -> Self {
        Self {
            scheduler: RwLock::new(None),
        }
    }

    pub fn init(&self, execution_service: Option<ExecutionService>) {
        let service = SchedulerService::new(execution_service);
        *self.scheduler.write().unwrap() = Some(service);
    }

    pub fn start_workers(&self, num_workers: u32) {
        if let Some(scheduler) = self.scheduler.read().unwrap().as_ref() {
            scheduler.start_workers(num_workers);
        }
    }
}

impl Default for SchedulerState {
    fn default() -> Self {
        Self::new()
    }
}

/// POST /api/v1/tasks - Enqueue a new task
pub async fn create_task(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateTaskRequest>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let guard = state
        .scheduler_state
        .scheduler
        .read()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let scheduler = guard.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let task = scheduler.submit_task(request);
    let response = TaskResponse::from(&task);

    Ok(Json(response))
}

/// GET /api/v1/tasks - List all tasks
pub async fn list_tasks(State(state): State<Arc<AppState>>) -> Result<Json<Value>, StatusCode> {
    let guard = state
        .scheduler_state
        .scheduler
        .read()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let scheduler = guard.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let tasks: Vec<TaskResponse> = scheduler
        .list_tasks()
        .iter()
        .map(TaskResponse::from)
        .collect();

    let stats = scheduler.get_stats();

    Ok(Json(json!({
        "tasks": tasks,
        "stats": stats,
    })))
}

/// GET /api/v1/tasks/:id - Get task status
pub async fn get_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let guard = state
        .scheduler_state
        .scheduler
        .read()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let scheduler = guard.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let task = scheduler.get_task(&id).ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(TaskResponse::from(&task)))
}

/// DELETE /api/v1/tasks/:id - Cancel a task
pub async fn cancel_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let guard = state
        .scheduler_state
        .scheduler
        .read()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let scheduler = guard.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    if scheduler.cancel_task(&id) {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// GET /api/v1/tasks/stats - Get queue statistics
pub async fn get_stats(State(state): State<Arc<AppState>>) -> Result<Json<QueueStats>, StatusCode> {
    let guard = state
        .scheduler_state
        .scheduler
        .read()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let scheduler = guard.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    Ok(Json(scheduler.get_stats()))
}

/// Create scheduler routes
pub fn create_scheduler_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/tasks", axum::routing::post(create_task))
        .route("/api/v1/tasks", axum::routing::get(list_tasks))
        .route("/api/v1/tasks/stats", axum::routing::get(get_stats))
        .route("/api/v1/tasks/:id", axum::routing::get(get_task))
        .route("/api/v1/tasks/:id", axum::routing::delete(cancel_task))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_task_request_serialization() {
        let json = r#"{
            "task_type": "workflow_execution",
            "payload": {"workflow_id": "test-123"},
            "delay_seconds": 60,
            "max_retries": 3
        }"#;

        let request: CreateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            request.task_type,
            crate::scheduler::TaskType::WorkflowExecution
        );
        assert_eq!(request.delay_seconds, Some(60));
        assert_eq!(request.max_retries, 3);
    }

    #[test]
    fn test_task_response_serialization() {
        use crate::scheduler::{ScheduledTask, TaskType};

        let task = ScheduledTask::new(
            TaskType::CodeReview,
            serde_json::json!({"repo_url": "https://github.com/test/repo"}),
            chrono::Utc::now(),
            3,
        );

        let response = TaskResponse::from(&task);
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"id\""));
        assert!(json.contains("\"task_type\":\"code_review\""));
        assert!(json.contains("\"status\":\"pending\""));
    }
}
