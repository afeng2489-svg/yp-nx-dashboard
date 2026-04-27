//! 会话持久化
//!
//! 支持会话状态保存到 SQLite 数据库。

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

use super::{Session, SessionId, SessionMetadata, SessionStatus};

/// 持久化错误
#[derive(Error, Debug)]
pub enum PersistenceError {
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("会话不存在: {0}")]
    SessionNotFound(SessionId),

    #[error("序列化错误: {0}")]
    Serialization(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}

/// SQLite 会话存储
#[derive(Debug)]
pub struct SessionStore {
    /// 数据库连接
    conn: Arc<Mutex<Connection>>,
}

impl SessionStore {
    /// 创建新的会话存储
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, PersistenceError> {
        let conn = Connection::open(db_path)?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                metadata TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                terminated_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
            CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at);
            ",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 创建内存存储（用于测试）
    pub fn in_memory() -> Result<Self, PersistenceError> {
        let conn = Connection::open_in_memory()?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                metadata TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                terminated_at TEXT
            );
            ",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 保存会话
    pub fn save(&self, session: &Session) -> Result<(), PersistenceError> {
        let conn = self.conn.lock();

        let metadata_json = serde_json::to_string(&session.metadata)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        conn.execute(
            "INSERT OR REPLACE INTO sessions
             (id, status, metadata, created_at, updated_at, terminated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session.id.0,
                session.status.to_string(),
                metadata_json,
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
                session.terminated_at.map(|t| t.to_rfc3339()),
            ],
        )?;

        Ok(())
    }

    /// 获取会话
    pub fn get(&self, id: &SessionId) -> Result<Session, PersistenceError> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            "SELECT id, status, metadata, created_at, updated_at, terminated_at
             FROM sessions WHERE id = ?1",
        )?;

        let session = stmt.query_row(params![id.0], |row| {
            let id_str: String = row.get(0)?;
            let status_str: String = row.get(1)?;
            let metadata_json: String = row.get(2)?;
            let created_at_str: String = row.get(3)?;
            let updated_at_str: String = row.get(4)?;
            let terminated_at_str: Option<String> = row.get(5)?;

            Ok((
                id_str,
                status_str,
                metadata_json,
                created_at_str,
                updated_at_str,
                terminated_at_str,
            ))
        })?;

        let (id_str, status_str, metadata_json, created_at_str, updated_at_str, terminated_at_str) =
            session;

        let metadata: SessionMetadata = serde_json::from_str(&metadata_json)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        let status = match status_str.as_str() {
            "pending" => SessionStatus::Pending,
            "running" => SessionStatus::Running,
            "active" => SessionStatus::Active,
            "idle" => SessionStatus::Idle,
            "paused" => SessionStatus::Paused,
            "terminated" => SessionStatus::Terminated,
            _ => SessionStatus::Pending,
        };

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let terminated_at = terminated_at_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        });

        Ok(Session {
            id: SessionId(id_str),
            status,
            metadata,
            created_at,
            updated_at,
            terminated_at,
        })
    }

    /// 删除会话
    pub fn delete(&self, id: &SessionId) -> Result<bool, PersistenceError> {
        let conn = self.conn.lock();

        let affected = conn.execute("DELETE FROM sessions WHERE id = ?1", params![id.0])?;

        Ok(affected > 0)
    }

    /// 列出所有会话
    pub fn list(&self) -> Result<Vec<Session>, PersistenceError> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            "SELECT id, status, metadata, created_at, updated_at, terminated_at
             FROM sessions ORDER BY updated_at DESC",
        )?;

        let sessions = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let status_str: String = row.get(1)?;
            let metadata_json: String = row.get(2)?;
            let created_at_str: String = row.get(3)?;
            let updated_at_str: String = row.get(4)?;
            let terminated_at_str: Option<String> = row.get(5)?;

            Ok((
                id_str,
                status_str,
                metadata_json,
                created_at_str,
                updated_at_str,
                terminated_at_str,
            ))
        })?;

        let mut result = Vec::new();
        for session_result in sessions {
            let (
                id_str,
                status_str,
                metadata_json,
                created_at_str,
                updated_at_str,
                terminated_at_str,
            ) = session_result?;

            if let Ok(session) = self.deserialize_session(
                &id_str,
                &status_str,
                &metadata_json,
                &created_at_str,
                &updated_at_str,
                terminated_at_str.as_deref(),
            ) {
                result.push(session);
            }
        }

        Ok(result)
    }

    /// 按状态列出会话
    pub fn list_by_status(&self, status: SessionStatus) -> Result<Vec<Session>, PersistenceError> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            "SELECT id, status, metadata, created_at, updated_at, terminated_at
             FROM sessions WHERE status = ?1 ORDER BY updated_at DESC",
        )?;

        let sessions = stmt.query_map(params![status.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let status_str: String = row.get(1)?;
            let metadata_json: String = row.get(2)?;
            let created_at_str: String = row.get(3)?;
            let updated_at_str: String = row.get(4)?;
            let terminated_at_str: Option<String> = row.get(5)?;

            Ok((
                id_str,
                status_str,
                metadata_json,
                created_at_str,
                updated_at_str,
                terminated_at_str,
            ))
        })?;

        let mut result = Vec::new();
        for session_result in sessions {
            let (
                id_str,
                status_str,
                metadata_json,
                created_at_str,
                updated_at_str,
                terminated_at_str,
            ) = session_result?;

            if let Ok(session) = self.deserialize_session(
                &id_str,
                &status_str,
                &metadata_json,
                &created_at_str,
                &updated_at_str,
                terminated_at_str.as_deref(),
            ) {
                result.push(session);
            }
        }

        Ok(result)
    }

    /// 清理旧会话（保留最近 N 个）
    pub fn cleanup_old(&self, keep_count: usize) -> Result<usize, PersistenceError> {
        let conn = self.conn.lock();

        let affected = conn.execute(
            "DELETE FROM sessions WHERE id NOT IN (
                SELECT id FROM sessions ORDER BY updated_at DESC LIMIT ?1
            )",
            params![keep_count],
        )?;

        Ok(affected)
    }

    /// 获取会话数量
    pub fn count(&self) -> Result<usize, PersistenceError> {
        let conn = self.conn.lock();

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;

        Ok(count as usize)
    }

    fn deserialize_session(
        &self,
        id_str: &str,
        status_str: &str,
        metadata_json: &str,
        created_at_str: &str,
        updated_at_str: &str,
        terminated_at_str: Option<&str>,
    ) -> Result<Session, PersistenceError> {
        let metadata: SessionMetadata = serde_json::from_str(metadata_json)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        let status = match status_str {
            "pending" => SessionStatus::Pending,
            "running" => SessionStatus::Running,
            "active" => SessionStatus::Active,
            "idle" => SessionStatus::Idle,
            "paused" => SessionStatus::Paused,
            "terminated" => SessionStatus::Terminated,
            _ => SessionStatus::Pending,
        };

        let created_at = DateTime::parse_from_rfc3339(created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = DateTime::parse_from_rfc3339(updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let terminated_at = terminated_at_str.and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        });

        Ok(Session {
            id: SessionId(id_str.to_string()),
            status,
            metadata,
            created_at,
            updated_at,
            terminated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_store() {
        let store = SessionStore::in_memory().unwrap();

        // 创建并保存会话
        let session = Session::new();
        store.save(&session).unwrap();

        // 获取会话
        let retrieved = store.get(&session.id).unwrap();
        assert_eq!(retrieved.id, session.id);
        assert_eq!(retrieved.status, session.status);

        // 列出所有会话
        let sessions = store.list().unwrap();
        assert_eq!(sessions.len(), 1);

        // 删除会话
        let deleted = store.delete(&session.id).unwrap();
        assert!(deleted);

        // 验证删除
        let sessions = store.list().unwrap();
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let store = SessionStore::in_memory().unwrap();

        let session1 = Session::new();
        let mut session2 = Session::new();
        session2.activate();

        store.save(&session1).unwrap();
        store.save(&session2).unwrap();

        let active_sessions = store.list_by_status(SessionStatus::Active).unwrap();
        assert_eq!(active_sessions.len(), 1);
        assert_eq!(active_sessions[0].id, session2.id);

        let pending_sessions = store.list_by_status(SessionStatus::Pending).unwrap();
        assert_eq!(pending_sessions.len(), 1);
    }
}
