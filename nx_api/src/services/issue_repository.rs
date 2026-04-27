//! Issue SQLite 仓储

use chrono::Utc;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::Path;
use std::sync::Mutex;
use uuid::Uuid;

use crate::models::issue::{
    CreateIssueRequest, Issue, IssueFilter, IssuePriority, IssueStatus, UpdateIssueRequest,
};

#[derive(Debug)]
pub enum IssueRepositoryError {
    Sqlite(rusqlite::Error),
    NotFound(String),
}

impl std::fmt::Display for IssueRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueRepositoryError::Sqlite(e) => write!(f, "SQLite error: {}", e),
            IssueRepositoryError::NotFound(id) => write!(f, "Issue not found: {}", id),
        }
    }
}

impl From<rusqlite::Error> for IssueRepositoryError {
    fn from(e: rusqlite::Error) -> Self {
        IssueRepositoryError::Sqlite(e)
    }
}

pub struct SqliteIssueRepository {
    conn: Mutex<Connection>,
}

impl SqliteIssueRepository {
    pub fn new(db_path: &Path) -> Result<Self, IssueRepositoryError> {
        let conn = Connection::open(db_path)?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS issues (
                id          TEXT PRIMARY KEY,
                title       TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                status      TEXT NOT NULL DEFAULT 'discovered',
                priority    TEXT NOT NULL DEFAULT 'medium',
                perspectives TEXT NOT NULL DEFAULT '[]',
                solution    TEXT,
                depends_on  TEXT NOT NULL DEFAULT '[]',
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn row_to_issue(row: &rusqlite::Row<'_>) -> SqliteResult<Issue> {
        let perspectives_str: String = row.get(5)?;
        let depends_on_str: String = row.get(7)?;
        let created_at_str: String = row.get(8)?;
        let updated_at_str: String = row.get(9)?;

        Ok(Issue {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            status: IssueStatus::from_str(&row.get::<_, String>(3)?),
            priority: IssuePriority::from_str(&row.get::<_, String>(4)?),
            perspectives: serde_json::from_str(&perspectives_str).unwrap_or_default(),
            solution: row.get(6)?,
            depends_on: serde_json::from_str(&depends_on_str).unwrap_or_default(),
            created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
            updated_at: updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
        })
    }

    pub fn find_all(&self, filter: &IssueFilter) -> Result<Vec<Issue>, IssueRepositoryError> {
        let conn = self.conn.lock().unwrap();
        let mut conditions = vec!["1=1"];
        let mut status_val = String::new();
        let mut priority_val = String::new();

        if filter.status.is_some() {
            conditions.push("status = ?1");
            status_val = filter.status.clone().unwrap();
        }
        if filter.priority.is_some() {
            conditions.push("priority = ?2");
            priority_val = filter.priority.clone().unwrap();
        }

        let sql = format!(
            "SELECT id,title,description,status,priority,perspectives,solution,depends_on,created_at,updated_at FROM issues WHERE {} ORDER BY created_at DESC",
            conditions.join(" AND ")
        );
        let mut stmt = conn.prepare(&sql)?;
        let issues = stmt
            .query_map(params![status_val, priority_val], Self::row_to_issue)?
            .collect::<SqliteResult<Vec<_>>>()?;
        Ok(issues)
    }

    pub fn find_by_id(&self, id: &str) -> Result<Issue, IssueRepositoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id,title,description,status,priority,perspectives,solution,depends_on,created_at,updated_at FROM issues WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], Self::row_to_issue)?;
        rows.next()
            .ok_or_else(|| IssueRepositoryError::NotFound(id.to_string()))?
            .map_err(IssueRepositoryError::from)
    }

    pub fn create(&self, req: CreateIssueRequest) -> Result<Issue, IssueRepositoryError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let perspectives =
            serde_json::to_string(&req.perspectives).unwrap_or_else(|_| "[]".to_string());
        let depends_on =
            serde_json::to_string(&req.depends_on).unwrap_or_else(|_| "[]".to_string());

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO issues (id,title,description,status,priority,perspectives,solution,depends_on,created_at,updated_at)
             VALUES (?1,?2,?3,'discovered',?4,?5,NULL,?6,?7,?7)",
            params![id, req.title, req.description, req.priority.as_str(), perspectives, depends_on, now],
        )?;

        drop(conn);
        self.find_by_id(&id)
    }

    pub fn update(&self, id: &str, req: UpdateIssueRequest) -> Result<Issue, IssueRepositoryError> {
        // Verify exists
        self.find_by_id(id)?;

        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();

        if let Some(title) = &req.title {
            conn.execute(
                "UPDATE issues SET title=?1,updated_at=?2 WHERE id=?3",
                params![title, now, id],
            )?;
        }
        if let Some(desc) = &req.description {
            conn.execute(
                "UPDATE issues SET description=?1,updated_at=?2 WHERE id=?3",
                params![desc, now, id],
            )?;
        }
        if let Some(status) = &req.status {
            conn.execute(
                "UPDATE issues SET status=?1,updated_at=?2 WHERE id=?3",
                params![status.as_str(), now, id],
            )?;
        }
        if let Some(priority) = &req.priority {
            conn.execute(
                "UPDATE issues SET priority=?1,updated_at=?2 WHERE id=?3",
                params![priority.as_str(), now, id],
            )?;
        }
        if let Some(solution) = &req.solution {
            conn.execute(
                "UPDATE issues SET solution=?1,updated_at=?2 WHERE id=?3",
                params![solution, now, id],
            )?;
        }
        if let Some(perspectives) = &req.perspectives {
            let json = serde_json::to_string(perspectives).unwrap_or_else(|_| "[]".to_string());
            conn.execute(
                "UPDATE issues SET perspectives=?1,updated_at=?2 WHERE id=?3",
                params![json, now, id],
            )?;
        }
        if let Some(depends_on) = &req.depends_on {
            let json = serde_json::to_string(depends_on).unwrap_or_else(|_| "[]".to_string());
            conn.execute(
                "UPDATE issues SET depends_on=?1,updated_at=?2 WHERE id=?3",
                params![json, now, id],
            )?;
        }

        drop(conn);
        self.find_by_id(id)
    }

    pub fn delete(&self, id: &str) -> Result<(), IssueRepositoryError> {
        self.find_by_id(id)?;
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM issues WHERE id=?1", params![id])?;
        Ok(())
    }
}
