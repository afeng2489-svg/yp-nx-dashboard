//! Project models
//!
//! Data models for project management with team execution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Project status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl Default for ProjectStatus {
    fn default() -> Self {
        ProjectStatus::Pending
    }
}

impl ProjectStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectStatus::Pending => "pending",
            ProjectStatus::InProgress => "in_progress",
            ProjectStatus::Completed => "completed",
            ProjectStatus::Failed => "failed",
            ProjectStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => ProjectStatus::Pending,
            "in_progress" => ProjectStatus::InProgress,
            "completed" => ProjectStatus::Completed,
            "failed" => ProjectStatus::Failed,
            "cancelled" => ProjectStatus::Cancelled,
            _ => ProjectStatus::Pending,
        }
    }
}

/// Project entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub team_id: String,
    pub workspace_id: Option<String>,  // Links to a workspace (folder)
    pub workflow_id: Option<String>,
    pub variables: HashMap<String, String>,
    pub status: ProjectStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Project {
    pub fn new(
        name: String,
        description: String,
        team_id: String,
        workspace_id: Option<String>,
        workflow_id: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            team_id,
            workspace_id,
            workflow_id,
            variables: HashMap::new(),
            status: ProjectStatus::Pending,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Project execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteProjectRequest {
    pub project_id: String,
    pub task: String,
    pub context: HashMap<String, String>,
}

/// Project execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteProjectResponse {
    pub success: bool,
    pub project_id: String,
    pub team_id: String,
    pub messages: Vec<ProjectMessage>,
    pub final_output: String,
    pub error: Option<String>,
}

/// Project message for execution history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMessage {
    pub id: String,
    pub project_id: String,
    pub role_id: Option<String>,
    pub role_name: Option<String>,
    pub content: String,
    pub message_type: String,
    pub created_at: DateTime<Utc>,
}

/// Create project request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: String,
    pub team_id: String,
    pub workspace_id: Option<String>,  // Optional workspace (folder) link
    pub workflow_id: Option<String>,
}

/// Update project request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub team_id: Option<String>,
    pub workspace_id: Option<String>,
    pub workflow_id: Option<String>,
    pub status: Option<ProjectStatus>,
}

/// Project with team info (for detailed view)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectWithTeam {
    pub project: Project,
    pub team_name: String,
    pub workflow_name: Option<String>,
    pub workspace_name: Option<String>,
    pub workspace_path: Option<String>,
}
