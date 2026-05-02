//! Project repository
//!
//! SQLite implementation for project data access.

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

use crate::models::project::{Project, ProjectStatus};

/// Repository error
#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Project not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Project repository trait
pub trait ProjectRepository: Send + Sync {
    fn create(&self, project: &Project) -> Result<(), RepositoryError>;
    fn find_by_id(&self, id: &str) -> Result<Option<Project>, RepositoryError>;
    fn find_all(&self) -> Result<Vec<Project>, RepositoryError>;
    fn find_by_team(&self, team_id: &str) -> Result<Vec<Project>, RepositoryError>;
    fn update(&self, project: &Project) -> Result<(), RepositoryError>;
    fn delete(&self, id: &str) -> Result<bool, RepositoryError>;
}

/// SQLite project repository
#[derive(Debug, Clone)]
pub struct SqliteProjectRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteProjectRepository {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, RepositoryError> {
        let conn = Connection::open(db_path)?;

        // Migration: add workspace_id column if it doesn't exist (for existing databases)
        let _: Result<usize, _> =
            conn.execute("ALTER TABLE projects ADD COLUMN workspace_id TEXT", []);
        // Ignore error - column might already exist

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    #[allow(dead_code)]
    pub fn in_memory() -> Result<Self, RepositoryError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(crate::migrations::PROJECT_SCHEMA)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn deserialize_row(
        id: String,
        name: String,
        description: String,
        team_id: String,
        workspace_id: Option<String>,
        workflow_id: Option<String>,
        variables: String,
        status: String,
        created_at: String,
        updated_at: String,
    ) -> Result<Project, RepositoryError> {
        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = DateTime::parse_from_rfc3339(&updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let variables: HashMap<String, String> =
            serde_json::from_str(&variables).unwrap_or_default();

        Ok(Project {
            id,
            name,
            description,
            team_id,
            workspace_id,
            workflow_id,
            variables,
            status: ProjectStatus::from_str(&status),
            created_at,
            updated_at,
        })
    }
}

impl ProjectRepository for SqliteProjectRepository {
    fn create(&self, project: &Project) -> Result<(), RepositoryError> {
        let conn = self.conn.lock();
        let variables_json = serde_json::to_string(&project.variables)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;

        conn.execute(
            "INSERT INTO projects (id, name, description, team_id, workspace_id, workflow_id, variables, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                project.id,
                project.name,
                project.description,
                project.team_id,
                project.workspace_id,
                project.workflow_id,
                variables_json,
                project.status.as_str(),
                project.created_at.to_rfc3339(),
                project.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn find_by_id(&self, id: &str) -> Result<Option<Project>, RepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, team_id, workspace_id, workflow_id, variables, status, created_at, updated_at
             FROM projects WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
            ))
        });

        match result {
            Ok((
                id,
                name,
                description,
                team_id,
                workspace_id,
                workflow_id,
                variables,
                status,
                created_at,
                updated_at,
            )) => Ok(Some(Self::deserialize_row(
                id,
                name,
                description,
                team_id,
                workspace_id,
                workflow_id,
                variables,
                status,
                created_at,
                updated_at,
            )?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn find_all(&self) -> Result<Vec<Project>, RepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, team_id, workspace_id, workflow_id, variables, status, created_at, updated_at
             FROM projects ORDER BY updated_at DESC",
        )?;

        let rows = stmt.query_map([], |row: &rusqlite::Row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
            ))
        })?;

        let mut projects = Vec::new();
        for row in rows {
            let (
                id,
                name,
                description,
                team_id,
                workspace_id,
                workflow_id,
                variables,
                status,
                created_at,
                updated_at,
            ) = row?;
            projects.push(Self::deserialize_row(
                id,
                name,
                description,
                team_id,
                workspace_id,
                workflow_id,
                variables,
                status,
                created_at,
                updated_at,
            )?);
        }
        Ok(projects)
    }

    fn find_by_team(&self, team_id: &str) -> Result<Vec<Project>, RepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, team_id, workspace_id, workflow_id, variables, status, created_at, updated_at
             FROM projects WHERE team_id = ?1 ORDER BY updated_at DESC",
        )?;

        let rows = stmt.query_map(params![team_id], |row: &rusqlite::Row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
            ))
        })?;

        let mut projects = Vec::new();
        for row in rows {
            let (
                id,
                name,
                description,
                team_id,
                workspace_id,
                workflow_id,
                variables,
                status,
                created_at,
                updated_at,
            ) = row?;
            projects.push(Self::deserialize_row(
                id,
                name,
                description,
                team_id,
                workspace_id,
                workflow_id,
                variables,
                status,
                created_at,
                updated_at,
            )?);
        }
        Ok(projects)
    }

    fn update(&self, project: &Project) -> Result<(), RepositoryError> {
        let conn = self.conn.lock();
        let variables_json = serde_json::to_string(&project.variables)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;

        let affected = conn.execute(
            "UPDATE projects SET name = ?1, description = ?2, team_id = ?3, workspace_id = ?4, workflow_id = ?5, variables = ?6, status = ?7, updated_at = ?8
             WHERE id = ?9",
            params![
                project.name,
                project.description,
                project.team_id,
                project.workspace_id,
                project.workflow_id,
                variables_json,
                project.status.as_str(),
                project.updated_at.to_rfc3339(),
                project.id,
            ],
        )?;
        if affected == 0 {
            return Err(RepositoryError::NotFound(project.id.clone()));
        }
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<bool, RepositoryError> {
        let conn = self.conn.lock();
        let affected = conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_find() {
        let repo = SqliteProjectRepository::in_memory().unwrap();
        let project = Project::new(
            "Test Project".to_string(),
            "A test project".to_string(),
            "team-1".to_string(),
            None,
            None,
        );

        repo.create(&project).unwrap();

        let found = repo.find_by_id(&project.id).unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, project.id);
        assert_eq!(found.name, "Test Project");
    }

    #[test]
    fn test_find_by_team() {
        let repo = SqliteProjectRepository::in_memory().unwrap();

        let project1 = Project::new(
            "Project 1".to_string(),
            "".to_string(),
            "team-1".to_string(),
            None,
            None,
        );
        let project2 = Project::new(
            "Project 2".to_string(),
            "".to_string(),
            "team-1".to_string(),
            None,
            None,
        );
        let project3 = Project::new(
            "Project 3".to_string(),
            "".to_string(),
            "team-2".to_string(),
            None,
            None,
        );

        repo.create(&project1).unwrap();
        repo.create(&project2).unwrap();
        repo.create(&project3).unwrap();

        let team_projects = repo.find_by_team("team-1").unwrap();
        assert_eq!(team_projects.len(), 2);
    }
}
