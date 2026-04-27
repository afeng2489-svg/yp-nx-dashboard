//! 断点续跑 API 路由

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::routes::AppState;
use crate::services::team_evolution::crash_detector::CrashRecoveryInfo;
use crate::services::team_evolution::error::TeamEvolutionError;
use crate::services::team_evolution::resume_service::ExecutionCheckpoint;

fn map_tev_error(err: TeamEvolutionError) -> (StatusCode, Json<serde_json::Value>) {
    let msg = err.to_string();
    let status = match &err {
        TeamEvolutionError::CheckpointNotFound(_) => StatusCode::NOT_FOUND,
        TeamEvolutionError::FeatureDisabled(_) => StatusCode::FORBIDDEN,
        TeamEvolutionError::ResumeFailed(_) => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(serde_json::json!({ "error": msg })))
}

/// GET /api/v1/executions/interrupted
pub async fn get_interrupted_executions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ExecutionCheckpoint>>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.resume_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Resume service not available" })),
        )
    })?;
    let interrupted = service.find_interrupted().map_err(map_tev_error)?;
    Ok(Json(interrupted))
}

/// POST /api/v1/executions/:id/resume
pub async fn resume_execution(
    State(state): State<Arc<AppState>>,
    Path(execution_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.resume_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Resume service not available" })),
        )
    })?;

    // 查找 checkpoint
    let interrupted = service.find_interrupted().map_err(map_tev_error)?;
    let checkpoint = interrupted
        .into_iter()
        .find(|c| c.execution_id == execution_id)
        .ok_or_else(|| TeamEvolutionError::CheckpointNotFound(execution_id.clone()))
        .map_err(map_tev_error)?;

    // 构建恢复提示词
    let resume_prompt = service.build_resume_prompt(&checkpoint);

    // 清理旧 checkpoint
    service
        .delete_checkpoint(&execution_id)
        .map_err(map_tev_error)?;

    Ok(Json(serde_json::json!({
        "execution_id": execution_id,
        "resume_prompt": resume_prompt,
        "project_id": checkpoint.project_id,
        "role_id": checkpoint.role_id,
        "pipeline_step_id": checkpoint.pipeline_step_id,
    })))
}

/// DELETE /api/v1/executions/:id/checkpoint
pub async fn abandon_checkpoint(
    State(state): State<Arc<AppState>>,
    Path(execution_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.resume_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Resume service not available" })),
        )
    })?;
    service
        .delete_checkpoint(&execution_id)
        .map_err(map_tev_error)?;
    Ok(Json(serde_json::json!({ "abandoned": execution_id })))
}

/// POST /api/v1/crash-detect
pub async fn trigger_crash_detection(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<CrashRecoveryInfo>>, (StatusCode, Json<serde_json::Value>)> {
    let detector = state.crash_detector.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Crash detector not available" })),
        )
    })?;
    let recoveries = detector.detect().map_err(map_tev_error)?;
    Ok(Json(recoveries))
}

/// POST /api/v1/temp-cleanup
pub async fn trigger_temp_cleanup(
    State(state): State<Arc<AppState>>,
) -> Result<
    Json<crate::services::team_evolution::temp_cleaner::TempCleanResult>,
    (StatusCode, Json<serde_json::Value>),
> {
    let cleaner = state.temp_cleaner.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Temp cleaner not available" })),
        )
    })?;
    let result = cleaner.run_all().map_err(map_tev_error)?;
    Ok(Json(result))
}
