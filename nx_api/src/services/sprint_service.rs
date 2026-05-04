use anyhow::Context;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::Mutex;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SprintCard {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub estimated_hours: i64,
    pub data_json: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SprintEvent {
    pub id: String,
    pub sprint_id: String,
    pub event_type: String,
    pub detail: Option<String>,
    pub created_at: String,
}

pub struct SprintService {
    conn: Arc<Mutex<Connection>>,
}

impl SprintService {
    pub fn new(db_path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path).context("open sprint db")?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    pub fn list(&self) -> anyhow::Result<Vec<SprintCard>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, status, priority, estimated_hours, data_json, updated_at FROM sprint_cards ORDER BY updated_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SprintCard {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                priority: row.get(3)?,
                estimated_hours: row.get(4)?,
                data_json: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().context("list sprint_cards")
    }

    pub fn upsert(&self, card: &SprintCard) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO sprint_cards (id, title, status, priority, estimated_hours, data_json, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
               title=excluded.title, status=excluded.status, priority=excluded.priority,
               estimated_hours=excluded.estimated_hours, data_json=excluded.data_json,
               updated_at=excluded.updated_at",
            params![card.id, card.title, card.status, card.priority, card.estimated_hours, card.data_json, card.updated_at],
        )?;
        Ok(())
    }

    pub fn update_status(&self, id: &str, status: &str) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE sprint_cards SET status=?1, updated_at=?2 WHERE id=?3",
            params![status, now, id],
        )?;
        Ok(())
    }

    pub fn record_event(&self, event: &SprintEvent) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO sprint_events (id, sprint_id, event_type, detail, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![event.id, event.sprint_id, event.event_type, event.detail, event.created_at],
        )?;
        Ok(())
    }

    pub fn events_for(&self, sprint_id: &str) -> anyhow::Result<Vec<SprintEvent>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, sprint_id, event_type, detail, created_at FROM sprint_events WHERE sprint_id=?1 ORDER BY created_at ASC"
        )?;
        let rows = stmt.query_map(params![sprint_id], |row| {
            Ok(SprintEvent {
                id: row.get(0)?,
                sprint_id: row.get(1)?,
                event_type: row.get(2)?,
                detail: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().context("list sprint_events")
    }

    pub fn seed_from_progress_json(&self, json_path: &str) -> anyhow::Result<()> {
        let content = std::fs::read_to_string(json_path)
            .with_context(|| format!("read {json_path}"))?;
        let doc: Value = serde_json::from_str(&content)?;
        let tasks = doc["tasks"].as_array().context("tasks not array")?;
        let now = chrono::Utc::now().to_rfc3339();
        for task in tasks {
            let id = task["id"].as_str().unwrap_or_default().to_string();
            if id.is_empty() { continue; }
            let exists: bool = {
                let conn = self.conn.lock();
                conn.query_row(
                    "SELECT COUNT(*) FROM sprint_cards WHERE id=?1",
                    params![id],
                    |r| r.get::<_, i64>(0),
                ).unwrap_or(0) > 0
            };
            if exists { continue; }
            let card = SprintCard {
                id,
                title: task["name"].as_str().unwrap_or("").to_string(),
                status: task["status"].as_str().unwrap_or("pending").to_string(),
                priority: task["priority"].as_str().unwrap_or("P2").to_string(),
                estimated_hours: task["estimated_hours"].as_i64().unwrap_or(0),
                data_json: serde_json::to_string(&task).unwrap_or_default(),
                updated_at: now.clone(),
            };
            self.upsert(&card)?;
        }
        Ok(())
    }
}
