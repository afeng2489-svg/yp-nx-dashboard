use anyhow::Context;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedMessage {
    pub id: String,
    pub session_id: String,
    pub execution_id: Option<String>,
    pub role: String,
    pub content_json: String,
    pub pending: bool,
    pub responded: bool,
    pub created_at: String,
}

pub struct SessionMessageStore {
    conn: Arc<Mutex<Connection>>,
}

impl SessionMessageStore {
    pub fn new(db_path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path).context("open session_messages db")?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    pub fn insert(&self, msg: &PersistedMessage) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR IGNORE INTO session_messages
             (id, session_id, execution_id, role, content_json, pending, responded, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                msg.id, msg.session_id, msg.execution_id, msg.role,
                msg.content_json, msg.pending as i64, msg.responded as i64, msg.created_at
            ],
        )?;
        Ok(())
    }

    pub fn mark_responded(&self, msg_id: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE session_messages SET responded=1, pending=0 WHERE id=?1",
            params![msg_id],
        )?;
        Ok(())
    }

    pub fn list_for_session(&self, session_id: &str) -> anyhow::Result<Vec<PersistedMessage>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, execution_id, role, content_json, pending, responded, created_at
             FROM session_messages WHERE session_id=?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![session_id], |r| {
            Ok(PersistedMessage {
                id: r.get(0)?,
                session_id: r.get(1)?,
                execution_id: r.get(2)?,
                role: r.get(3)?,
                content_json: r.get(4)?,
                pending: r.get::<_, i64>(5)? != 0,
                responded: r.get::<_, i64>(6)? != 0,
                created_at: r.get(7)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().context("list session_messages")
    }
}
