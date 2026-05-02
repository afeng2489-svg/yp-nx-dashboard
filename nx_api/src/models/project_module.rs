//! Project module models
//!
//! Tracks module-level status within a project so AI knows what's done.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Module status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl Default for ModuleStatus {
    fn default() -> Self {
        ModuleStatus::Pending
    }
}

impl ModuleStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModuleStatus::Pending => "pending",
            ModuleStatus::InProgress => "in_progress",
            ModuleStatus::Completed => "completed",
            ModuleStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => ModuleStatus::Pending,
            "in_progress" => ModuleStatus::InProgress,
            "completed" => ModuleStatus::Completed,
            "failed" => ModuleStatus::Failed,
            _ => ModuleStatus::Pending,
        }
    }
}

/// Project module entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectModule {
    pub id: String,
    pub project_id: String,
    pub module_name: String,
    pub status: ModuleStatus,
    pub summary: String,
    pub files_changed: Vec<String>,
    pub last_execution_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProjectModule {
    pub fn new(project_id: String, module_name: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            project_id,
            module_name,
            status: ModuleStatus::Pending,
            summary: String::new(),
            files_changed: Vec::new(),
            last_execution_id: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Create/update module request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertModuleRequest {
    pub module_name: String,
    #[serde(default)]
    pub status: Option<ModuleStatus>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub files_changed: Option<Vec<String>>,
    #[serde(default)]
    pub last_execution_id: Option<String>,
}
