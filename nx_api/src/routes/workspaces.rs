//! 工作区路由

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::services::Workspace;
use super::AppState;

/// 列出所有工作区
pub async fn list_workspaces(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<WorkspaceSummary>>, (StatusCode, String)> {
    match state.workspace_service.list_workspaces() {
        Ok(workspaces) => {
            let summaries = workspaces
                .into_iter()
                .map(|w| WorkspaceSummary {
                    id: w.id,
                    name: w.name,
                    description: w.description,
                    root_path: w.root_path,
                    owner_id: w.owner_id,
                    created_at: w.created_at.to_rfc3339(),
                    updated_at: w.updated_at.to_rfc3339(),
                })
                .collect();
            Ok(Json(summaries))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 获取工作区
pub async fn get_workspace(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Workspace>, (StatusCode, String)> {
    match state.workspace_service.get_workspace(&id) {
        Ok(Some(workspace)) => Ok(Json(workspace)),
        Ok(None) => Err((StatusCode::NOT_FOUND, format!("Workspace not found: {}", id))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 创建工作区
pub async fn create_workspace(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateWorkspaceRequest>,
) -> Result<(StatusCode, Json<Workspace>), (StatusCode, String)> {
    match state
        .workspace_service
        .create_workspace(payload.name, payload.owner_id, payload.description, payload.root_path)
    {
        Ok(workspace) => Ok((StatusCode::CREATED, Json(workspace))),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

/// 更新工作区
pub async fn update_workspace(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateWorkspaceRequest>,
) -> Result<Json<Workspace>, (StatusCode, String)> {
    match state
        .workspace_service
        .update_workspace(&id, payload.name, payload.description, payload.root_path, payload.settings)
    {
        Ok(workspace) => Ok(Json(workspace)),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

/// 删除工作区
pub async fn delete_workspace(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    match state.workspace_service.delete_workspace(&id) {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((StatusCode::NOT_FOUND, format!("Workspace not found: {}", id))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 浏览工作区文件
pub async fn browse_workspace(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<String>,
    Query(params): Query<BrowseQuery>,
) -> Result<Json<Vec<FileNodeResponse>>, (StatusCode, String)> {
    match state
        .workspace_service
        .browse_workspace_files(&workspace_id, params.path.as_deref())
    {
        Ok(nodes) => {
            let response = nodes
                .into_iter()
                .map(|n| FileNodeResponse {
                    id: n.id,
                    name: n.name,
                    path: n.path,
                    is_directory: n.is_directory,
                    size: n.size,
                    modified_at: n.modified_at,
                })
                .collect();
            Ok(Json(response))
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

/// 获取工作区 Git 变更列表
pub async fn get_git_diffs(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<String>,
) -> Result<Json<Vec<GitDiffResponse>>, (StatusCode, String)> {
    match state.workspace_service.get_git_diffs(&workspace_id) {
        Ok(diffs) => {
            let response = diffs
                .into_iter()
                .map(|d| GitDiffResponse {
                    path: d.path,
                    filename: d.filename,
                    diff_type: d.diff_type,
                    additions: d.additions,
                    deletions: d.deletions,
                })
                .collect();
            Ok(Json(response))
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

/// 获取单个文件的 Git diff
pub async fn get_file_diff(
    State(state): State<Arc<AppState>>,
    Path((workspace_id, file_path)): Path<(String, String)>,
) -> Result<Json<FileDiffResponse>, (StatusCode, String)> {
    match state.workspace_service.get_file_diff(&workspace_id, &file_path) {
        Ok(diff) => Ok(Json(FileDiffResponse { content: diff })),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

/// 获取 Git 状态
pub async fn get_git_status(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<String>,
) -> Result<Json<GitStatusResponse>, (StatusCode, String)> {
    match state.workspace_service.get_git_status(&workspace_id) {
        Ok(status) => Ok(Json(GitStatusResponse {
            branch: status.branch,
            ahead: status.ahead,
            behind: status.behind,
            is_dirty: status.is_dirty,
        })),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

// ============ 请求/响应类型 ============

#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    #[serde(default = "default_owner_id")]
    pub owner_id: String,
    pub description: Option<String>,
    pub root_path: Option<String>,
}

fn default_owner_id() -> String {
    "default".to_string()
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub root_path: Option<String>,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct BrowseQuery {
    pub path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub root_path: Option<String>,
    pub owner_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct FileNodeResponse {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: u64,
    pub modified_at: String,
}

#[derive(Debug, Serialize)]
pub struct GitDiffResponse {
    pub path: String,
    pub filename: String,
    pub diff_type: crate::services::workspace_service::GitDiffType,
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug, Serialize)]
pub struct FileDiffResponse {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct GitStatusResponse {
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub is_dirty: bool,
}
