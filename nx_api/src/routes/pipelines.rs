//! Pipeline API 路由

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::pipeline::{Pipeline, PipelineStep, StepStatus};
use crate::routes::{resolve_project_id, AppState};
use crate::services::team_evolution::error::TeamEvolutionError;

#[derive(Serialize)]
pub struct PipelineResponse {
    id: String,
    project_id: String,
    team_id: String,
    current_phase: String,
    status: String,
    steps: Vec<StepResponse>,
    progress: ProgressSummary,
}

#[derive(Serialize)]
pub struct StepResponse {
    id: String,
    task_id: String,
    phase: String,
    role_id: String,
    instruction: String,
    status: String,
    output: Option<String>,
    retry_count: u32,
}

#[derive(Serialize)]
pub struct ProgressSummary {
    total_steps: usize,
    completed_steps: usize,
    running_steps: usize,
    failed_steps: usize,
    progress_pct: u32,
}

#[derive(Deserialize)]
pub struct CreatePipelineRequest {
    pub team_id: String,
}

fn build_response(pipeline: Pipeline, steps: Vec<PipelineStep>) -> PipelineResponse {
    let total = steps.len();
    let completed = steps
        .iter()
        .filter(|s| s.status == StepStatus::Completed)
        .count();
    let running = steps
        .iter()
        .filter(|s| s.status == StepStatus::Running)
        .count();
    let failed = steps
        .iter()
        .filter(|s| s.status == StepStatus::Failed)
        .count();
    let pct = (completed * 100).checked_div(total).unwrap_or(0) as u32;

    PipelineResponse {
        id: pipeline.id,
        project_id: pipeline.project_id,
        team_id: pipeline.team_id,
        current_phase: pipeline.current_phase.as_str().to_string(),
        status: pipeline.status.as_str().to_string(),
        steps: steps
            .into_iter()
            .map(|s| StepResponse {
                id: s.id,
                task_id: s.task_id,
                phase: s.phase.as_str().to_string(),
                role_id: s.role_id,
                instruction: s.instruction,
                status: s.status.as_str().to_string(),
                output: s.output,
                retry_count: s.retry_count,
            })
            .collect(),
        progress: ProgressSummary {
            total_steps: total,
            completed_steps: completed,
            running_steps: running,
            failed_steps: failed,
            progress_pct: pct,
        },
    }
}

fn map_tev_error(err: TeamEvolutionError) -> (StatusCode, Json<serde_json::Value>) {
    let msg = err.to_string();
    let status = match &err {
        TeamEvolutionError::PipelineNotFound(_) => StatusCode::NOT_FOUND,
        TeamEvolutionError::StepNotFound { .. } => StatusCode::NOT_FOUND,
        TeamEvolutionError::PipelineAlreadyRunning(_) => StatusCode::CONFLICT,
        TeamEvolutionError::FeatureDisabled(_) => StatusCode::FORBIDDEN,
        TeamEvolutionError::StepNotRetriable { .. } => StatusCode::BAD_REQUEST,
        TeamEvolutionError::PipelinePaused(_) => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(serde_json::json!({ "error": msg })))
}

/// POST /api/v1/projects/:id/pipeline
pub async fn create_pipeline(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    Json(body): Json<CreatePipelineRequest>,
) -> Result<(StatusCode, Json<PipelineResponse>), (StatusCode, Json<serde_json::Value>)> {
    let service = state.pipeline_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Pipeline service not available" })),
        )
    })?;

    let resolved_id = resolve_project_id(&state, &project_id);
    let pipeline = service
        .create_pipeline(&resolved_id, &body.team_id)
        .map_err(map_tev_error)?;
    let resp = build_response(pipeline, vec![]);
    Ok((StatusCode::CREATED, Json(resp)))
}

/// GET /api/v1/projects/:id/pipeline
pub async fn get_project_pipeline(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
) -> Result<Json<Option<PipelineResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.pipeline_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Pipeline service not available" })),
        )
    })?;

    let resolved_id = resolve_project_id(&state, &project_id);
    match service
        .get_by_project(&resolved_id)
        .map_err(map_tev_error)?
    {
        Some((pipeline, steps)) => Ok(Json(Some(build_response(pipeline, steps)))),
        None => Ok(Json(None)),
    }
}

/// POST /api/v1/pipelines/:id/start
pub async fn start_pipeline(
    State(state): State<Arc<AppState>>,
    Path(pipeline_id): Path<String>,
) -> Result<Json<PipelineResponse>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.pipeline_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Pipeline service not available" })),
        )
    })?;

    let pipeline = service.start(&pipeline_id).map_err(map_tev_error)?;
    let (pipeline, steps) = service.get_status(&pipeline_id).map_err(map_tev_error)?;
    Ok(Json(build_response(pipeline, steps)))
}

/// POST /api/v1/pipelines/:id/pause
pub async fn pause_pipeline(
    State(state): State<Arc<AppState>>,
    Path(pipeline_id): Path<String>,
) -> Result<Json<PipelineResponse>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.pipeline_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Pipeline service not available" })),
        )
    })?;

    let _pipeline = service.pause(&pipeline_id).map_err(map_tev_error)?;
    let (pipeline, steps) = service.get_status(&pipeline_id).map_err(map_tev_error)?;
    Ok(Json(build_response(pipeline, steps)))
}

/// POST /api/v1/pipelines/:id/resume
pub async fn resume_pipeline(
    State(state): State<Arc<AppState>>,
    Path(pipeline_id): Path<String>,
) -> Result<Json<PipelineResponse>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.pipeline_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Pipeline service not available" })),
        )
    })?;

    let _pipeline = service.resume(&pipeline_id).map_err(map_tev_error)?;
    let (pipeline, steps) = service.get_status(&pipeline_id).map_err(map_tev_error)?;
    Ok(Json(build_response(pipeline, steps)))
}

/// GET /api/v1/pipelines/:id/steps
pub async fn get_pipeline_steps(
    State(state): State<Arc<AppState>>,
    Path(pipeline_id): Path<String>,
) -> Result<Json<PipelineResponse>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.pipeline_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Pipeline service not available" })),
        )
    })?;

    let (pipeline, steps) = service.get_status(&pipeline_id).map_err(map_tev_error)?;
    Ok(Json(build_response(pipeline, steps)))
}

/// POST /api/v1/pipelines/:pipeline_id/steps/:step_id/retry
pub async fn retry_step(
    State(state): State<Arc<AppState>>,
    Path((pipeline_id, step_id)): Path<(String, String)>,
) -> Result<Json<StepResponse>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.pipeline_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Pipeline service not available" })),
        )
    })?;

    let step = service
        .retry_step(&pipeline_id, &step_id)
        .map_err(map_tev_error)?;
    Ok(Json(StepResponse {
        id: step.id,
        task_id: step.task_id,
        phase: step.phase.as_str().to_string(),
        role_id: step.role_id,
        instruction: step.instruction,
        status: step.status.as_str().to_string(),
        output: step.output,
        retry_count: step.retry_count,
    }))
}

/// POST /api/v1/pipelines/:id/dispatch
/// 获取可调度步骤并通过已有 execute_team_task 路径执行
pub async fn dispatch_pipeline_steps(
    State(state): State<Arc<AppState>>,
    Path(pipeline_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.pipeline_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Pipeline service not available" })),
        )
    })?;

    // Pre-fetch pipeline info for step dispatch
    let pipeline = service.find_pipeline(&pipeline_id).map_err(map_tev_error)?;
    let pipeline = pipeline.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Pipeline not found" })),
        )
    })?;

    let steps = service
        .get_dispatchable_steps(&pipeline_id)
        .map_err(map_tev_error)?;

    if steps.is_empty() {
        return Ok(Json(
            serde_json::json!({ "dispatched": 0, "message": "no steps ready" }),
        ));
    }

    let mut dispatched = Vec::new();
    let handler = state.team_evolution_handler.as_ref();

    for step in &steps {
        // Mark step as Running
        service
            .update_step_status(
                &step.id,
                &crate::models::pipeline::StepStatus::Running,
                None,
            )
            .map_err(map_tev_error)?;

        // Register with event handler for auto-tracking
        if let Some(h) = handler {
            let execution_id = uuid::Uuid::new_v4().to_string();
            let working_dir = state.current_workspace_path.read().clone();
            h.register_step_execution(
                &execution_id,
                &pipeline_id,
                &step.id,
                &pipeline.project_id,
                &pipeline.team_id,
                &step.role_id,
                working_dir.as_deref(),
            );

            // Register with process lifecycle
            if let Some(lc) = state.process_lifecycle.as_ref() {
                let _ =
                    lc.register_process(&execution_id, &pipeline.project_id, &step.role_id, None);
            }

            // Register cancel token
            let cancel_token = tokio_util::sync::CancellationToken::new();
            state
                .agent_execution_manager
                .register_cancel_token(&execution_id, cancel_token.clone());

            // ── PTY-first dispatch (reusing existing infra) ──
            let event_tx = state.agent_execution_manager.event_sender();
            let working_dir = state.current_workspace_path.read().clone();

            let pty_result = super::teams::try_pty_dispatch_pub(
                &state,
                &pipeline.team_id,
                &step.role_id,
                &step.instruction,
                &execution_id,
                working_dir.as_deref(),
                event_tx.clone(),
                cancel_token,
                Some(&step.id),
            );

            if let Ok(session_id) = pty_result {
                let _ = event_tx.send(crate::ws::agent_execution::AgentExecutionEvent::Started {
                    execution_id: execution_id.clone(),
                    agent_role: "pipeline".to_string(),
                    task_summary: step.instruction.clone(),
                    role_id: Some(step.role_id.clone()),
                    session_id: Some(session_id),
                });
                dispatched.push(serde_json::json!({
                    "step_id": step.id,
                    "execution_id": execution_id,
                    "method": "pty",
                }));
            } else if let Err(pty_err) = pty_result {
                // PTY failed — roll back step status to Pending so it can be retried
                tracing::warn!(
                    "[Pipeline] PTY dispatch failed for step {}: {pty_err}",
                    step.id
                );
                let _ = service.update_step_status(
                    &step.id,
                    &crate::models::pipeline::StepStatus::Pending,
                    None,
                );
                // Clean up registered process and cancel token
                if let Some(lc) = state.process_lifecycle.as_ref() {
                    lc.unregister_process(&execution_id);
                }
                state
                    .agent_execution_manager
                    .remove_execution(&execution_id);
                dispatched.push(serde_json::json!({
                    "step_id": step.id,
                    "execution_id": execution_id,
                    "method": "failed",
                    "error": pty_err,
                }));
            }
        }
    }

    Ok(Json(serde_json::json!({
        "dispatched": dispatched.len(),
        "steps": dispatched,
    })))
}

#[derive(Deserialize)]
pub struct RejectRequest {
    pub reason: Option<String>,
}

/// POST /api/v1/pipelines/:id/approve
pub async fn approve_pipeline(
    State(state): State<Arc<AppState>>,
    Path(pipeline_id): Path<String>,
) -> Result<Json<Pipeline>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.pipeline_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Pipeline service not available" })),
        )
    })?;
    service
        .approve(&pipeline_id)
        .map(Json)
        .map_err(map_tev_error)
}

/// POST /api/v1/pipelines/:id/reject
pub async fn reject_pipeline(
    State(state): State<Arc<AppState>>,
    Path(pipeline_id): Path<String>,
    Json(body): Json<RejectRequest>,
) -> Result<Json<Pipeline>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.pipeline_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Pipeline service not available" })),
        )
    })?;
    let reason = body.reason.as_deref().unwrap_or("rejected by user");
    service
        .reject(&pipeline_id, reason)
        .map(Json)
        .map_err(map_tev_error)
}
