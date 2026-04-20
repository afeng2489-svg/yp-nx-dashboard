//! Issue CRUD 路由

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::issue::{CreateIssueRequest, Issue, IssueFilter, UpdateIssueRequest};
use crate::services::issue_repository::IssueRepositoryError;
use super::AppState;

// ── 错误类型 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub error: String,
}

pub enum IssueError {
    NotFound(String),
    Internal(String),
}

impl axum::response::IntoResponse for IssueError {
    fn into_response(self) -> axum::response::Response {
        match self {
            IssueError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                Json(ErrorBody { error: msg }),
            ).into_response(),
            IssueError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody { error: msg }),
            ).into_response(),
        }
    }
}

impl From<IssueRepositoryError> for IssueError {
    fn from(e: IssueRepositoryError) -> Self {
        match e {
            IssueRepositoryError::NotFound(id) => IssueError::NotFound(format!("Issue {} 不存在", id)),
            IssueRepositoryError::Sqlite(e) => IssueError::Internal(e.to_string()),
        }
    }
}

// ── 响应结构 ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct IssueResponse {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub perspectives: Vec<String>,
    pub solution: Option<String>,
    pub depends_on: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Issue> for IssueResponse {
    fn from(issue: Issue) -> Self {
        Self {
            id: issue.id,
            title: issue.title,
            description: issue.description,
            status: issue.status.as_str().to_string(),
            priority: issue.priority.as_str().to_string(),
            perspectives: issue.perspectives,
            solution: issue.solution,
            depends_on: issue.depends_on,
            created_at: issue.created_at.to_rfc3339(),
            updated_at: issue.updated_at.to_rfc3339(),
        }
    }
}

// ── 处理函数 ──────────────────────────────────────────────────────────────────

/// GET /api/v1/issues
pub async fn list_issues(
    State(state): State<Arc<AppState>>,
    Query(filter): Query<IssueFilter>,
) -> Result<Json<Vec<IssueResponse>>, IssueError> {
    let issues = state
        .issue_repository
        .find_all(&filter)
        .map_err(IssueError::from)?;
    Ok(Json(issues.into_iter().map(IssueResponse::from).collect()))
}

/// GET /api/v1/issues/:id
pub async fn get_issue(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<IssueResponse>, IssueError> {
    let issue = state
        .issue_repository
        .find_by_id(&id)
        .map_err(IssueError::from)?;
    Ok(Json(IssueResponse::from(issue)))
}

/// POST /api/v1/issues
pub async fn create_issue(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateIssueRequest>,
) -> Result<(StatusCode, Json<IssueResponse>), IssueError> {
    let issue = state
        .issue_repository
        .create(req)
        .map_err(IssueError::from)?;
    Ok((StatusCode::CREATED, Json(IssueResponse::from(issue))))
}

/// PUT /api/v1/issues/:id
pub async fn update_issue(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateIssueRequest>,
) -> Result<Json<IssueResponse>, IssueError> {
    let issue = state
        .issue_repository
        .update(&id, req)
        .map_err(IssueError::from)?;
    Ok(Json(IssueResponse::from(issue)))
}

/// DELETE /api/v1/issues/:id
pub async fn delete_issue(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, IssueError> {
    state
        .issue_repository
        .delete(&id)
        .map_err(IssueError::from)?;
    Ok(StatusCode::NO_CONTENT)
}
