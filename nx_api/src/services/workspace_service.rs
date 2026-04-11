//! 工作区服务
//!
//! 工作区业务逻辑层。

use std::path::Path;
use std::sync::Arc;
use chrono::Utc;
use thiserror::Error;

use super::workspace_repository::{WorkspaceRepository, Workspace, RepositoryError};

/// 服务错误
#[derive(Error, Debug)]
pub enum WorkspaceServiceError {
    #[error("工作区不存在: {0}")]
    NotFound(String),

    #[error("内部错误: {0}")]
    Internal(String),

    #[error("创建失败: {0}")]
    CreationFailed(String),

    #[error("更新失败: {0}")]
    UpdateFailed(String),

    #[error("文件操作失败: {0}")]
    FileError(String),
}

impl From<RepositoryError> for WorkspaceServiceError {
    fn from(err: RepositoryError) -> Self {
        match err {
            RepositoryError::NotFound(id) => WorkspaceServiceError::NotFound(id),
            _ => WorkspaceServiceError::Internal(err.to_string()),
        }
    }
}

/// 文件节点（文件或目录）
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileNode {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: u64,
    pub modified_at: String,
}

/// 工作区服务
#[derive(Clone)]
pub struct WorkspaceService {
    repository: Arc<dyn WorkspaceRepository>,
}

impl WorkspaceService {
    /// 创建工作区服务
    pub fn new(repository: Arc<dyn WorkspaceRepository>) -> Self {
        Self { repository }
    }

    /// 列出所有工作区
    pub fn list_workspaces(&self) -> Result<Vec<Workspace>, WorkspaceServiceError> {
        self.repository.find_all().map_err(Into::into)
    }

    /// 获取工作区
    pub fn get_workspace(&self, id: &str) -> Result<Option<Workspace>, WorkspaceServiceError> {
        self.repository.find_by_id(id).map_err(Into::into)
    }

    /// 根据所有者获取工作区
    pub fn list_workspaces_by_owner(
        &self,
        owner_id: &str,
    ) -> Result<Vec<Workspace>, WorkspaceServiceError> {
        self.repository.find_by_owner(owner_id).map_err(Into::into)
    }

    /// 创建工作区
    pub fn create_workspace(
        &self,
        name: String,
        owner_id: String,
        description: Option<String>,
        root_path: Option<String>,
    ) -> Result<Workspace, WorkspaceServiceError> {
        let workspace = Workspace::new(name, owner_id, description, root_path);
        self.repository
            .create(&workspace)
            .map_err(|e| WorkspaceServiceError::CreationFailed(e.to_string()))?;
        Ok(workspace)
    }

    /// 更新工作区
    pub fn update_workspace(
        &self,
        id: &str,
        name: Option<String>,
        description: Option<String>,
        root_path: Option<String>,
        settings: Option<serde_json::Value>,
    ) -> Result<Workspace, WorkspaceServiceError> {
        let mut workspace = self
            .repository
            .find_by_id(id)?
            .ok_or_else(|| WorkspaceServiceError::NotFound(id.to_string()))?;

        if let Some(n) = name {
            workspace.name = n;
        }
        if description.is_some() {
            workspace.description = description;
        }
        if let Some(path) = root_path {
            workspace.root_path = Some(path);
        }
        if let Some(s) = settings {
            workspace.settings = s;
        }
        workspace.updated_at = Utc::now();

        self.repository
            .update(&workspace)
            .map_err(|e| WorkspaceServiceError::UpdateFailed(e.to_string()))?;
        Ok(workspace)
    }

    /// 删除工作区
    pub fn delete_workspace(&self, id: &str) -> Result<bool, WorkspaceServiceError> {
        self.repository.delete(id).map_err(Into::into)
    }

    /// 浏览工作区文件
    pub fn browse_workspace_files(
        &self,
        workspace_id: &str,
        path: Option<&str>,
    ) -> Result<Vec<FileNode>, WorkspaceServiceError> {
        let workspace = self
            .repository
            .find_by_id(workspace_id)?
            .ok_or_else(|| WorkspaceServiceError::NotFound(workspace_id.to_string()))?;

        let root = workspace
            .root_path
            .ok_or_else(|| WorkspaceServiceError::FileError("工作区未设置根目录".to_string()))?;

        let full_path = match path {
            Some(p) => Path::new(&root).join(p),
            None => Path::new(&root).to_path_buf(),
        };

        if !full_path.exists() {
            return Err(WorkspaceServiceError::FileError(format!(
                "路径不存在: {}",
                full_path.display()
            )));
        }

        if !full_path.is_dir() {
            return Err(WorkspaceServiceError::FileError(format!(
                "路径不是目录: {}",
                full_path.display()
            )));
        }

        let mut nodes = Vec::new();

        match std::fs::read_dir(&full_path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path_buf = entry.path();
                    let metadata = entry.metadata().ok();

                    let is_directory = path_buf.is_dir();
                    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                    let modified_at = metadata
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            chrono::DateTime::<Utc>::from(t)
                                .format("%Y-%m-%d %H:%M:%S")
                                .to_string()
                        })
                        .unwrap_or_default();

                    let name = path_buf
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    let relative_path = path_buf
                        .strip_prefix(&root)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| name.clone());

                    nodes.push(FileNode {
                        id: uuid::Uuid::new_v4().to_string(),
                        name,
                        path: relative_path,
                        is_directory,
                        size,
                        modified_at,
                    });
                }
            }
            Err(e) => {
                return Err(WorkspaceServiceError::FileError(format!(
                    "读取目录失败: {}",
                    e
                )));
            }
        }

        // 按目录优先，然后按名称排序
        nodes.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        Ok(nodes)
    }

    /// Git 变更类型
    pub fn get_git_diffs(&self, workspace_id: &str) -> Result<Vec<GitDiff>, WorkspaceServiceError> {
        let workspace = self
            .repository
            .find_by_id(workspace_id)?
            .ok_or_else(|| WorkspaceServiceError::NotFound(workspace_id.to_string()))?;

        let root = workspace
            .root_path
            .ok_or_else(|| WorkspaceServiceError::FileError("工作区未设置根目录".to_string()))?;

        // Check if it's a git repo
        let git_dir = Path::new(&root).join(".git");
        if !git_dir.exists() {
            return Ok(vec![]);
        }

        // Run git status --porcelain to get changed files
        let output = std::process::Command::new("git")
            .args(["status", "--porcelain", "-uall"])
            .current_dir(&root)
            .output()
            .map_err(|e| WorkspaceServiceError::FileError(format!("Failed to run git: {}", e)))?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let status_output = String::from_utf8_lossy(&output.stdout);
        let mut diffs = Vec::new();

        // Parse git status output
        for line in status_output.lines() {
            if line.len() < 3 {
                continue;
            }

            let index_and_working_tree = &line[..2];
            let file_path = line[3..].trim();

            // Skip submodules and unmerged files
            if index_and_working_tree == "??" {
                // Untracked file - treat as added
                diffs.push(GitDiff {
                    path: file_path.to_string(),
                    filename: Path::new(file_path)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| file_path.to_string()),
                    diff_type: GitDiffType::Added,
                    additions: 0,
                    deletions: 0,
                });
            } else if index_and_working_tree.contains('D') || index_and_working_tree == "DD" || index_and_working_tree == "AU" || index_and_working_tree == "UD" {
                // Deleted file
                diffs.push(GitDiff {
                    path: file_path.to_string(),
                    filename: Path::new(file_path)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| file_path.to_string()),
                    diff_type: GitDiffType::Deleted,
                    additions: 0,
                    deletions: 0,
                });
            } else {
                // Modified or added in index
                let diff_type = if index_and_working_tree == "A " || index_and_working_tree == "AM" || index_and_working_tree == "M " {
                    GitDiffType::Added
                } else {
                    GitDiffType::Modified
                };

                // Get line counts with git diff --numstat
                let numstat_output = std::process::Command::new("git")
                    .args(["diff", "--numstat", "--", file_path])
                    .current_dir(&root)
                    .output();

                let mut additions = 0;
                let mut deletions = 0;
                if let Ok(numstat) = numstat_output {
                    if numstat.status.success() {
                        let numstat_str = String::from_utf8_lossy(&numstat.stdout);
                        if let Some(stats_line) = numstat_str.lines().next() {
                            let parts: Vec<&str> = stats_line.split('\t').collect();
                            if parts.len() >= 2 {
                                additions = parts[0].parse().unwrap_or(0);
                                deletions = parts[1].parse().unwrap_or(0);
                            }
                        }
                    }
                }

                diffs.push(GitDiff {
                    path: file_path.to_string(),
                    filename: Path::new(file_path)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| file_path.to_string()),
                    diff_type,
                    additions,
                    deletions,
                });
            }
        }

        Ok(diffs)
    }

    /// 获取单个文件的 diff 内容
    pub fn get_file_diff(&self, workspace_id: &str, file_path: &str) -> Result<String, WorkspaceServiceError> {
        let workspace = self
            .repository
            .find_by_id(workspace_id)?
            .ok_or_else(|| WorkspaceServiceError::NotFound(workspace_id.to_string()))?;

        let root = workspace
            .root_path
            .ok_or_else(|| WorkspaceServiceError::FileError("工作区未设置根目录".to_string()))?;

        // Get staged diff first, then working tree diff
        let output = std::process::Command::new("git")
            .args(["diff", "--", file_path])
            .current_dir(&root)
            .output()
            .map_err(|e| WorkspaceServiceError::FileError(format!("Failed to run git diff: {}", e)))?;

        let diff_content = String::from_utf8_lossy(&output.stdout).to_string();

        // If no diff in working tree, try staged
        if diff_content.is_empty() {
            let staged_output = std::process::Command::new("git")
                .args(["diff", "--cached", "--", file_path])
                .current_dir(&root)
                .output()
                .map_err(|e| WorkspaceServiceError::FileError(format!("Failed to run git diff --cached: {}", e)))?;
            return Ok(String::from_utf8_lossy(&staged_output.stdout).to_string());
        }

        Ok(diff_content)
    }

    /// 获取 git 仓库状态摘要
    pub fn get_git_status(&self, workspace_id: &str) -> Result<GitStatus, WorkspaceServiceError> {
        let workspace = self
            .repository
            .find_by_id(workspace_id)?
            .ok_or_else(|| WorkspaceServiceError::NotFound(workspace_id.to_string()))?;

        let root = workspace
            .root_path
            .ok_or_else(|| WorkspaceServiceError::FileError("工作区未设置根目录".to_string()))?;

        let git_dir = Path::new(&root).join(".git");
        if !git_dir.exists() {
            return Err(WorkspaceServiceError::FileError("Not a git repository".to_string()));
        }

        // Get branch name
        let branch_output = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&root)
            .output();

        let branch = branch_output
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Get ahead/behind info
        let tracking_output = std::process::Command::new("git")
            .args(["rev-list", "--left-right", "--count", "@{upstream}...HEAD"])
            .current_dir(&root)
            .output();

        let (ahead, behind) = tracking_output
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let parts: Vec<&str> = s.split_whitespace().collect();
                let ahead = parts.get(0).and_then(|p| p.parse().ok()).unwrap_or(0);
                let behind = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
                (ahead, behind)
            })
            .unwrap_or((0, 0));

        Ok(GitStatus {
            branch,
            ahead,
            behind,
            is_dirty: true, // Simplified - just check if there are changes
        })
    }
}

/// Git 变更类型
#[derive(Debug, Clone, serde::Serialize)]
pub struct GitDiff {
    pub path: String,
    pub filename: String,
    pub diff_type: GitDiffType,
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GitDiffType {
    Added,
    Modified,
    Deleted,
}

/// Git 状态摘要
#[derive(Debug, Clone, serde::Serialize)]
pub struct GitStatus {
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub is_dirty: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::workspace_repository::SqliteWorkspaceRepository;

    #[test]
    fn test_create_and_list() {
        let repo = Arc::new(SqliteWorkspaceRepository::in_memory().unwrap());
        let service = WorkspaceService::new(repo);

        let workspace = service
            .create_workspace(
                "Test".to_string(),
                "owner-1".to_string(),
                Some("Test workspace".to_string()),
                None,
            )
            .unwrap();

        let all = service.list_workspaces().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, workspace.id);
    }

    #[test]
    fn test_get_workspace() {
        let repo = Arc::new(SqliteWorkspaceRepository::in_memory().unwrap());
        let service = WorkspaceService::new(repo);

        let created = service
            .create_workspace("Test".to_string(), "owner-1".to_string(), None, None)
            .unwrap();

        let found = service.get_workspace(&created.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test");
    }

    #[test]
    fn test_update_workspace() {
        let repo = Arc::new(SqliteWorkspaceRepository::in_memory().unwrap());
        let service = WorkspaceService::new(repo);

        let created = service
            .create_workspace("Original".to_string(), "owner-1".to_string(), None, None)
            .unwrap();

        let updated = service
            .update_workspace(&created.id, Some("Updated".to_string()), None, None, None)
            .unwrap();

        assert_eq!(updated.name, "Updated");
    }

    #[test]
    fn test_delete_workspace() {
        let repo = Arc::new(SqliteWorkspaceRepository::in_memory().unwrap());
        let service = WorkspaceService::new(repo);

        let created = service
            .create_workspace("To Delete".to_string(), "owner-1".to_string(), None, None)
            .unwrap();

        let deleted = service.delete_workspace(&created.id).unwrap();
        assert!(deleted);

        let found = service.get_workspace(&created.id).unwrap();
        assert!(found.is_none());
    }
}
