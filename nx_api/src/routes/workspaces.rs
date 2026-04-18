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

/// 检测工作区项目脚本
pub async fn detect_scripts(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<String>,
) -> Result<Json<ProjectScriptsResponse>, (StatusCode, String)> {
    let workspace = state
        .workspace_service
        .get_workspace(&workspace_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Workspace not found".to_string()))?;

    let root_path = workspace
        .root_path
        .as_deref()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Workspace has no root_path".to_string()))?;

    let root = std::path::Path::new(root_path);
    if !root.exists() {
        return Err((StatusCode::BAD_REQUEST, format!("Path does not exist: {}", root_path)));
    }

    let mut scripts = Vec::new();
    let mut project_type = "unknown".to_string();

    // Node.js: package.json
    let pkg_json = root.join("package.json");
    if pkg_json.exists() {
        project_type = "node".to_string();
        if let Ok(content) = std::fs::read_to_string(&pkg_json) {
            if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(pkg_scripts) = pkg.get("scripts").and_then(|s| s.as_object()) {
                    for (name, _cmd) in pkg_scripts {
                        let command = if name == "test" {
                            format!("npm test")
                        } else {
                            format!("npm run {}", name)
                        };
                        scripts.push(ScriptEntry {
                            name: name.clone(),
                            command,
                        });
                    }
                }
            }
        }
    }

    // Rust: Cargo.toml
    let cargo_toml = root.join("Cargo.toml");
    if cargo_toml.exists() {
        if project_type == "unknown" {
            project_type = "rust".to_string();
        }
        scripts.push(ScriptEntry { name: "run".to_string(), command: "cargo run".to_string() });
        scripts.push(ScriptEntry { name: "build".to_string(), command: "cargo build".to_string() });
        scripts.push(ScriptEntry { name: "test".to_string(), command: "cargo test".to_string() });
        scripts.push(ScriptEntry { name: "check".to_string(), command: "cargo check".to_string() });
    }

    // Python: pyproject.toml or requirements.txt
    let pyproject = root.join("pyproject.toml");
    let requirements = root.join("requirements.txt");
    let manage_py = root.join("manage.py");
    if pyproject.exists() || requirements.exists() {
        if project_type == "unknown" {
            project_type = "python".to_string();
        }
        if manage_py.exists() {
            scripts.push(ScriptEntry { name: "runserver".to_string(), command: "python manage.py runserver".to_string() });
        }
        scripts.push(ScriptEntry { name: "python".to_string(), command: "python main.py".to_string() });
        if requirements.exists() {
            scripts.push(ScriptEntry { name: "install".to_string(), command: "pip install -r requirements.txt".to_string() });
        }
    }

    // Makefile
    let makefile = root.join("Makefile");
    if makefile.exists() {
        if project_type == "unknown" {
            project_type = "make".to_string();
        }
        if let Ok(content) = std::fs::read_to_string(&makefile) {
            for line in content.lines() {
                if let Some(target) = line.strip_suffix(':').or_else(|| {
                    line.split(':').next().filter(|t| !t.contains('\t') && !t.starts_with('#') && !t.starts_with('.') && !t.contains(' '))
                }) {
                    let target = target.trim();
                    if !target.is_empty() && !target.starts_with('#') && !target.starts_with('.') {
                        scripts.push(ScriptEntry { name: target.to_string(), command: format!("make {}", target) });
                    }
                }
                if scripts.len() > 20 { break; }
            }
        }
    }

    Ok(Json(ProjectScriptsResponse { project_type, scripts }))
}

/// 读取文件内容
pub async fn read_file(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<String>,
    Query(params): Query<FileQuery>,
) -> Result<Json<FileContentResponse>, (StatusCode, String)> {
    let file_path = params.path.ok_or_else(|| {
        (StatusCode::BAD_REQUEST, "Missing 'path' query parameter".to_string())
    })?;

    match state.workspace_service.read_file_content(&workspace_id, &file_path) {
        Ok(content) => Ok(Json(FileContentResponse {
            path: content.path,
            content: content.content,
            language: content.language,
            size: content.size,
            modified_at: content.modified_at,
        })),
        Err(e) => {
            let status = match &e {
                crate::services::workspace_service::WorkspaceServiceError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::BAD_REQUEST,
            };
            Err((status, e.to_string()))
        }
    }
}

/// 写入文件内容
pub async fn write_file(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<String>,
    Query(params): Query<FileQuery>,
    Json(payload): Json<WriteFileRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let file_path = params.path.ok_or_else(|| {
        (StatusCode::BAD_REQUEST, "Missing 'path' query parameter".to_string())
    })?;

    match state.workspace_service.write_file_content(&workspace_id, &file_path, &payload.content) {
        Ok(()) => Ok(StatusCode::OK),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

/// 删除文件
pub async fn delete_file(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<String>,
    Query(params): Query<FileQuery>,
) -> Result<StatusCode, (StatusCode, String)> {
    let file_path = params.path.ok_or_else(|| {
        (StatusCode::BAD_REQUEST, "Missing 'path' query parameter".to_string())
    })?;

    match state.workspace_service.delete_file(&workspace_id, &file_path) {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
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

#[derive(Debug, Serialize)]
pub struct ProjectScriptsResponse {
    pub project_type: String,
    pub scripts: Vec<ScriptEntry>,
}

#[derive(Debug, Serialize)]
pub struct ScriptEntry {
    pub name: String,
    pub command: String,
}

#[derive(Debug, Deserialize)]
pub struct FileQuery {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WriteFileRequest {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct FileContentResponse {
    pub path: String,
    pub content: String,
    pub language: String,
    pub size: u64,
    pub modified_at: String,
}
