//! Scheduler API routes — backed by core/orchestrator TaskScheduler

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use nexus_orchestrator::{
    CliManager, MessageBus, QueueStatus, QueuedTask, SchedulerStats, TaskPriority, TaskScheduler,
    TeamId, TeamManager, WorkflowDefinition,
};

use crate::routes::AppState;

/// Wrapper to hold the orchestrator TaskScheduler in AppState
pub struct OrchestratorScheduler {
    pub scheduler: Arc<TaskScheduler>,
}

impl OrchestratorScheduler {
    pub fn new(db_path: &str) -> anyhow::Result<Self> {
        let message_bus = Arc::new(MessageBus::new());
        let team_manager = Arc::new(TeamManager::new(message_bus.clone()));
        let cli_manager = Arc::new(CliManager::new());

        let scheduler = TaskScheduler::new(cli_manager, team_manager, message_bus, 4);
        scheduler
            .init_database(db_path)
            .map_err(|e| anyhow::anyhow!("Failed to init scheduler database: {e}"))?;

        Ok(Self {
            scheduler: Arc::new(scheduler),
        })
    }

    /// Start the scheduler event loop as a background task
    pub fn start_background(self: &Arc<Self>) {
        let scheduler = self.scheduler.clone();
        tokio::spawn(async move {
            scheduler.run().await;
        });
    }
}

/// POST /api/v1/tasks - Enqueue a new task
#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub stages: Vec<StageRequest>,
    #[serde(default)]
    pub variables: std::collections::HashMap<String, Value>,
    #[serde(default = "default_priority")]
    pub priority: String,
}

#[derive(Debug, Deserialize)]
pub struct StageRequest {
    pub name: String,
    pub agents: Vec<String>,
    #[serde(default)]
    pub prompt_template: String,
    #[serde(default)]
    pub parallel: bool,
}

fn default_priority() -> String {
    "normal".to_string()
}

#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub name: String,
    pub status: String,
    pub priority: String,
    pub retry_count: u32,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub error: Option<String>,
}

impl From<&QueuedTask> for TaskResponse {
    fn from(task: &QueuedTask) -> Self {
        Self {
            id: task.id.to_string(),
            name: task.workflow.name.clone(),
            status: task.status.to_string(),
            priority: task.priority.to_string(),
            retry_count: task.retry_count,
            created_at: task.created_at.to_rfc3339(),
            started_at: task.started_at.map(|t| t.to_rfc3339()),
            finished_at: task.finished_at.map(|t| t.to_rfc3339()),
            error: task.error.clone(),
        }
    }
}

/// POST /api/v1/tasks - Enqueue a new task
pub async fn create_task(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateTaskRequest>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let scheduler = &state.task_scheduler.scheduler;

    let workflow = WorkflowDefinition {
        id: uuid::Uuid::new_v4(),
        name: request.name,
        description: request.description,
        stages: request
            .stages
            .into_iter()
            .map(|s| nexus_orchestrator::StageDefinition {
                name: s.name,
                agents: s.agents,
                parallel: s.parallel,
                continue_on_error: false,
                prompt_template: s.prompt_template,
            })
            .collect(),
    };

    let priority = match request.priority.to_lowercase().as_str() {
        "high" => TaskPriority::High,
        "critical" => TaskPriority::Critical,
        "low" => TaskPriority::Low,
        _ => TaskPriority::Normal,
    };

    let team_id = TeamId::new();
    let task_id = scheduler.enqueue(workflow, team_id, request.variables, priority);

    // Fetch the enqueued task to build response
    let task = scheduler
        .get_task(task_id)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(TaskResponse::from(&task)))
}

/// GET /api/v1/tasks - List all tasks
pub async fn list_tasks(State(state): State<Arc<AppState>>) -> Result<Json<Value>, StatusCode> {
    let scheduler = &state.task_scheduler.scheduler;

    let tasks: Vec<TaskResponse> = scheduler
        .list_tasks()
        .iter()
        .map(TaskResponse::from)
        .collect();

    let stats = scheduler.get_stats();

    Ok(Json(json!({
        "tasks": tasks,
        "stats": {
            "queued": stats.queued_count,
            "running": stats.running_count,
            "completed": stats.completed_count,
            "failed": stats.failed_count,
            "scheduled_jobs": stats.scheduled_jobs_count,
        },
    })))
}

/// GET /api/v1/tasks/:id - Get task by ID
pub async fn get_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let scheduler = &state.task_scheduler.scheduler;

    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let task = scheduler.get_task(uuid).ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(TaskResponse::from(&task)))
}

/// DELETE /api/v1/tasks/:id - Cancel a task
pub async fn cancel_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let scheduler = &state.task_scheduler.scheduler;

    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    if scheduler.cancel_task(uuid) {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// GET /api/v1/tasks/stats - Get queue statistics
pub async fn get_stats(State(state): State<Arc<AppState>>) -> Result<Json<Value>, StatusCode> {
    let scheduler = &state.task_scheduler.scheduler;
    let stats = scheduler.get_stats();

    Ok(Json(json!({
        "queued": stats.queued_count,
        "running": stats.running_count,
        "completed": stats.completed_count,
        "failed": stats.failed_count,
        "scheduled_jobs": stats.scheduled_jobs_count,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_task_request_deserialization() {
        let data = json!({
            "name": "test-task",
            "description": "A test task",
            "stages": [
                {
                    "name": "run",
                    "agents": ["claude"],
                    "prompt_template": "do something"
                }
            ],
            "variables": {},
            "priority": "high"
        });

        let req: CreateTaskRequest = serde_json::from_value(data).unwrap();
        assert_eq!(req.name, "test-task");
        assert_eq!(req.priority, "high");
        assert_eq!(req.stages.len(), 1);
    }

    #[test]
    fn test_task_response_from_queued_task() {
        use chrono::Utc;

        let task = QueuedTask {
            id: uuid::Uuid::new_v4(),
            workflow: WorkflowDefinition {
                id: uuid::Uuid::new_v4(),
                name: "my-workflow".to_string(),
                description: "desc".to_string(),
                stages: vec![],
            },
            team_id: TeamId::new(),
            variables: Default::default(),
            priority: TaskPriority::Normal,
            status: QueueStatus::Queued,
            retry_count: 0,
            retry_config: Default::default(),
            timeout_secs: None,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
            result: None,
            error: None,
        };

        let resp = TaskResponse::from(&task);
        assert_eq!(resp.name, "my-workflow");
        assert_eq!(resp.status, "queued");
        assert_eq!(resp.priority, "normal");
    }
}
