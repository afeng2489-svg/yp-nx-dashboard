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

// ============ 定时任务 CRUD ============

#[derive(Debug, Deserialize)]
pub struct ScheduleTaskRequest {
    pub workflow_id: String,
    pub cron_expr: String,
    #[serde(default)]
    pub variables: std::collections::HashMap<String, Value>,
}

#[derive(Debug, Serialize)]
pub struct ScheduledJobResponse {
    pub id: String,
    pub workflow_name: String,
    pub cron_expr: String,
    pub enabled: bool,
    pub next_run: Option<String>,
    pub created_at: String,
}

/// POST /api/v1/tasks/schedule - 创建定时任务
pub async fn create_scheduled_job(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ScheduleTaskRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let scheduler = &state.task_scheduler.scheduler;

    // 从 DB 获取 workflow
    let workflow = state
        .workflow_service
        .get_workflow(&request.workflow_id)
        .map_err(|_| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!("工作流 {} 不存在", request.workflow_id)})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!("工作流 {} 不存在", request.workflow_id)})),
            )
        })?;

    // 转换为 orchestrator WorkflowDefinition
    let stages = workflow
        .definition
        .get("stages")
        .and_then(|s| s.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| {
                    Some(nexus_orchestrator::StageDefinition {
                        name: s.get("name")?.as_str()?.to_string(),
                        agents: s
                            .get("agents")
                            .and_then(|a| a.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        parallel: s.get("parallel").and_then(|p| p.as_bool()).unwrap_or(false),
                        continue_on_error: false,
                        prompt_template: s
                            .get("prompt_template")
                            .and_then(|p| p.as_str())
                            .unwrap_or("")
                            .to_string(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let orch_workflow = WorkflowDefinition {
        id: uuid::Uuid::new_v4(),
        name: workflow.name.clone(),
        description: workflow.description.unwrap_or_default(),
        stages,
    };

    let team_id = TeamId::new();
    let job_id = scheduler
        .add_scheduled_job(
            orch_workflow,
            team_id,
            request.variables,
            &request.cron_expr,
        )
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("创建定时任务失败: {}", e)})),
            )
        })?;

    Ok(Json(json!({
        "id": job_id.to_string(),
        "workflow_name": workflow.name,
        "cron_expr": request.cron_expr,
    })))
}

/// GET /api/v1/tasks/scheduled - 列出所有定时任务
pub async fn list_scheduled_jobs(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    let scheduler = &state.task_scheduler.scheduler;
    let jobs = scheduler.list_scheduled_jobs();

    let responses: Vec<ScheduledJobResponse> = jobs
        .iter()
        .map(|job| ScheduledJobResponse {
            id: job.id.to_string(),
            workflow_name: job.workflow.name.clone(),
            cron_expr: job.cron_expr.clone(),
            enabled: job.enabled,
            next_run: Some(job.next_run.to_rfc3339()),
            created_at: job.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(json!({"scheduled_jobs": responses})))
}

/// PUT /api/v1/tasks/scheduled/:id/toggle - 启用/禁用定时任务
pub async fn toggle_scheduled_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let scheduler = &state.task_scheduler.scheduler;
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let enabled = body
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    if scheduler.set_scheduled_job_enabled(uuid, enabled) {
        Ok(Json(json!({"id": id, "enabled": enabled})))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// DELETE /api/v1/tasks/scheduled/:id - 删除定时任务
pub async fn delete_scheduled_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let scheduler = &state.task_scheduler.scheduler;
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;

    if scheduler.remove_scheduled_job(uuid) {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
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
