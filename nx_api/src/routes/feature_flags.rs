//! Feature Flag API 路由

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::feature_flag::{FeatureFlag, FeatureFlagState};
use crate::routes::AppState;
use crate::services::team_evolution::error::TeamEvolutionError;

#[derive(Serialize)]
pub struct FeatureFlagResponse {
    key: String,
    state: String,
    circuit_breaker: bool,
    error_count: u32,
    error_threshold: u32,
}

impl From<FeatureFlag> for FeatureFlagResponse {
    fn from(f: FeatureFlag) -> Self {
        Self {
            key: f.key,
            state: f.state.as_str().to_string(),
            circuit_breaker: f.circuit_breaker,
            error_count: f.error_count,
            error_threshold: f.error_threshold,
        }
    }
}

#[derive(Deserialize)]
pub struct UpdateFlagRequest {
    pub state: String,
}

fn map_tev_error(err: TeamEvolutionError) -> (StatusCode, Json<serde_json::Value>) {
    let msg = err.to_string();
    let status = match &err {
        TeamEvolutionError::FlagNotFound(_) => StatusCode::NOT_FOUND,
        TeamEvolutionError::FeatureDisabled(_) => StatusCode::FORBIDDEN,
        TeamEvolutionError::FeatureReadOnly(_) => StatusCode::FORBIDDEN,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(serde_json::json!({ "error": msg })))
}

/// GET /api/v1/feature-flags
pub async fn list_feature_flags(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<FeatureFlagResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.feature_flag_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Feature flag service not available" })),
        )
    })?;

    let flags = service.list_all().map_err(map_tev_error)?;
    Ok(Json(
        flags.into_iter().map(FeatureFlagResponse::from).collect(),
    ))
}

/// GET /api/v1/feature-flags/:key
pub async fn get_feature_flag(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> Result<Json<FeatureFlagResponse>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.feature_flag_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Feature flag service not available" })),
        )
    })?;

    let flag = service.get(&key).map_err(map_tev_error)?;
    Ok(Json(FeatureFlagResponse::from(flag)))
}

/// PUT /api/v1/feature-flags/:key
pub async fn update_feature_flag(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
    Json(body): Json<UpdateFlagRequest>,
) -> Result<Json<FeatureFlagResponse>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.feature_flag_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Feature flag service not available" })),
        )
    })?;

    let new_state = FeatureFlagState::from_str(&body.state)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": format!("Invalid state: {}. Expected: on, readonly, off", body.state) }))))?;

    let flag = service.set_state(&key, new_state).map_err(map_tev_error)?;
    Ok(Json(FeatureFlagResponse::from(flag)))
}

/// POST /api/v1/feature-flags/:key/reset
pub async fn reset_feature_flag(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> Result<Json<FeatureFlagResponse>, (StatusCode, Json<serde_json::Value>)> {
    let service = state.feature_flag_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Feature flag service not available" })),
        )
    })?;

    let flag = service.reset(&key).map_err(map_tev_error)?;
    Ok(Json(FeatureFlagResponse::from(flag)))
}
