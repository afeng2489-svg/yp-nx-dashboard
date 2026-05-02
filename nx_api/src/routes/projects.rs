//! Project Routes
//!
//! API routes for project management and team execution.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::project::{
    CreateProjectRequest, ExecuteProjectRequest, ExecuteProjectResponse, Project, ProjectWithTeam,
    UpdateProjectRequest,
};
use crate::models::project_module::{ProjectModule, UpsertModuleRequest};
use crate::routes::AppState;

/// List all projects
pub async fn list_projects(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Project>>, (StatusCode, String)> {
    state
        .project_service
        .list_projects()
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get project by ID
pub async fn get_project(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
) -> Result<Json<ProjectWithTeam>, (StatusCode, String)> {
    state
        .project_service
        .get_project_with_team(&project_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Project not found".to_string()))
}

/// Create new project
pub async fn create_project(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<Project>), (StatusCode, String)> {
    let project = state
        .project_service
        .create_project(req)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok((StatusCode::CREATED, Json(project)))
}

/// Update project
pub async fn update_project(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    Json(req): Json<UpdateProjectRequest>,
) -> Result<Json<Project>, (StatusCode, String)> {
    state
        .project_service
        .update_project(&project_id, req)
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

/// Delete project
pub async fn delete_project(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .project_service
        .delete_project(&project_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

/// List projects by team
pub async fn list_projects_by_team(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
) -> Result<Json<Vec<Project>>, (StatusCode, String)> {
    state
        .project_service
        .list_projects_by_team(&team_id)
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Execute project via team
pub async fn execute_project(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecuteProjectRequest>,
) -> Result<Json<ExecuteProjectResponse>, (StatusCode, String)> {
    state
        .project_service
        .execute_project(req)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// List project modules
pub async fn list_project_modules(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<ProjectModule>>, (StatusCode, String)> {
    state
        .project_module_service
        .get_modules(&project_id)
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create or update a project module (upsert by module_name)
pub async fn upsert_project_module(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    Json(req): Json<UpsertModuleRequest>,
) -> Result<Json<ProjectModule>, (StatusCode, String)> {
    state
        .project_module_service
        .upsert_module(&project_id, req)
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Delete a project module
pub async fn delete_project_module(
    State(state): State<Arc<AppState>>,
    Path((_project_id, module_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let deleted = state
        .project_module_service
        .delete_module(&module_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "Module not found".to_string()))
    }
}
