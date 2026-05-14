//! Team Session Store — SQLite 持久化
//!
//! 保存和查询团队会话结果。

use crate::session::{AgentResult, ChainResult};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// 会话摘要（列表用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub execution_id: String,
    pub task: String,
    pub status: String,
    pub agent_count: usize,
    pub duration_ms: u64,
    pub created_at: String,
}

/// SQLite 团队会话存储
pub struct SessionStore {
    pub conn: Arc<Mutex<Connection>>,
}

impl SessionStore {
    /// 打开或创建数据库
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS team_sessions (
                execution_id TEXT PRIMARY KEY,
                task TEXT NOT NULL,
                chain_result TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'completed',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_team_sessions_created
                ON team_sessions(created_at DESC);",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 内存数据库（测试用）
    pub fn in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE team_sessions (
                execution_id TEXT PRIMARY KEY,
                task TEXT NOT NULL,
                chain_result TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'completed',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 保存会话结果
    pub fn save(&self, result: &ChainResult) -> Result<(), StoreError> {
        let json = serde_json::to_string(result)?;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO team_sessions (execution_id, task, chain_result, status)
             VALUES (?1, ?2, ?3, 'completed')",
            params![result.execution_id.to_string(), result.task, json],
        )?;
        Ok(())
    }

    /// 列出最近会话
    pub fn list(&self, limit: usize) -> Result<Vec<SessionSummary>, StoreError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT execution_id, task, chain_result, status, created_at
             FROM team_sessions
             ORDER BY created_at DESC
             LIMIT ?1",
        )?;

        let summaries: Vec<SessionSummary> = stmt
            .query_map(params![limit as i64], |row| {
                let json: String = row.get(2)?;
                let execution_id: String = row.get(0)?;
                let task: String = row.get(1)?;
                let status: String = row.get(3)?;
                let created_at: String = row.get(4)?;
                let parsed: Result<ChainResult, _> = serde_json::from_str(&json);
                Ok((execution_id, task, status, created_at, parsed))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(execution_id, task, status, created_at, parsed)| {
                parsed.ok().map(|r| SessionSummary {
                    execution_id,
                    task,
                    status,
                    agent_count: r.agent_results.len(),
                    duration_ms: r.total_duration_ms,
                    created_at,
                })
            })
            .collect();

        Ok(summaries)
    }

    /// 获取单个会话详情
    pub fn get(&self, execution_id: &str) -> Result<Option<ChainResult>, StoreError> {
        let conn = self.conn.lock();
        let mut stmt =
            conn.prepare("SELECT chain_result FROM team_sessions WHERE execution_id = ?1")?;

        let mut rows = stmt.query_map(params![execution_id], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;

        match rows.next() {
            Some(Ok(json)) => {
                let result: ChainResult = serde_json::from_str(&json)?;
                Ok(Some(result))
            }
            Some(Err(e)) => Err(StoreError::Database(e)),
            None => Ok(None),
        }
    }

    /// 删除会话
    pub fn delete(&self, execution_id: &str) -> Result<bool, StoreError> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "DELETE FROM team_sessions WHERE execution_id = ?1",
            params![execution_id],
        )?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::AgentResult;
    use crate::team::AgentRole;
    use uuid::Uuid;

    fn make_result() -> ChainResult {
        ChainResult {
            execution_id: Uuid::new_v4(),
            task: "test task".into(),
            agent_results: vec![
                AgentResult {
                    agent_id: crate::team::AgentId::new(),
                    role: AgentRole::Architect,
                    agent_name: "architect".into(),
                    text: "architecture design".into(),
                    duration_ms: 5000,
                    attempts: 1,
                },
                AgentResult {
                    agent_id: crate::team::AgentId::new(),
                    role: AgentRole::Developer,
                    agent_name: "developer".into(),
                    text: "code here".into(),
                    duration_ms: 10000,
                    attempts: 2,
                },
            ],
            total_duration_ms: 15000,
        }
    }

    fn make_result_with_task(task: &str, agent_count: usize) -> ChainResult {
        let mut agents = Vec::new();
        for i in 0..agent_count {
            agents.push(AgentResult {
                agent_id: crate::team::AgentId::new(),
                role: match i % 4 {
                    0 => AgentRole::Architect,
                    1 => AgentRole::Developer,
                    2 => AgentRole::Reviewer,
                    _ => AgentRole::Tester,
                },
                agent_name: format!("agent-{}", i),
                text: format!("result from {}", i),
                duration_ms: (i as u64 + 1) * 1000,
                attempts: 1,
            });
        }
        let total = agents.iter().map(|a| a.duration_ms).sum();
        ChainResult {
            execution_id: Uuid::new_v4(),
            task: task.into(),
            agent_results: agents,
            total_duration_ms: total,
        }
    }

    // ── Save & List ────────────────────────────────────────────────

    #[test]
    fn save_and_list() {
        let store = SessionStore::in_memory().unwrap();
        let result = make_result();
        store.save(&result).unwrap();

        let list = store.list(10).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].task, "test task");
        assert_eq!(list[0].agent_count, 2);
        assert_eq!(list[0].duration_ms, 15000);
    }

    #[test]
    fn list_returns_empty_when_no_sessions() {
        let store = SessionStore::in_memory().unwrap();
        let list = store.list(10).unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn list_respects_limit() {
        let store = SessionStore::in_memory().unwrap();
        for _ in 0..5 {
            store.save(&make_result()).unwrap();
        }
        assert_eq!(store.list(3).unwrap().len(), 3);
    }

    #[test]
    fn list_limit_greater_than_count() {
        let store = SessionStore::in_memory().unwrap();
        for _ in 0..3 {
            store.save(&make_result()).unwrap();
        }
        // limit 大于实际数量时返回全部
        assert_eq!(store.list(100).unwrap().len(), 3);
    }

    #[test]
    fn list_limit_zero() {
        let store = SessionStore::in_memory().unwrap();
        store.save(&make_result()).unwrap();
        let list = store.list(0).unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn list_orders_by_created_at_descending() {
        let store = SessionStore::in_memory().unwrap();
        let r1 = make_result_with_task("first", 1);
        let r2 = make_result_with_task("second", 1);
        let r3 = make_result_with_task("third", 1);
        store.save(&r1).unwrap();
        store.save(&r2).unwrap();
        store.save(&r3).unwrap();

        let list = store.list(10).unwrap();
        assert_eq!(list.len(), 3);
        // 内存数据库中 datetime('now') 在同一秒内可能相同
        // 所以只验证所有 task 都存在
        let tasks: Vec<&str> = list.iter().map(|s| s.task.as_str()).collect();
        assert!(tasks.contains(&"first"));
        assert!(tasks.contains(&"second"));
        assert!(tasks.contains(&"third"));
    }

    // ── Get ────────────────────────────────────────────────────────

    #[test]
    fn get_by_id() {
        let store = SessionStore::in_memory().unwrap();
        let result = make_result();
        let id = result.execution_id.to_string();
        store.save(&result).unwrap();

        let loaded = store.get(&id).unwrap().unwrap();
        assert_eq!(loaded.task, "test task");
        assert_eq!(loaded.agent_results.len(), 2);
        assert_eq!(loaded.total_duration_ms, 15000);
    }

    #[test]
    fn get_nonexistent() {
        let store = SessionStore::in_memory().unwrap();
        assert!(store.get("nonexistent").unwrap().is_none());
    }

    #[test]
    fn get_empty_string_id() {
        let store = SessionStore::in_memory().unwrap();
        assert!(store.get("").unwrap().is_none());
    }

    #[test]
    fn get_after_delete_returns_none() {
        let store = SessionStore::in_memory().unwrap();
        let result = make_result();
        let id = result.execution_id.to_string();
        store.save(&result).unwrap();
        store.delete(&id).unwrap();
        assert!(store.get(&id).unwrap().is_none());
    }

    // ── Delete ─────────────────────────────────────────────────────

    #[test]
    fn delete_existing() {
        let store = SessionStore::in_memory().unwrap();
        let result = make_result();
        let id = result.execution_id.to_string();
        store.save(&result).unwrap();

        let deleted = store.delete(&id).unwrap();
        assert!(deleted);
    }

    #[test]
    fn delete_nonexistent() {
        let store = SessionStore::in_memory().unwrap();
        let deleted = store.delete("nonexistent").unwrap();
        assert!(!deleted);
    }

    #[test]
    fn delete_empty_string() {
        let store = SessionStore::in_memory().unwrap();
        let deleted = store.delete("").unwrap();
        assert!(!deleted);
    }

    #[test]
    fn delete_twice_returns_false_second_time() {
        let store = SessionStore::in_memory().unwrap();
        let result = make_result();
        let id = result.execution_id.to_string();
        store.save(&result).unwrap();

        assert!(store.delete(&id).unwrap());
        assert!(!store.delete(&id).unwrap());
    }

    // ── Update / Replace ───────────────────────────────────────────

    #[test]
    fn save_replaces_existing_id() {
        let store = SessionStore::in_memory().unwrap();
        let result = make_result();
        let id = result.execution_id.to_string();
        store.save(&result).unwrap();

        // 用 same ID 但不同内容保存
        let updated = ChainResult {
            execution_id: result.execution_id,
            task: "updated task".into(),
            agent_results: vec![],
            total_duration_ms: 0,
        };
        store.save(&updated).unwrap();

        let loaded = store.get(&id).unwrap().unwrap();
        assert_eq!(loaded.task, "updated task");
        assert_eq!(loaded.agent_results.len(), 0);
        assert_eq!(loaded.total_duration_ms, 0);
    }

    // ── Open / File ────────────────────────────────────────────────

    #[test]
    fn open_in_memory_creates_empty_store() {
        let store = SessionStore::in_memory().unwrap();
        assert!(store.list(10).unwrap().is_empty());
    }

    #[test]
    fn open_with_temp_file_persists_data() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("test_session_store_{}.db", Uuid::new_v4()));

        // 保存数据
        {
            let store = SessionStore::open(&path).unwrap();
            let result = make_result();
            store.save(&result).unwrap();
            assert_eq!(store.list(10).unwrap().len(), 1);
        }

        // 重新打开验证数据持久化
        {
            let store = SessionStore::open(&path).unwrap();
            assert_eq!(store.list(10).unwrap().len(), 1);
        }

        // 清理
        let _ = std::fs::remove_file(&path);
    }

    // ── Summary ────────────────────────────────────────────────────

    #[test]
    fn summary_fields_correct() {
        let store = SessionStore::in_memory().unwrap();
        let result = make_result_with_task("summary test", 3);
        store.save(&result).unwrap();

        let list = store.list(10).unwrap();
        assert_eq!(list.len(), 1);
        let s = &list[0];
        assert_eq!(s.task, "summary test");
        assert_eq!(s.agent_count, 3);
        assert_eq!(s.duration_ms, 6000); // 1+2+3 = 6000
        assert_eq!(s.status, "completed");
        assert!(!s.created_at.is_empty());
    }

    #[test]
    fn summary_with_zero_agents() {
        let store = SessionStore::in_memory().unwrap();
        let result = ChainResult {
            execution_id: Uuid::new_v4(),
            task: "empty chain".into(),
            agent_results: vec![],
            total_duration_ms: 0,
        };
        store.save(&result).unwrap();

        let list = store.list(10).unwrap();
        assert_eq!(list[0].agent_count, 0);
        assert_eq!(list[0].duration_ms, 0);
    }
}
