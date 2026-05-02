//! 会话仓库
//!
//! SQLite 实现的数据访问层。

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

use super::session_service::{Session, SessionStatus};

/// 仓库错误
#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("会话不存在: {0}")]
    NotFound(String),

    #[error("序列化错误: {0}")]
    Serialization(String),
}

/// 会话仓库 trait
pub trait SessionRepository: Send + Sync {
    fn create(&self, session: &Session) -> Result<(), RepositoryError>;
    fn find_by_id(&self, id: &str) -> Result<Option<Session>, RepositoryError>;
    fn find_by_resume_key(&self, resume_key: &str) -> Result<Option<Session>, RepositoryError>;
    fn find_all(&self) -> Result<Vec<Session>, RepositoryError>;
    fn update(&self, session: &Session) -> Result<(), RepositoryError>;
    fn delete(&self, id: &str) -> Result<bool, RepositoryError>;
}

/// SQLite 会话仓库
#[derive(Debug, Clone)]
pub struct SqliteSessionRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteSessionRepository {
    /// 创建新的 SQLite 仓库
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, RepositoryError> {
        let conn = Connection::open(db_path)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 创建内存仓库（用于测试）
    #[allow(dead_code)]
    pub fn in_memory() -> Result<Self, RepositoryError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(crate::migrations::SESSION_SCHEMA)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn status_to_str(status: SessionStatus) -> &'static str {
        match status {
            SessionStatus::Pending => "pending",
            SessionStatus::Active => "active",
            SessionStatus::Idle => "idle",
            SessionStatus::Paused => "paused",
            SessionStatus::Terminated => "terminated",
        }
    }

    fn str_to_status(s: &str) -> SessionStatus {
        match s {
            "pending" => SessionStatus::Pending,
            "active" => SessionStatus::Active,
            "idle" => SessionStatus::Idle,
            "paused" => SessionStatus::Paused,
            "terminated" => SessionStatus::Terminated,
            _ => SessionStatus::Pending,
        }
    }

    fn deserialize_row(
        id: String,
        workflow_id: Option<String>,
        status: String,
        resume_key: Option<String>,
        created_at: String,
        updated_at: String,
    ) -> Result<Session, RepositoryError> {
        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = DateTime::parse_from_rfc3339(&updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Session {
            id,
            workflow_id: workflow_id.unwrap_or_default(),
            status: Self::str_to_status(&status),
            resume_key,
            created_at,
            updated_at,
        })
    }
}

impl SessionRepository for SqliteSessionRepository {
    fn create(&self, session: &Session) -> Result<(), RepositoryError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO sessions (id, workflow_id, status, resume_key, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session.id,
                session.workflow_id,
                Self::status_to_str(session.status),
                session.resume_key,
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn find_by_id(&self, id: &str) -> Result<Option<Session>, RepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, workflow_id, status, resume_key, created_at, updated_at
             FROM sessions WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        });

        match result {
            Ok((id, workflow_id, status, resume_key, created_at, updated_at)) => Ok(Some(
                Self::deserialize_row(id, workflow_id, status, resume_key, created_at, updated_at)?,
            )),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn find_by_resume_key(&self, resume_key: &str) -> Result<Option<Session>, RepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, workflow_id, status, resume_key, created_at, updated_at
             FROM sessions WHERE resume_key = ?1",
        )?;

        let result = stmt.query_row(params![resume_key], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        });

        match result {
            Ok((id, workflow_id, status, resume_key, created_at, updated_at)) => Ok(Some(
                Self::deserialize_row(id, workflow_id, status, resume_key, created_at, updated_at)?,
            )),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn find_all(&self) -> Result<Vec<Session>, RepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, workflow_id, status, resume_key, created_at, updated_at
             FROM sessions ORDER BY updated_at DESC",
        )?;

        let rows = stmt.query_map([], |row: &rusqlite::Row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;

        let mut sessions = Vec::new();
        for row in rows {
            let (id, workflow_id, status, resume_key, created_at, updated_at) = row?;
            sessions.push(Self::deserialize_row(
                id,
                workflow_id,
                status,
                resume_key,
                created_at,
                updated_at,
            )?);
        }
        Ok(sessions)
    }

    fn update(&self, session: &Session) -> Result<(), RepositoryError> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "UPDATE sessions SET workflow_id = ?1, status = ?2, resume_key = ?3, updated_at = ?4
             WHERE id = ?5",
            params![
                session.workflow_id,
                Self::status_to_str(session.status),
                session.resume_key,
                session.updated_at.to_rfc3339(),
                session.id,
            ],
        )?;
        if affected == 0 {
            return Err(RepositoryError::NotFound(session.id.clone()));
        }
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<bool, RepositoryError> {
        let conn = self.conn.lock();
        let affected = conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_find() {
        let repo = SqliteSessionRepository::in_memory().unwrap();
        let session = Session::new("workflow-1".to_string());

        repo.create(&session).unwrap();

        let found = repo.find_by_id(&session.id).unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, session.id);
        assert_eq!(found.workflow_id, "workflow-1");
        assert_eq!(found.status, SessionStatus::Pending);
    }

    #[test]
    fn test_find_all() {
        let repo = SqliteSessionRepository::in_memory().unwrap();

        let session1 = Session::new("workflow-1".to_string());
        let session2 = Session::new("workflow-2".to_string());

        repo.create(&session1).unwrap();
        repo.create(&session2).unwrap();

        let all = repo.find_all().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_delete() {
        let repo = SqliteSessionRepository::in_memory().unwrap();
        let session = Session::new("workflow-1".to_string());

        repo.create(&session).unwrap();

        let deleted = repo.delete(&session.id).unwrap();
        assert!(deleted);

        let found = repo.find_by_id(&session.id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_update() {
        let repo = SqliteSessionRepository::in_memory().unwrap();
        let mut session = Session::new("workflow-1".to_string());

        repo.create(&session).unwrap();

        session.status = SessionStatus::Active;
        repo.update(&session).unwrap();

        let found = repo.find_by_id(&session.id).unwrap().unwrap();
        assert_eq!(found.status, SessionStatus::Active);
    }
}
