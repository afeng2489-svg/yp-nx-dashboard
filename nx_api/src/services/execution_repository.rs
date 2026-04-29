//! 执行仓储层
//!
//! 持久化工作流执行记录到 SQLite，重启后历史记录不丢失。

use crate::services::execution_service::{Execution, StageResult};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutionRepositoryError {
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("JSON 序列化错误: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct SqliteExecutionRepository {
    conn: Mutex<Connection>,
}

impl SqliteExecutionRepository {
    pub fn new(db_path: &Path) -> Result<Self, ExecutionRepositoryError> {
        let conn = Connection::open(db_path)?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn init_schema(conn: &Connection) -> Result<(), ExecutionRepositoryError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS executions (
                id            TEXT PRIMARY KEY,
                workflow_id   TEXT NOT NULL,
                status        TEXT NOT NULL DEFAULT 'pending',
                variables     TEXT NOT NULL DEFAULT '{}',
                error         TEXT,
                started_at    TEXT,
                finished_at   TEXT
            );
            CREATE TABLE IF NOT EXISTS stage_results (
                id            TEXT PRIMARY KEY,
                execution_id  TEXT NOT NULL,
                stage_name    TEXT NOT NULL,
                outputs       TEXT NOT NULL DEFAULT '[]',
                completed_at  TEXT,
                FOREIGN KEY (execution_id) REFERENCES executions(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_executions_workflow
                ON executions(workflow_id);
            CREATE INDEX IF NOT EXISTS idx_stage_results_execution
                ON stage_results(execution_id);",
        )?;
        Ok(())
    }

    /// 保存执行记录（新建）
    pub fn insert(&self, execution: &Execution) -> Result<(), ExecutionRepositoryError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO executions (id, workflow_id, status, variables, error, started_at, finished_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                execution.id,
                execution.workflow_id,
                status_to_str(execution.status),
                serde_json::to_string(&execution.variables)?,
                execution.error,
                execution.started_at.map(|t| t.to_rfc3339()),
                execution.finished_at.map(|t| t.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    /// 更新执行状态
    pub fn update_status(
        &self,
        id: &str,
        status: &str,
        error: Option<&str>,
        finished_at: Option<&str>,
    ) -> Result<(), ExecutionRepositoryError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE executions SET status = ?1, error = ?2, finished_at = ?3 WHERE id = ?4",
            params![status, error, finished_at, id],
        )?;
        Ok(())
    }

    /// 添加阶段结果
    pub fn insert_stage_result(
        &self,
        execution_id: &str,
        sr: &StageResult,
    ) -> Result<(), ExecutionRepositoryError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO stage_results (id, execution_id, stage_name, outputs, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                uuid::Uuid::new_v4().to_string(),
                execution_id,
                sr.stage_name,
                serde_json::to_string(&sr.outputs)?,
                sr.completed_at.map(|t| t.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    /// 查询单个执行
    pub fn find_by_id(&self, id: &str) -> Result<Option<Execution>, ExecutionRepositoryError> {
        let conn = self.conn.lock().unwrap();
        find_by_id_with_conn(&conn, id)
    }

    /// 查询所有执行（最新在前）
    pub fn find_all(&self) -> Result<Vec<Execution>, ExecutionRepositoryError> {
        let conn = self.conn.lock().unwrap();
        find_all_with_conn(&conn)
    }
}

fn find_by_id_with_conn(
    conn: &Connection,
    id: &str,
) -> Result<Option<Execution>, ExecutionRepositoryError> {
    let mut stmt = conn.prepare(
        "SELECT id, workflow_id, status, variables, error, started_at, finished_at
         FROM executions WHERE id = ?1",
    )?;

    let exec = match stmt.query_row(params![id], row_to_execution) {
        Ok(e) => Some(e),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(e.into()),
    };

    if let Some(mut exec) = exec {
        exec.stage_results = find_stage_results_with_conn(conn, id)?;
        return Ok(Some(exec));
    }
    Ok(None)
}

fn find_all_with_conn(conn: &Connection) -> Result<Vec<Execution>, ExecutionRepositoryError> {
    let mut stmt = conn.prepare(
        "SELECT id, workflow_id, status, variables, error, started_at, finished_at
         FROM executions ORDER BY started_at DESC",
    )?;

    let rows = stmt.query_map([], row_to_execution)?;
    let mut out = Vec::new();
    for row in rows {
        let mut exec = row?;
        exec.stage_results = find_stage_results_with_conn(conn, &exec.id)?;
        out.push(exec);
    }
    Ok(out)
}

fn find_stage_results_with_conn(
    conn: &Connection,
    execution_id: &str,
) -> Result<Vec<StageResult>, ExecutionRepositoryError> {
    let mut stmt = conn.prepare(
        "SELECT stage_name, outputs, completed_at
         FROM stage_results WHERE execution_id = ?1 ORDER BY completed_at ASC",
    )?;
    let rows = stmt.query_map(params![execution_id], |row| {
        let outputs_str: String = row.get(1)?;
        Ok(StageResult {
            stage_name: row.get(0)?,
            outputs: serde_json::from_str(&outputs_str).unwrap_or_default(),
            completed_at: row.get::<_, Option<String>>(2)?.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|t| t.with_timezone(&chrono::Utc))
                    .ok()
            }),
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn status_to_str(status: crate::services::execution_service::ExecutionStatus) -> &'static str {
    use crate::services::execution_service::ExecutionStatus;
    match status {
        ExecutionStatus::Pending => "pending",
        ExecutionStatus::Running => "running",
        ExecutionStatus::Paused => "paused",
        ExecutionStatus::Completed => "completed",
        ExecutionStatus::Failed => "failed",
        ExecutionStatus::Cancelled => "cancelled",
    }
}

fn status_from_str(s: &str) -> crate::services::execution_service::ExecutionStatus {
    use crate::services::execution_service::ExecutionStatus;
    match s {
        "pending" => ExecutionStatus::Pending,
        "running" => ExecutionStatus::Running,
        "paused" => ExecutionStatus::Paused,
        "completed" => ExecutionStatus::Completed,
        "failed" => ExecutionStatus::Failed,
        "cancelled" => ExecutionStatus::Cancelled,
        _ => ExecutionStatus::Pending,
    }
}

fn row_to_execution(row: &rusqlite::Row) -> rusqlite::Result<Execution> {
    Ok(Execution {
        id: row.get(0)?,
        workflow_id: row.get(1)?,
        status: status_from_str(&row.get::<_, String>(2)?),
        variables: {
            let s: String = row.get(3)?;
            serde_json::from_str(&s).unwrap_or_default()
        },
        error: row.get(4)?,
        started_at: row.get::<_, Option<String>>(5)?.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|t| t.with_timezone(&chrono::Utc))
                .ok()
        }),
        finished_at: row.get::<_, Option<String>>(6)?.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|t| t.with_timezone(&chrono::Utc))
                .ok()
        }),
        stage_results: Vec::new(),
        // 以下字段重启后不恢复（运行时状态）
        output_log: Vec::new(),
        current_stage: None,
        running_agents: Vec::new(),
        pending_pause: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::execution_service::{Execution, ExecutionStatus, StageResult};

    fn tmp_db() -> SqliteExecutionRepository {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_exec.db");
        Box::leak(Box::new(dir));
        SqliteExecutionRepository::new(&path).unwrap()
    }

    fn new_exec(id: &str) -> Execution {
        let mut e = Execution::new("wf-1".into(), serde_json::json!({"k": "v"}));
        e.id = id.into();
        e.status = ExecutionStatus::Running;
        e.started_at = Some(chrono::Utc::now());
        e
    }

    #[test]
    fn insert_and_find_by_id() {
        let repo = tmp_db();
        let exec = new_exec("exec-1");
        repo.insert(&exec).unwrap();

        let found = repo.find_by_id("exec-1").unwrap().unwrap();
        assert_eq!(found.id, "exec-1");
        assert_eq!(found.workflow_id, "wf-1");
        assert_eq!(found.status, ExecutionStatus::Running);
        assert_eq!(found.variables, serde_json::json!({"k": "v"}));
    }

    #[test]
    fn find_all_ordered() {
        let repo = tmp_db();
        let mut e1 = new_exec("exec-1");
        e1.started_at = Some(
            chrono::DateTime::parse_from_rfc3339("2026-04-28T10:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        );
        let mut e2 = new_exec("exec-2");
        e2.started_at = Some(
            chrono::DateTime::parse_from_rfc3339("2026-04-29T10:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        );

        repo.insert(&e1).unwrap();
        repo.insert(&e2).unwrap();

        let all = repo.find_all().unwrap();
        assert_eq!(all.len(), 2);
        // 最新在前
        assert_eq!(all[0].id, "exec-2");
        assert_eq!(all[1].id, "exec-1");
    }

    #[test]
    fn update_status() {
        let repo = tmp_db();
        let mut exec = new_exec("exec-1");
        repo.insert(&exec).unwrap();

        repo.update_status("exec-1", "completed", None, Some("2026-04-29T12:00:00Z"))
            .unwrap();

        let found = repo.find_by_id("exec-1").unwrap().unwrap();
        assert_eq!(found.status, ExecutionStatus::Completed);
        assert!(found.finished_at.is_some());
    }

    #[test]
    fn stage_results() {
        let repo = tmp_db();
        let exec = new_exec("exec-1");
        repo.insert(&exec).unwrap();

        let sr = StageResult {
            stage_name: "plan".into(),
            outputs: vec![serde_json::json!({"ok": true})],
            completed_at: Some(chrono::Utc::now()),
        };
        repo.insert_stage_result("exec-1", &sr).unwrap();

        let found = repo.find_by_id("exec-1").unwrap().unwrap();
        assert_eq!(found.stage_results.len(), 1);
        assert_eq!(found.stage_results[0].stage_name, "plan");
    }
}
