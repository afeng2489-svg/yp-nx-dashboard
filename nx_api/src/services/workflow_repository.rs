//! 工作流仓储层
//!
//! 使用 Repository 模式封装工作流的数据库访问。

use crate::services::workflow_service::Workflow;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// 工作流仓储错误
#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("工作流不存在: {0}")]
    NotFound(String),

    #[error("工作流已存在: {0}")]
    AlreadyExists(String),

    #[error("JSON 序列化错误: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// 工作流数据模型（用于数据库存储）
#[derive(Debug, Clone)]
struct WorkflowRow {
    id: String,
    name: String,
    version: String,
    description: Option<String>,
    definition: String,
    created_at: String,
    updated_at: String,
}

impl WorkflowRow {
    fn into_workflow(self) -> Workflow {
        Workflow {
            id: self.id,
            name: self.name,
            version: self.version,
            description: self.description,
            definition: serde_json::from_str(&self.definition).unwrap_or_default(),
            created_at: DateTime::parse_from_rfc3339(&self.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&self.updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }
    }
}

/// 工作流仓储 trait（用于测试和依赖注入）
pub trait WorkflowRepository: Send + Sync {
    fn find_all(&self) -> Result<Vec<Workflow>, RepositoryError>;
    fn find_by_id(&self, id: &str) -> Result<Option<Workflow>, RepositoryError>;
    fn save(&self, workflow: &Workflow) -> Result<Workflow, RepositoryError>;
    fn update(&self, workflow: &Workflow) -> Result<Workflow, RepositoryError>;
    fn delete(&self, id: &str) -> Result<(), RepositoryError>;
}

/// SQLite 工作流仓储实现
pub struct SqliteWorkflowRepository {
    conn: Mutex<Connection>,
}

impl SqliteWorkflowRepository {
    /// 创建新的 SQLite 工作流仓储
    pub fn new(db_path: &Path) -> Result<Self, RepositoryError> {
        let conn = Connection::open(db_path)?;

        // 初始化数据库 schema
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS workflows (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                description TEXT,
                definition TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            ",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 从现有连接创建仓储（用于测试）
    pub fn from_connection(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    fn row_to_workflow(&self, row: WorkflowRow) -> Workflow {
        row.into_workflow()
    }
}

impl WorkflowRepository for SqliteWorkflowRepository {
    fn find_all(&self) -> Result<Vec<Workflow>, RepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, version, description, definition, created_at, updated_at FROM workflows ORDER BY created_at DESC",
        )?;

        let workflows = stmt
            .query_map([], |row| {
                Ok(WorkflowRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    description: row.get(3)?,
                    definition: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .map(|row| self.row_to_workflow(row))
            .collect();

        Ok(workflows)
    }

    fn find_by_id(&self, id: &str) -> Result<Option<Workflow>, RepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, version, description, definition, created_at, updated_at FROM workflows WHERE id = ?",
        )?;

        let workflow = stmt
            .query_row(params![id], |row| {
                Ok(WorkflowRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    description: row.get(3)?,
                    definition: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .ok()
            .map(|row| self.row_to_workflow(row));

        Ok(workflow)
    }

    fn save(&self, workflow: &Workflow) -> Result<Workflow, RepositoryError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO workflows (id, name, version, description, definition, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                workflow.id,
                workflow.name,
                workflow.version,
                workflow.description,
                serde_json::to_string(&workflow.definition)?,
                workflow.created_at.to_rfc3339(),
                workflow.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(workflow.clone())
    }

    fn update(&self, workflow: &Workflow) -> Result<Workflow, RepositoryError> {
        let conn = self.conn.lock();
        let rows_affected = conn.execute(
            "UPDATE workflows SET name = ?1, version = ?2, description = ?3, definition = ?4, updated_at = ?5 WHERE id = ?6",
            params![
                workflow.name,
                workflow.version,
                workflow.description,
                serde_json::to_string(&workflow.definition)?,
                workflow.updated_at.to_rfc3339(),
                workflow.id,
            ],
        )?;

        if rows_affected == 0 {
            return Err(RepositoryError::NotFound(workflow.id.clone()));
        }

        Ok(workflow.clone())
    }

    fn delete(&self, id: &str) -> Result<(), RepositoryError> {
        let conn = self.conn.lock();
        let rows_affected = conn.execute("DELETE FROM workflows WHERE id = ?", params![id])?;

        if rows_affected == 0 {
            return Err(RepositoryError::NotFound(id.to_string()));
        }

        Ok(())
    }
}

/// 带锁的仓储封装（用于 Arc 共享）
pub type SharedWorkflowRepository = Arc<dyn WorkflowRepository>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn create_temp_db_path() -> PathBuf {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("test_workflow_{}.db", uuid::Uuid::new_v4()));
        // Clean up any existing file
        let _ = fs::remove_file(&path);
        path
    }

    fn create_test_workflow() -> Workflow {
        Workflow {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Test Workflow".to_string(),
            version: "1.0".to_string(),
            description: Some("A test workflow".to_string()),
            definition: serde_json::json!({
                "stages": [
                    {"name": "stage1", "agents": ["agent1"]}
                ]
            }),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_save_and_find_by_id() {
        let db_path = create_temp_db_path();
        let repo = SqliteWorkflowRepository::new(&db_path).unwrap();

        let workflow = create_test_workflow();
        let saved = repo.save(&workflow).unwrap();
        assert_eq!(saved.id, workflow.id);

        let found = repo.find_by_id(&workflow.id).unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.name, workflow.name);
        assert_eq!(found.version, workflow.version);

        let _ = fs::remove_file(&db_path);
    }

    #[test]
    fn test_find_all() {
        let db_path = create_temp_db_path();
        let repo = SqliteWorkflowRepository::new(&db_path).unwrap();

        repo.save(&create_test_workflow()).unwrap();
        repo.save(&create_test_workflow()).unwrap();

        let all = repo.find_all().unwrap();
        assert_eq!(all.len(), 2);

        let _ = fs::remove_file(&db_path);
    }

    #[test]
    fn test_update() {
        let db_path = create_temp_db_path();
        let repo = SqliteWorkflowRepository::new(&db_path).unwrap();

        let mut workflow = create_test_workflow();
        repo.save(&workflow).unwrap();

        workflow.name = "Updated Workflow".to_string();
        let updated = repo.update(&workflow).unwrap();
        assert_eq!(updated.name, "Updated Workflow");

        let found = repo.find_by_id(&workflow.id).unwrap().unwrap();
        assert_eq!(found.name, "Updated Workflow");

        let _ = fs::remove_file(&db_path);
    }

    #[test]
    fn test_delete() {
        let db_path = create_temp_db_path();
        let repo = SqliteWorkflowRepository::new(&db_path).unwrap();

        let workflow = create_test_workflow();
        repo.save(&workflow).unwrap();

        repo.delete(&workflow.id).unwrap();

        let found = repo.find_by_id(&workflow.id).unwrap();
        assert!(found.is_none());

        let _ = fs::remove_file(&db_path);
    }

    #[test]
    fn test_delete_not_found() {
        let db_path = create_temp_db_path();
        let repo = SqliteWorkflowRepository::new(&db_path).unwrap();

        let result = repo.delete("non-existent-id");
        assert!(result.is_err());

        let _ = fs::remove_file(&db_path);
    }
}
