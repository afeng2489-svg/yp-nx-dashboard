//! Project module repository
//!
//! SQLite implementation for project module data access.

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

use crate::models::project_module::{ModuleStatus, ProjectModule};

#[derive(Error, Debug)]
pub enum ModuleRepoError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

fn deserialize_row(
    id: String,
    project_id: String,
    module_name: String,
    status: String,
    summary: String,
    files_changed: String,
    last_execution_id: Option<String>,
    created_at: String,
    updated_at: String,
) -> Result<ProjectModule, ModuleRepoError> {
    let created_at = DateTime::parse_from_rfc3339(&created_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let updated_at = DateTime::parse_from_rfc3339(&updated_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let files_changed: Vec<String> = serde_json::from_str(&files_changed).unwrap_or_default();
    Ok(ProjectModule {
        id,
        project_id,
        module_name,
        status: ModuleStatus::from_str(&status),
        summary,
        files_changed,
        last_execution_id,
        created_at,
        updated_at,
    })
}

#[derive(Debug, Clone)]
pub struct SqliteModuleRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteModuleRepository {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, ModuleRepoError> {
        let conn = Connection::open(db_path)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn in_memory() -> Result<Self, ModuleRepoError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = OFF;")?;
        conn.execute_batch(crate::migrations::PROJECT_SCHEMA)?;
        conn.execute_batch(crate::migrations::PROJECT_MODULE_SCHEMA)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn create(&self, module: &ProjectModule) -> Result<(), ModuleRepoError> {
        let conn = self.conn.lock();
        let files_json = serde_json::to_string(&module.files_changed)
            .map_err(|e| ModuleRepoError::Serialization(e.to_string()))?;
        conn.execute(
            "INSERT INTO project_modules (id, project_id, module_name, status, summary, files_changed, last_execution_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                module.id,
                module.project_id,
                module.module_name,
                module.status.as_str(),
                module.summary,
                files_json,
                module.last_execution_id,
                module.created_at.to_rfc3339(),
                module.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn find_by_project(&self, project_id: &str) -> Result<Vec<ProjectModule>, ModuleRepoError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, module_name, status, summary, files_changed, last_execution_id, created_at, updated_at
             FROM project_modules WHERE project_id = ?1 ORDER BY updated_at ASC",
        )?;
        let rows = stmt.query_map(params![project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        })?;
        let mut modules = Vec::new();
        for row in rows {
            let (id, pid, name, status, summary, files, exec_id, ca, ua) = row?;
            modules.push(deserialize_row(
                id, pid, name, status, summary, files, exec_id, ca, ua,
            )?);
        }
        Ok(modules)
    }

    pub fn find_by_project_and_name(
        &self,
        project_id: &str,
        module_name: &str,
    ) -> Result<Option<ProjectModule>, ModuleRepoError> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT id, project_id, module_name, status, summary, files_changed, last_execution_id, created_at, updated_at
             FROM project_modules WHERE project_id = ?1 AND module_name = ?2",
            params![project_id, module_name],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            },
        );
        match result {
            Ok((id, pid, name, status, summary, files, exec_id, ca, ua)) => Ok(Some(
                deserialize_row(id, pid, name, status, summary, files, exec_id, ca, ua)?,
            )),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn update(&self, module: &ProjectModule) -> Result<(), ModuleRepoError> {
        let conn = self.conn.lock();
        let files_json = serde_json::to_string(&module.files_changed)
            .map_err(|e| ModuleRepoError::Serialization(e.to_string()))?;
        conn.execute(
            "UPDATE project_modules SET status = ?1, summary = ?2, files_changed = ?3, last_execution_id = ?4, updated_at = ?5
             WHERE id = ?6",
            params![
                module.status.as_str(),
                module.summary,
                files_json,
                module.last_execution_id,
                module.updated_at.to_rfc3339(),
                module.id,
            ],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<bool, ModuleRepoError> {
        let conn = self.conn.lock();
        let affected = conn.execute("DELETE FROM project_modules WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub fn delete_by_project(&self, project_id: &str) -> Result<usize, ModuleRepoError> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "DELETE FROM project_modules WHERE project_id = ?1",
            params![project_id],
        )?;
        Ok(affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_find() {
        let repo = SqliteModuleRepository::in_memory().unwrap();
        let module = ProjectModule::new("proj-1".to_string(), "auth".to_string());
        repo.create(&module).unwrap();

        let found = repo.find_by_project("proj-1").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].module_name, "auth");
    }

    #[test]
    fn test_find_by_name() {
        let repo = SqliteModuleRepository::in_memory().unwrap();
        let module = ProjectModule::new("proj-1".to_string(), "auth".to_string());
        repo.create(&module).unwrap();

        let found = repo.find_by_project_and_name("proj-1", "auth").unwrap();
        assert!(found.is_some());

        let missing = repo.find_by_project_and_name("proj-1", "payment").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_update_status() {
        let repo = SqliteModuleRepository::in_memory().unwrap();
        let mut module = ProjectModule::new("proj-1".to_string(), "auth".to_string());
        repo.create(&module).unwrap();

        module.status = ModuleStatus::Completed;
        module.summary = "JWT login done".to_string();
        module.updated_at = Utc::now();
        repo.update(&module).unwrap();

        let found = repo.find_by_project("proj-1").unwrap();
        assert_eq!(found[0].status, ModuleStatus::Completed);
        assert_eq!(found[0].summary, "JWT login done");
    }
}
