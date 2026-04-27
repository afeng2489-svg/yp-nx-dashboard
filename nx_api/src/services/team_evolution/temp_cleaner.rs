//! 临时数据清理 — 定期清理过期 checkpoint 和旧快照历史
//!
//! - execution_checkpoints: 7 天后清理 completed 记录
//! - role_snapshot_history: 30 天后清理

use parking_lot::Mutex;
use rusqlite::Connection;
use std::sync::Arc;

use super::error::TeamEvolutionError;

pub struct TempCleaner {
    conn: Arc<Mutex<Connection>>,
    /// Separate connection for snapshot_history table (may be on different DB connection)
    snapshot_conn: Option<Arc<Mutex<Connection>>>,
}

impl TempCleaner {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            conn,
            snapshot_conn: None,
        }
    }

    /// Set the connection for snapshot history operations
    pub fn with_snapshot_conn(mut self, conn: Arc<Mutex<Connection>>) -> Self {
        self.snapshot_conn = Some(conn);
        self
    }

    /// 清理 7 天前已完成的 checkpoint
    pub fn clean_old_checkpoints(&self) -> Result<u64, TeamEvolutionError> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "DELETE FROM execution_checkpoints
             WHERE phase = 'completed'
               AND datetime(last_heartbeat) < datetime('now', '-7 days')",
            [],
        )?;
        Ok(affected as u64)
    }

    /// 清理 30 天前的快照历史
    pub fn clean_old_snapshot_history(&self) -> Result<u64, TeamEvolutionError> {
        let conn = match &self.snapshot_conn {
            Some(c) => c,
            None => {
                tracing::debug!("[TempCleaner] No snapshot connection configured, skipping snapshot history cleanup");
                return Ok(0);
            }
        };
        let conn = conn.lock();
        let affected = conn.execute(
            "DELETE FROM role_snapshot_history
             WHERE datetime(created_at) < datetime('now', '-30 days')",
            [],
        )?;
        Ok(affected as u64)
    }

    /// 标记所有超过 1 小时仍为 running 的 checkpoint 为 interrupted
    pub fn mark_stale_checkpoints(&self) -> Result<u64, TeamEvolutionError> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "UPDATE execution_checkpoints
             SET phase = 'interrupted'
             WHERE phase = 'running'
               AND datetime(last_heartbeat) < datetime('now', '-1 hour')",
            [],
        )?;
        Ok(affected as u64)
    }

    /// 执行全部清理
    pub fn run_all(&self) -> Result<TempCleanResult, TeamEvolutionError> {
        let checkpoints = self.clean_old_checkpoints()?;
        let history = self.clean_old_snapshot_history()?;
        let stale = self.mark_stale_checkpoints()?;

        if checkpoints > 0 || history > 0 || stale > 0 {
            tracing::info!(
                "[TempCleaner] 清理完成: checkpoints={}, history={}, stale_marked={}",
                checkpoints,
                history,
                stale
            );
        }

        Ok(TempCleanResult {
            checkpoints_cleaned: checkpoints,
            history_cleaned: history,
            stale_marked: stale,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TempCleanResult {
    pub checkpoints_cleaned: u64,
    pub history_cleaned: u64,
    pub stale_marked: u64,
}
