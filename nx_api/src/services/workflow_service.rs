//! 工作流服务

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::workflow_repository::{RepositoryError, SharedWorkflowRepository};

/// 工作流服务错误
#[derive(Debug, thiserror::Error)]
pub enum WorkflowServiceError {
    #[error("工作流不存在: {0}")]
    NotFound(String),

    #[error("工作流已存在: {0}")]
    AlreadyExists(String),

    #[error("内部错误: {0}")]
    Internal(String),
}

impl From<RepositoryError> for WorkflowServiceError {
    fn from(err: RepositoryError) -> Self {
        match err {
            RepositoryError::NotFound(id) => WorkflowServiceError::NotFound(id),
            RepositoryError::AlreadyExists(id) => WorkflowServiceError::AlreadyExists(id),
            RepositoryError::Database(e) => WorkflowServiceError::Internal(e.to_string()),
            RepositoryError::JsonError(e) => WorkflowServiceError::Internal(e.to_string()),
        }
    }
}

/// 工作流服务
#[derive(Clone)]
pub struct WorkflowService {
    repository: SharedWorkflowRepository,
}

impl std::fmt::Debug for WorkflowService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkflowService").finish()
    }
}

impl WorkflowService {
    /// 创建新的工作流服务（使用内存仓储）
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建带指定仓储的工作流服务
    pub fn with_repository(repository: SharedWorkflowRepository) -> Self {
        Self { repository }
    }

    /// 创建带 SQLite 仓储的工作流服务
    pub fn with_sqlite(db_path: &std::path::Path) -> Result<Self, WorkflowServiceError> {
        let repo = super::SqliteWorkflowRepository::new(db_path)?;
        Ok(Self {
            repository: Arc::new(repo),
        })
    }

    /// 列出所有工作流
    pub fn list_workflows(&self) -> Result<Vec<Workflow>, WorkflowServiceError> {
        self.repository.find_all().map_err(Into::into)
    }

    /// 获取工作流
    pub fn get_workflow(&self, id: &str) -> Result<Option<Workflow>, WorkflowServiceError> {
        self.repository.find_by_id(id).map_err(Into::into)
    }

    /// 创建工作流
    pub fn create_workflow(
        &self,
        name: String,
        version: Option<String>,
        description: Option<String>,
        definition: serde_json::Value,
    ) -> Result<Workflow, WorkflowServiceError> {
        let now = Utc::now();
        let workflow = Workflow {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            version: version.unwrap_or_else(|| "1.0".to_string()),
            description,
            definition,
            created_at: now,
            updated_at: now,
        };

        self.repository.save(&workflow).map_err(Into::into)
    }

    /// 更新工作流
    pub fn update_workflow(
        &self,
        id: &str,
        name: Option<String>,
        version: Option<String>,
        description: Option<String>,
        definition: Option<serde_json::Value>,
    ) -> Result<Workflow, WorkflowServiceError> {
        // 先获取现有工作流
        let existing = self
            .repository
            .find_by_id(id)?
            .ok_or_else(|| WorkflowServiceError::NotFound(id.to_string()))?;

        let updated = Workflow {
            id: existing.id.clone(),
            name: name.unwrap_or(existing.name),
            version: version.unwrap_or(existing.version),
            description: description.or(existing.description),
            definition: definition.unwrap_or(existing.definition),
            created_at: existing.created_at,
            updated_at: Utc::now(),
        };

        self.repository.update(&updated).map_err(Into::into)
    }

    /// 删除工作流
    pub fn delete_workflow(&self, id: &str) -> Result<(), WorkflowServiceError> {
        self.repository.delete(id).map_err(Into::into)
    }
}

impl Default for WorkflowService {
    fn default() -> Self {
        Self {
            repository: Arc::new(InMemoryWorkflowRepository),
        }
    }
}

/// 内存仓储（用于测试和默认情况）
#[derive(Clone)]
struct InMemoryWorkflowRepository;

impl super::workflow_repository::WorkflowRepository for InMemoryWorkflowRepository {
    fn find_all(&self) -> Result<Vec<Workflow>, RepositoryError> {
        Ok(vec![])
    }
    fn find_by_id(&self, _id: &str) -> Result<Option<Workflow>, RepositoryError> {
        Ok(None)
    }
    fn save(&self, workflow: &Workflow) -> Result<Workflow, RepositoryError> {
        Ok(workflow.clone())
    }
    fn update(&self, workflow: &Workflow) -> Result<Workflow, RepositoryError> {
        Ok(workflow.clone())
    }
    fn delete(&self, _id: &str) -> Result<(), RepositoryError> {
        Ok(())
    }
}

/// 工作流定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub definition: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 工作流摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub stage_count: usize,
    pub agent_count: usize,
}

impl Workflow {
    /// 从 YAML 定义创建工作流
    pub fn from_yaml(id: String, yaml: &str) -> Result<Self, String> {
        let definition: serde_json::Value =
            serde_yaml::from_str(yaml).map_err(|e| format!("YAML 解析失败: {}", e))?;

        let name = definition
            .get("name")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("未命名")
            .to_string();

        let version = definition
            .get("version")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("1.0")
            .to_string();

        let description = definition
            .get("description")
            .and_then(|v: &serde_json::Value| v.as_str())
            .map(String::from);

        let now = Utc::now();

        Ok(Self {
            id,
            name,
            version,
            description,
            definition,
            created_at: now,
            updated_at: now,
        })
    }
}
