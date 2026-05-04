use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLog {
    pub id: String,
    pub trace_id: String,
    pub execution_id: String,
    pub stage_name: Option<String>,
    pub model: Option<String>,
    pub attempt: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub duration_ms: i64,
    pub status: String, // "success" | "failed" | "retrying" | "escalated"
    pub error: Option<String>,
    pub timestamp: String,
}

pub struct ExecutionLogService {
    conn: Arc<Mutex<Connection>>,
}

impl ExecutionLogService {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn append(&self, log: &ExecutionLog) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "INSERT INTO execution_logs (id,trace_id,execution_id,stage_name,model,attempt,
             prompt_tokens,completion_tokens,duration_ms,status,error,timestamp)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                log.id,
                log.trace_id,
                log.execution_id,
                log.stage_name,
                log.model,
                log.attempt,
                log.prompt_tokens,
                log.completion_tokens,
                log.duration_ms,
                log.status,
                log.error,
                log.timestamp,
            ],
        );
    }

    pub fn list_by_execution(&self, execution_id: &str) -> Vec<ExecutionLog> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id,trace_id,execution_id,stage_name,model,attempt,
                 prompt_tokens,completion_tokens,duration_ms,status,error,timestamp
                 FROM execution_logs WHERE execution_id=?1 ORDER BY timestamp ASC",
            )
            .unwrap();
        stmt.query_map(params![execution_id], |row| {
            Ok(ExecutionLog {
                id: row.get(0)?,
                trace_id: row.get(1)?,
                execution_id: row.get(2)?,
                stage_name: row.get(3)?,
                model: row.get(4)?,
                attempt: row.get(5)?,
                prompt_tokens: row.get(6)?,
                completion_tokens: row.get(7)?,
                duration_ms: row.get(8)?,
                status: row.get(9)?,
                error: row.get(10)?,
                timestamp: row.get(11)?,
            })
        })
        .unwrap()
        .flatten()
        .collect()
    }

    pub fn new_log(
        execution_id: &str,
        trace_id: &str,
        stage_name: Option<&str>,
        model: Option<&str>,
        attempt: usize,
        status: &str,
        error: Option<&str>,
        duration_ms: i64,
    ) -> ExecutionLog {
        ExecutionLog {
            id: Uuid::new_v4().to_string(),
            trace_id: trace_id.to_string(),
            execution_id: execution_id.to_string(),
            stage_name: stage_name.map(str::to_string),
            model: model.map(str::to_string),
            attempt: attempt as i64,
            prompt_tokens: 0,
            completion_tokens: 0,
            duration_ms,
            status: status.to_string(),
            error: error.map(str::to_string),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}
