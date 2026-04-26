//! 断点续跑 — execution_checkpoints 持久化 + 恢复提示词构建
//!
//! 核心问题修复: pty_task_watcher.rs:122-138 通道关闭直接发 Completed，
//! 无崩溃状态保存。本模块在每次 Progress/Output 事件时更新 checkpoint，
//! 启动时查找中断记录，构建续跑 prompt。

use std::sync::Arc;
use parking_lot::Mutex;
use rusqlite::Connection;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::TeamEvolutionError;
use super::feature_flag_service::FeatureFlagService;
use crate::models::feature_flag::keys;

/// 执行检查点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCheckpoint {
    pub id: String,
    pub execution_id: String,
    pub project_id: String,
    pub pipeline_step_id: Option<String>,
    pub role_id: String,
    pub task_prompt: String,
    pub accumulated_output: String,
    /// idle / running / completed / interrupted
    pub phase: String,
    pub started_at: String,
    pub last_heartbeat: String,
}

pub struct ResumeService {
    conn: Arc<Mutex<Connection>>,
    feature_flags: Arc<FeatureFlagService>,
}

impl ResumeService {
    pub fn new(conn: Arc<Mutex<Connection>>, feature_flags: Arc<FeatureFlagService>) -> Result<Self, TeamEvolutionError> {
        let svc = Self { conn, feature_flags };
        svc.initialize_tables()?;
        Ok(svc)
    }

    fn initialize_tables(&self) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS execution_checkpoints (
                id TEXT PRIMARY KEY,
                execution_id TEXT NOT NULL,
                project_id TEXT NOT NULL,
                pipeline_step_id TEXT,
                role_id TEXT NOT NULL,
                task_prompt TEXT NOT NULL,
                accumulated_output TEXT DEFAULT '',
                phase TEXT NOT NULL DEFAULT 'running',
                started_at TEXT NOT NULL,
                last_heartbeat TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_chk_exec ON execution_checkpoints(execution_id);
            CREATE INDEX IF NOT EXISTS idx_chk_project ON execution_checkpoints(project_id);
            CREATE INDEX IF NOT EXISTS idx_chk_heartbeat ON execution_checkpoints(last_heartbeat);"
        )?;
        Ok(())
    }

    /// 创建检查点（任务开始时调用）
    pub fn create_checkpoint(
        &self,
        execution_id: &str,
        project_id: &str,
        pipeline_step_id: Option<&str>,
        role_id: &str,
        task_prompt: &str,
    ) -> Result<ExecutionCheckpoint, TeamEvolutionError> {
        if !self.feature_flags.is_enabled(keys::CRASH_RESUME).unwrap_or(false) {
            return Err(TeamEvolutionError::FeatureDisabled(keys::CRASH_RESUME.to_string()));
        }

        let now = Utc::now().to_rfc3339();
        let checkpoint = ExecutionCheckpoint {
            id: Uuid::new_v4().to_string(),
            execution_id: execution_id.to_string(),
            project_id: project_id.to_string(),
            pipeline_step_id: pipeline_step_id.map(|s| s.to_string()),
            role_id: role_id.to_string(),
            task_prompt: task_prompt.to_string(),
            accumulated_output: String::new(),
            phase: "running".to_string(),
            started_at: now.clone(),
            last_heartbeat: now,
        };

        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO execution_checkpoints
                (id, execution_id, project_id, pipeline_step_id, role_id, task_prompt,
                 accumulated_output, phase, started_at, last_heartbeat)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            rusqlite::params![
                checkpoint.id, checkpoint.execution_id, checkpoint.project_id,
                checkpoint.pipeline_step_id, checkpoint.role_id, checkpoint.task_prompt,
                checkpoint.accumulated_output, checkpoint.phase,
                checkpoint.started_at, checkpoint.last_heartbeat,
            ],
        )?;
        Ok(checkpoint)
    }

    /// 更新检查点（每次 Progress/Output 事件时调用）
    pub fn update_checkpoint(
        &self,
        execution_id: &str,
        new_output: &str,
    ) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();

        // Append output (keep last 10000 chars to avoid bloat)
        conn.execute(
            "UPDATE execution_checkpoints
             SET accumulated_output = substr(accumulated_output || ?1, -10000),
                 last_heartbeat = ?2
             WHERE execution_id = ?3 AND phase = 'running'",
            rusqlite::params![new_output, now, execution_id],
        )?;
        Ok(())
    }

    /// 标记检查点为完成
    pub fn mark_completed(&self, execution_id: &str) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE execution_checkpoints SET phase = 'completed', last_heartbeat = ?1 WHERE execution_id = ?2",
            rusqlite::params![now, execution_id],
        )?;
        Ok(())
    }

    /// 查找被中断的检查点（last_heartbeat > 30s 未更新且 phase=running 或 interrupted）
    pub fn find_interrupted(&self) -> Result<Vec<ExecutionCheckpoint>, TeamEvolutionError> {
        if !self.feature_flags.is_enabled(keys::CRASH_RESUME).unwrap_or(false) {
            return Ok(vec![]);
        }

        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, execution_id, project_id, pipeline_step_id, role_id, task_prompt,
                    accumulated_output, phase, started_at, last_heartbeat
             FROM execution_checkpoints
             WHERE phase IN ('running', 'interrupted')
               AND datetime(last_heartbeat) < datetime('now', '-30 seconds')
             ORDER BY last_heartbeat DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(ExecutionCheckpoint {
                id: row.get(0)?,
                execution_id: row.get(1)?,
                project_id: row.get(2)?,
                pipeline_step_id: row.get(3)?,
                role_id: row.get(4)?,
                task_prompt: row.get(5)?,
                accumulated_output: row.get(6)?,
                phase: row.get(7)?,
                started_at: row.get(8)?,
                last_heartbeat: row.get(9)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// 构建恢复提示词
    pub fn build_resume_prompt(&self, checkpoint: &ExecutionCheckpoint) -> String {
        let output_summary = if checkpoint.accumulated_output.len() > 2000 {
            format!("...{}...", &checkpoint.accumulated_output[checkpoint.accumulated_output.len() - 2000..])
        } else {
            checkpoint.accumulated_output.clone()
        };

        format!(
            r#"## 任务恢复

上次执行在 {last_hb} 被中断。

### 原始任务
{task}

### 已完成的工作（摘要）
{output}

### 指令
请从断点继续执行上述任务。不要重新开始已完成的部分，直接接着做。
"#,
            last_hb = checkpoint.last_heartbeat,
            task = checkpoint.task_prompt,
            output = output_summary,
        )
    }

    /// 删除检查点
    pub fn delete_checkpoint(&self, execution_id: &str) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM execution_checkpoints WHERE execution_id = ?1",
            rusqlite::params![execution_id],
        )?;
        Ok(())
    }

    /// 获取项目的检查点
    pub fn find_by_project(&self, project_id: &str) -> Result<Vec<ExecutionCheckpoint>, TeamEvolutionError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, execution_id, project_id, pipeline_step_id, role_id, task_prompt,
                    accumulated_output, phase, started_at, last_heartbeat
             FROM execution_checkpoints
             WHERE project_id = ?1
             ORDER BY started_at DESC"
        )?;
        let rows = stmt.query_map(rusqlite::params![project_id], |row| {
            Ok(ExecutionCheckpoint {
                id: row.get(0)?,
                execution_id: row.get(1)?,
                project_id: row.get(2)?,
                pipeline_step_id: row.get(3)?,
                role_id: row.get(4)?,
                task_prompt: row.get(5)?,
                accumulated_output: row.get(6)?,
                phase: row.get(7)?,
                started_at: row.get(8)?,
                last_heartbeat: row.get(9)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}
