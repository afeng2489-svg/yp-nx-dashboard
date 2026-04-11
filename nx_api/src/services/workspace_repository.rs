//! 工作区仓库
//!
//! SQLite 实现的数据访问层。

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// 仓库错误
#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("工作区不存在: {0}")]
    NotFound(String),

    #[error("序列化错误: {0}")]
    Serialization(String),
}

/// 工作区模型
#[derive(Debug, Clone, serde::Serialize)]
pub struct Workspace {
    /// 工作区 ID
    pub id: String,
    /// 工作区名称
    pub name: String,
    /// 工作区描述
    pub description: Option<String>,
    /// 所有者 ID
    pub owner_id: String,
    /// 工作区根目录路径
    pub root_path: Option<String>,
    /// 工作区配置（JSON）
    pub settings: serde_json::Value,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

impl Workspace {
    /// 创建新的工作区
    pub fn new(name: String, owner_id: String, description: Option<String>, root_path: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            owner_id,
            root_path,
            settings: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        }
    }

    /// 设置工作区根目录
    pub fn set_root_path(&mut self, path: String) {
        self.root_path = Some(path);
    }
}

/// 工作区仓库 trait
pub trait WorkspaceRepository: Send + Sync {
    /// 创建工作区
    fn create(&self, workspace: &Workspace) -> Result<(), RepositoryError>;

    /// 根据 ID 查找工作区
    fn find_by_id(&self, id: &str) -> Result<Option<Workspace>, RepositoryError>;

    /// 查找所有工作区
    fn find_all(&self) -> Result<Vec<Workspace>, RepositoryError>;

    /// 根据所有者查找工作区
    fn find_by_owner(&self, owner_id: &str) -> Result<Vec<Workspace>, RepositoryError>;

    /// 更新工作区
    fn update(&self, workspace: &Workspace) -> Result<(), RepositoryError>;

    /// 删除工作区
    fn delete(&self, id: &str) -> Result<bool, RepositoryError>;
}

/// SQLite 工作区仓库
#[derive(Debug, Clone)]
pub struct SqliteWorkspaceRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteWorkspaceRepository {
    /// 创建新的 SQLite 仓库
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, RepositoryError> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS workspaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                owner_id TEXT NOT NULL,
                root_path TEXT,
                settings TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_workspaces_owner ON workspaces(owner_id);
            CREATE INDEX IF NOT EXISTS idx_workspaces_updated_at ON workspaces(updated_at);",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 创建内存仓库（用于测试）
    #[allow(dead_code)]
    pub fn in_memory() -> Result<Self, RepositoryError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS workspaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                owner_id TEXT NOT NULL,
                root_path TEXT,
                settings TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_workspaces_owner ON workspaces(owner_id);",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn deserialize_row(
        id: String,
        name: String,
        description: Option<String>,
        owner_id: String,
        root_path: Option<String>,
        settings: String,
        created_at: String,
        updated_at: String,
    ) -> Result<Workspace, RepositoryError> {
        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = DateTime::parse_from_rfc3339(&updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let settings = serde_json::from_str(&settings)
            .unwrap_or_else(|_| serde_json::json!({}));

        Ok(Workspace {
            id,
            name,
            description,
            owner_id,
            root_path,
            settings,
            created_at,
            updated_at,
        })
    }
}

impl WorkspaceRepository for SqliteWorkspaceRepository {
    fn create(&self, workspace: &Workspace) -> Result<(), RepositoryError> {
        let conn = self.conn.lock();
        let settings_json = serde_json::to_string(&workspace.settings)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;

        conn.execute(
            "INSERT INTO workspaces (id, name, description, owner_id, root_path, settings, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                workspace.id,
                workspace.name,
                workspace.description,
                workspace.owner_id,
                workspace.root_path,
                settings_json,
                workspace.created_at.to_rfc3339(),
                workspace.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn find_by_id(&self, id: &str) -> Result<Option<Workspace>, RepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, owner_id, root_path, settings, created_at, updated_at
             FROM workspaces WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        });

        match result {
            Ok((id, name, description, owner_id, root_path, settings, created_at, updated_at)) => {
                Ok(Some(Self::deserialize_row(
                    id,
                    name,
                    description,
                    owner_id,
                    root_path,
                    settings,
                    created_at,
                    updated_at,
                )?))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn find_all(&self) -> Result<Vec<Workspace>, RepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, owner_id, root_path, settings, created_at, updated_at
             FROM workspaces ORDER BY updated_at DESC",
        )?;

        let rows = stmt.query_map([], |row: &rusqlite::Row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        })?;

        let mut workspaces = Vec::new();
        for row in rows {
            let (id, name, description, owner_id, root_path, settings, created_at, updated_at) = row?;
            workspaces.push(Self::deserialize_row(
                id,
                name,
                description,
                owner_id,
                root_path,
                settings,
                created_at,
                updated_at,
            )?);
        }
        Ok(workspaces)
    }

    fn find_by_owner(&self, owner_id: &str) -> Result<Vec<Workspace>, RepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, owner_id, root_path, settings, created_at, updated_at
             FROM workspaces WHERE owner_id = ?1 ORDER BY updated_at DESC",
        )?;

        let rows = stmt.query_map(params![owner_id], |row: &rusqlite::Row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        })?;

        let mut workspaces = Vec::new();
        for row in rows {
            let (id, name, description, owner_id, root_path, settings, created_at, updated_at) = row?;
            workspaces.push(Self::deserialize_row(
                id,
                name,
                description,
                owner_id,
                root_path,
                settings,
                created_at,
                updated_at,
            )?);
        }
        Ok(workspaces)
    }

    fn update(&self, workspace: &Workspace) -> Result<(), RepositoryError> {
        let conn = self.conn.lock();
        let settings_json = serde_json::to_string(&workspace.settings)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;

        let affected = conn.execute(
            "UPDATE workspaces SET name = ?1, description = ?2, root_path = ?3, settings = ?4, updated_at = ?5
             WHERE id = ?6",
            params![
                workspace.name,
                workspace.description,
                workspace.root_path,
                settings_json,
                workspace.updated_at.to_rfc3339(),
                workspace.id,
            ],
        )?;
        if affected == 0 {
            return Err(RepositoryError::NotFound(workspace.id.clone()));
        }
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<bool, RepositoryError> {
        let conn = self.conn.lock();
        let affected = conn.execute("DELETE FROM workspaces WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_find() {
        let repo = SqliteWorkspaceRepository::in_memory().unwrap();
        let workspace = Workspace::new(
            "Test Workspace".to_string(),
            "owner-1".to_string(),
            Some("A test workspace".to_string()),
            None,
        );

        repo.create(&workspace).unwrap();

        let found = repo.find_by_id(&workspace.id).unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, workspace.id);
        assert_eq!(found.name, "Test Workspace");
        assert_eq!(found.owner_id, "owner-1");
    }

    #[test]
    fn test_find_by_owner() {
        let repo = SqliteWorkspaceRepository::in_memory().unwrap();

        let workspace1 = Workspace::new("Workspace 1".to_string(), "owner-1".to_string(), None, None);
        let workspace2 = Workspace::new("Workspace 2".to_string(), "owner-2".to_string(), None, None);

        repo.create(&workspace1).unwrap();
        repo.create(&workspace2).unwrap();

        let owned = repo.find_by_owner("owner-1").unwrap();
        assert_eq!(owned.len(), 1);
        assert_eq!(owned[0].name, "Workspace 1");
    }

    #[test]
    fn test_update() {
        let repo = SqliteWorkspaceRepository::in_memory().unwrap();
        let mut workspace = Workspace::new("Original".to_string(), "owner-1".to_string(), None, None);

        repo.create(&workspace).unwrap();

        workspace.name = "Updated".to_string();
        workspace.updated_at = Utc::now();
        repo.update(&workspace).unwrap();

        let found = repo.find_by_id(&workspace.id).unwrap().unwrap();
        assert_eq!(found.name, "Updated");
    }

    #[test]
    fn test_delete() {
        let repo = SqliteWorkspaceRepository::in_memory().unwrap();
        let workspace = Workspace::new("To Delete".to_string(), "owner-1".to_string(), None, None);

        repo.create(&workspace).unwrap();

        let deleted = repo.delete(&workspace.id).unwrap();
        assert!(deleted);

        let found = repo.find_by_id(&workspace.id).unwrap();
        assert!(found.is_none());
    }
}
