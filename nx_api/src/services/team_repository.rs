//! Team repository
//!
//! SQLite implementation of team data access layer.

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

use crate::models::team::{
    ModelConfig, RoleSkill, SkillPriority, Team, TeamMessage, TeamRole, TelegramBotConfig,
    MessageType,
};

/// Repository error
#[derive(Error, Debug)]
pub enum TeamRepositoryError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Team not found: {0}")]
    TeamNotFound(String),

    #[error("Role not found: {0}")]
    RoleNotFound(String),

    #[error("Message not found: {0}")]
    MessageNotFound(String),

    #[error("Telegram config not found for role: {0}")]
    TelegramConfigNotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Team repository trait
pub trait TeamRepository: Send + Sync {
    // Team CRUD
    fn create_team(&self, team: &Team) -> Result<(), TeamRepositoryError>;
    fn find_team_by_id(&self, id: &str) -> Result<Option<Team>, TeamRepositoryError>;
    fn find_all_teams(&self) -> Result<Vec<Team>, TeamRepositoryError>;
    fn update_team(&self, team: &Team) -> Result<(), TeamRepositoryError>;
    fn delete_team(&self, id: &str) -> Result<bool, TeamRepositoryError>;

    // Role CRUD
    fn create_role(&self, role: &TeamRole) -> Result<(), TeamRepositoryError>;
    fn find_role_by_id(&self, id: &str) -> Result<Option<TeamRole>, TeamRepositoryError>;
    fn find_roles_by_team(&self, team_id: &str) -> Result<Vec<TeamRole>, TeamRepositoryError>;
    fn find_all_roles(&self) -> Result<Vec<TeamRole>, TeamRepositoryError>;
    fn update_role(&self, role: &TeamRole) -> Result<(), TeamRepositoryError>;
    fn add_role_to_team(&self, role_id: &str, team_id: &str) -> Result<(), TeamRepositoryError>;
    fn remove_role_from_team(&self, role_id: &str, team_id: &str) -> Result<bool, TeamRepositoryError>;
    fn delete_role(&self, id: &str) -> Result<bool, TeamRepositoryError>;

    // Skill assignment
    fn assign_skill(&self, role_id: &str, skill_id: &str, priority: SkillPriority) -> Result<(), TeamRepositoryError>;
    fn remove_skill(&self, role_id: &str, skill_id: &str) -> Result<bool, TeamRepositoryError>;
    fn find_skills_by_role(&self, role_id: &str) -> Result<Vec<RoleSkill>, TeamRepositoryError>;

    // Messages
    fn create_message(&self, message: &TeamMessage) -> Result<(), TeamRepositoryError>;
    fn find_messages_by_team(&self, team_id: &str, limit: Option<usize>) -> Result<Vec<TeamMessage>, TeamRepositoryError>;
    fn find_messages_by_role(&self, role_id: &str, limit: Option<usize>) -> Result<Vec<TeamMessage>, TeamRepositoryError>;

    // Telegram config
    fn upsert_telegram_config(&self, config: &TelegramBotConfig) -> Result<(), TeamRepositoryError>;
    fn find_telegram_config_by_role(&self, role_id: &str) -> Result<Option<TelegramBotConfig>, TeamRepositoryError>;
    fn delete_telegram_config(&self, role_id: &str) -> Result<bool, TeamRepositoryError>;
}

/// SQLite team repository
#[derive(Debug, Clone)]
pub struct SqliteTeamRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteTeamRepository {
    /// Create new SQLite repository
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, TeamRepositoryError> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS teams (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS team_roles (
                id TEXT PRIMARY KEY,
                team_id TEXT,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                model_config TEXT NOT NULL,
                system_prompt TEXT NOT NULL,
                trigger_keywords TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE SET NULL
            );

            CREATE INDEX IF NOT EXISTS idx_team_roles_team_id ON team_roles(team_id);

            -- Junction table for many-to-many relationship between teams and roles
            CREATE TABLE IF NOT EXISTS team_role_members (
                team_id TEXT NOT NULL,
                role_id TEXT NOT NULL,
                added_at TEXT NOT NULL,
                PRIMARY KEY (team_id, role_id),
                FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
                FOREIGN KEY (role_id) REFERENCES team_roles(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS team_role_skills (
                role_id TEXT NOT NULL,
                skill_id TEXT NOT NULL,
                priority TEXT NOT NULL DEFAULT 'medium',
                PRIMARY KEY (role_id, skill_id),
                FOREIGN KEY (role_id) REFERENCES team_roles(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS team_messages (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL,
                role_id TEXT,
                content TEXT NOT NULL,
                message_type TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
                FOREIGN KEY (role_id) REFERENCES team_roles(id) ON DELETE SET NULL
            );

            CREATE INDEX IF NOT EXISTS idx_team_messages_team_id ON team_messages(team_id);
            CREATE INDEX IF NOT EXISTS idx_team_messages_role_id ON team_messages(role_id);
            CREATE INDEX IF NOT EXISTS idx_team_messages_created_at ON team_messages(created_at);

            CREATE TABLE IF NOT EXISTS telegram_bot_configs (
                id TEXT PRIMARY KEY,
                role_id TEXT NOT NULL UNIQUE,
                bot_token TEXT NOT NULL,
                chat_id TEXT,
                enabled INTEGER NOT NULL DEFAULT 0,
                notifications_enabled INTEGER NOT NULL DEFAULT 1,
                conversation_enabled INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (role_id) REFERENCES team_roles(id) ON DELETE CASCADE
            );",
        )?;

        // Migration: Copy existing team_id relationships to junction table
        // This handles the case where roles had a direct team_id reference
        let conn = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        if let Err(e) = conn.migrate_existing_roles() {
            eprintln!("Warning: failed to migrate existing roles: {}", e);
        }
        Ok(conn)
    }

    /// Create in-memory repository (for testing)
    #[allow(dead_code)]
    pub fn in_memory() -> Result<Self, TeamRepositoryError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS teams (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS team_roles (
                id TEXT PRIMARY KEY,
                team_id TEXT,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                model_config TEXT NOT NULL,
                system_prompt TEXT NOT NULL,
                trigger_keywords TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS team_role_members (
                team_id TEXT NOT NULL,
                role_id TEXT NOT NULL,
                added_at TEXT NOT NULL,
                PRIMARY KEY (team_id, role_id)
            );

            CREATE TABLE IF NOT EXISTS team_role_skills (
                role_id TEXT NOT NULL,
                skill_id TEXT NOT NULL,
                priority TEXT NOT NULL DEFAULT 'medium',
                PRIMARY KEY (role_id, skill_id)
            );

            CREATE TABLE IF NOT EXISTS team_messages (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL,
                role_id TEXT,
                content TEXT NOT NULL,
                message_type TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS telegram_bot_configs (
                id TEXT PRIMARY KEY,
                role_id TEXT NOT NULL UNIQUE,
                bot_token TEXT NOT NULL,
                chat_id TEXT,
                enabled INTEGER NOT NULL DEFAULT 0,
                notifications_enabled INTEGER NOT NULL DEFAULT 1,
                conversation_enabled INTEGER NOT NULL DEFAULT 0
            );",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn deserialize_model_config(json: &str) -> ModelConfig {
        serde_json::from_str(json).unwrap_or_else(|_| ModelConfig::default())
    }

    fn serialize_model_config(config: &ModelConfig) -> String {
        serde_json::to_string(config).unwrap_or_else(|_| "{}".to_string())
    }

    fn deserialize_team(
        id: String,
        name: String,
        description: String,
        created_at: String,
        updated_at: String,
    ) -> Result<Team, TeamRepositoryError> {
        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let updated_at = DateTime::parse_from_rfc3339(&updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Team {
            id,
            name,
            description,
            created_at,
            updated_at,
        })
    }

    fn deserialize_role(
        id: String,
        team_id: Option<String>,
        name: String,
        description: String,
        model_config: String,
        system_prompt: String,
        trigger_keywords: String,
        created_at: String,
        updated_at: String,
    ) -> Result<TeamRole, TeamRepositoryError> {
        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let updated_at = DateTime::parse_from_rfc3339(&updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let model_config = Self::deserialize_model_config(&model_config);
        let trigger_keywords: Vec<String> = serde_json::from_str(&trigger_keywords).unwrap_or_default();

        Ok(TeamRole {
            id,
            team_id,
            name,
            description,
            model_config,
            system_prompt,
            trigger_keywords,
            created_at,
            updated_at,
        })
    }

    fn deserialize_message(
        id: String,
        team_id: String,
        role_id: Option<String>,
        content: String,
        message_type: String,
        metadata: String,
        created_at: String,
    ) -> Result<TeamMessage, TeamRepositoryError> {
        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let message_type = MessageType::from_str(&message_type);
        let metadata: HashMap<String, String> =
            serde_json::from_str(&metadata).unwrap_or_default();

        Ok(TeamMessage {
            id,
            team_id,
            role_id,
            content,
            message_type,
            metadata,
            created_at,
        })
    }

    ///// Migration: Copy existing team_id references to junction table
    fn migrate_existing_roles(&self) -> Result<(), TeamRepositoryError> {
        let conn = self.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();

        // Copy roles with team_id to junction table
        conn.execute(
            "INSERT OR IGNORE INTO team_role_members (team_id, role_id, added_at)
             SELECT team_id, id, ?1 FROM team_roles WHERE team_id IS NOT NULL",
            params![now],
        )?;

        // Set team_id to NULL (roles are now global, accessed via junction table)
        conn.execute(
            "UPDATE team_roles SET team_id = NULL WHERE team_id IS NOT NULL",
            [],
        )?;

        // Migration: Add trigger_keywords column if not exists (SQLite doesn't support ADD COLUMN IF NOT EXISTS)
        // We check if the column exists by trying to select it
        let column_exists: Result<i32, _> = conn.query_row(
            "SELECT 1 FROM pragma_table_info('team_roles') WHERE name = 'trigger_keywords'",
            [],
            |_| Ok(1),
        );
        if column_exists.is_err() {
            tracing::info!("[Migration] Adding trigger_keywords column to team_roles");
            conn.execute(
                "ALTER TABLE team_roles ADD COLUMN trigger_keywords TEXT NOT NULL DEFAULT '[]'",
                [],
            )?;
        }

        Ok(())
    }
}

impl TeamRepository for SqliteTeamRepository {
    // Team CRUD
    fn create_team(&self, team: &Team) -> Result<(), TeamRepositoryError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO teams (id, name, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                team.id,
                team.name,
                team.description,
                team.created_at.to_rfc3339(),
                team.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn find_team_by_id(&self, id: &str) -> Result<Option<Team>, TeamRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_at, updated_at FROM teams WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        });

        match result {
            Ok((id, name, description, created_at, updated_at)) => {
                Ok(Some(Self::deserialize_team(
                    id, name, description, created_at, updated_at,
                )?))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn find_all_teams(&self) -> Result<Vec<Team>, TeamRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt =
            conn.prepare("SELECT id, name, description, created_at, updated_at FROM teams ORDER BY created_at DESC")?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;

        let mut teams = Vec::new();
        for row in rows {
            let (id, name, description, created_at, updated_at) = row?;
            teams.push(Self::deserialize_team(
                id, name, description, created_at, updated_at,
            )?);
        }
        Ok(teams)
    }

    fn update_team(&self, team: &Team) -> Result<(), TeamRepositoryError> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "UPDATE teams SET name = ?1, description = ?2, updated_at = ?3 WHERE id = ?4",
            params![
                team.name,
                team.description,
                team.updated_at.to_rfc3339(),
                team.id,
            ],
        )?;
        if affected == 0 {
            return Err(TeamRepositoryError::TeamNotFound(team.id.clone()));
        }
        Ok(())
    }

    fn delete_team(&self, id: &str) -> Result<bool, TeamRepositoryError> {
        let conn = self.conn.lock();
        let affected = conn.execute("DELETE FROM teams WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    // Role CRUD
    fn create_role(&self, role: &TeamRole) -> Result<(), TeamRepositoryError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO team_roles (id, team_id, name, description, model_config, system_prompt, trigger_keywords, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                role.id,
                role.team_id,
                role.name,
                role.description,
                Self::serialize_model_config(&role.model_config),
                role.system_prompt,
                serde_json::to_string(&role.trigger_keywords).unwrap_or_else(|_| "[]".to_string()),
                role.created_at.to_rfc3339(),
                role.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn find_role_by_id(&self, id: &str) -> Result<Option<TeamRole>, TeamRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, team_id, name, description, model_config, system_prompt, trigger_keywords, created_at, updated_at
             FROM team_roles WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        });

        match result {
            Ok((id, team_id, name, description, model_config, system_prompt, trigger_keywords, created_at, updated_at)) => {
                Ok(Some(Self::deserialize_role(
                    id, team_id, name, description, model_config, system_prompt, trigger_keywords, created_at, updated_at,
                )?))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn find_roles_by_team(&self, team_id: &str) -> Result<Vec<TeamRole>, TeamRepositoryError> {
        let conn = self.conn.lock();
        // Union: roles in junction table OR roles with direct team_id field (legacy)
        let mut stmt = conn.prepare(
            "SELECT DISTINCT r.id, r.team_id, r.name, r.description, r.model_config, r.system_prompt, r.trigger_keywords, r.created_at, r.updated_at
             FROM team_roles r
             WHERE r.id IN (
                 SELECT role_id FROM team_role_members WHERE team_id = ?1
             ) OR r.team_id = ?1
             ORDER BY r.created_at ASC",
        )?;

        let rows = stmt.query_map(params![team_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        })?;

        let mut roles = Vec::new();
        for row in rows {
            let (id, team_id, name, description, model_config, system_prompt, trigger_keywords, created_at, updated_at) = row?;
            roles.push(Self::deserialize_role(
                id, team_id, name, description, model_config, system_prompt, trigger_keywords, created_at, updated_at,
            )?);
        }
        Ok(roles)
    }

    fn find_all_roles(&self) -> Result<Vec<TeamRole>, TeamRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, team_id, name, description, model_config, system_prompt, trigger_keywords, created_at, updated_at
             FROM team_roles ORDER BY created_at ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        })?;

        let mut roles = Vec::new();
        for row in rows {
            let (id, team_id, name, description, model_config, system_prompt, trigger_keywords, created_at, updated_at) = row?;
            roles.push(Self::deserialize_role(
                id, team_id, name, description, model_config, system_prompt, trigger_keywords, created_at, updated_at,
            )?);
        }
        Ok(roles)
    }

    fn add_role_to_team(&self, role_id: &str, team_id: &str) -> Result<(), TeamRepositoryError> {
        let conn = self.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR IGNORE INTO team_role_members (team_id, role_id, added_at) VALUES (?1, ?2, ?3)",
            params![team_id, role_id, now],
        )?;
        Ok(())
    }

    fn remove_role_from_team(&self, role_id: &str, team_id: &str) -> Result<bool, TeamRepositoryError> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "DELETE FROM team_role_members WHERE team_id = ?1 AND role_id = ?2",
            params![team_id, role_id],
        )?;
        Ok(affected > 0)
    }

    fn update_role(&self, role: &TeamRole) -> Result<(), TeamRepositoryError> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "UPDATE team_roles SET name = ?1, description = ?2, model_config = ?3, system_prompt = ?4, trigger_keywords = ?5, updated_at = ?6
             WHERE id = ?7",
            params![
                role.name,
                role.description,
                Self::serialize_model_config(&role.model_config),
                role.system_prompt,
                serde_json::to_string(&role.trigger_keywords).unwrap_or_else(|_| "[]".to_string()),
                role.updated_at.to_rfc3339(),
                role.id,
            ],
        )?;
        if affected == 0 {
            return Err(TeamRepositoryError::RoleNotFound(role.id.clone()));
        }
        Ok(())
    }

    fn delete_role(&self, id: &str) -> Result<bool, TeamRepositoryError> {
        let conn = self.conn.lock();
        let affected = conn.execute("DELETE FROM team_roles WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    // Skill assignment
    fn assign_skill(&self, role_id: &str, skill_id: &str, priority: SkillPriority) -> Result<(), TeamRepositoryError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO team_role_skills (role_id, skill_id, priority)
             VALUES (?1, ?2, ?3)",
            params![role_id, skill_id, priority.as_str()],
        )?;
        Ok(())
    }

    fn remove_skill(&self, role_id: &str, skill_id: &str) -> Result<bool, TeamRepositoryError> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "DELETE FROM team_role_skills WHERE role_id = ?1 AND skill_id = ?2",
            params![role_id, skill_id],
        )?;
        Ok(affected > 0)
    }

    fn find_skills_by_role(&self, role_id: &str) -> Result<Vec<RoleSkill>, TeamRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT role_id, skill_id, priority FROM team_role_skills WHERE role_id = ?1",
        )?;

        let rows = stmt.query_map(params![role_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        let mut skills = Vec::new();
        for row in rows {
            let (role_id, skill_id, priority) = row?;
            skills.push(RoleSkill {
                role_id,
                skill_id,
                priority: SkillPriority::from_str(&priority),
            });
        }
        Ok(skills)
    }

    // Messages
    fn create_message(&self, message: &TeamMessage) -> Result<(), TeamRepositoryError> {
        let conn = self.conn.lock();
        let metadata_json = serde_json::to_string(&message.metadata)
            .unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "INSERT INTO team_messages (id, team_id, role_id, content, message_type, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                message.id,
                message.team_id,
                message.role_id,
                message.content,
                message.message_type.as_str(),
                metadata_json,
                message.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn find_messages_by_team(
        &self,
        team_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<TeamMessage>, TeamRepositoryError> {
        let conn = self.conn.lock();
        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();
        let query = format!(
            "SELECT id, team_id, role_id, content, message_type, metadata, created_at
             FROM team_messages WHERE team_id = ?1 ORDER BY created_at ASC{}",
            limit_clause
        );
        let mut stmt = conn.prepare(&query)?;

        let rows = stmt.query_map(params![team_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;

        let mut messages = Vec::new();
        for row in rows {
            let (id, team_id, role_id, content, message_type, metadata, created_at) = row?;
            messages.push(Self::deserialize_message(
                id, team_id, role_id, content, message_type, metadata, created_at,
            )?);
        }
        Ok(messages)
    }

    fn find_messages_by_role(
        &self,
        role_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<TeamMessage>, TeamRepositoryError> {
        let conn = self.conn.lock();
        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();
        let query = format!(
            "SELECT id, team_id, role_id, content, message_type, metadata, created_at
             FROM team_messages WHERE role_id = ?1 ORDER BY created_at ASC{}",
            limit_clause
        );
        let mut stmt = conn.prepare(&query)?;

        let rows = stmt.query_map(params![role_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;

        let mut messages = Vec::new();
        for row in rows {
            let (id, team_id, role_id, content, message_type, metadata, created_at) = row?;
            messages.push(Self::deserialize_message(
                id, team_id, role_id, content, message_type, metadata, created_at,
            )?);
        }
        Ok(messages)
    }

    // Telegram config
    fn upsert_telegram_config(&self, config: &TelegramBotConfig) -> Result<(), TeamRepositoryError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO telegram_bot_configs
             (id, role_id, bot_token, chat_id, enabled, notifications_enabled, conversation_enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                config.id,
                config.role_id,
                config.bot_token,
                config.chat_id,
                config.enabled as i32,
                config.notifications_enabled as i32,
                config.conversation_enabled as i32,
            ],
        )?;
        Ok(())
    }

    fn find_telegram_config_by_role(
        &self,
        role_id: &str,
    ) -> Result<Option<TelegramBotConfig>, TeamRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, role_id, bot_token, chat_id, enabled, notifications_enabled, conversation_enabled
             FROM telegram_bot_configs WHERE role_id = ?1",
        )?;

        let result = stmt.query_row(params![role_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i32>(4)?,
                row.get::<_, i32>(5)?,
                row.get::<_, i32>(6)?,
            ))
        });

        match result {
            Ok((id, role_id, bot_token, chat_id, enabled, notifications_enabled, conversation_enabled)) => {
                Ok(Some(TelegramBotConfig {
                    id,
                    role_id,
                    bot_token,
                    chat_id,
                    enabled: enabled != 0,
                    notifications_enabled: notifications_enabled != 0,
                    conversation_enabled: conversation_enabled != 0,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn delete_telegram_config(&self, role_id: &str) -> Result<bool, TeamRepositoryError> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "DELETE FROM telegram_bot_configs WHERE role_id = ?1",
            params![role_id],
        )?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_find_team() {
        let repo = SqliteTeamRepository::in_memory().unwrap();
        let team = Team::new("Test Team".to_string(), "A test team".to_string());

        repo.create_team(&team).unwrap();
        let found = repo.find_team_by_id(&team.id).unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, team.id);
        assert_eq!(found.name, "Test Team");
    }

    #[test]
    fn test_find_all_teams() {
        let repo = SqliteTeamRepository::in_memory().unwrap();
        let team1 = Team::new("Team 1".to_string(), "First".to_string());
        let team2 = Team::new("Team 2".to_string(), "Second".to_string());

        repo.create_team(&team1).unwrap();
        repo.create_team(&team2).unwrap();

        let all = repo.find_all_teams().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_role_crud() {
        let repo = SqliteTeamRepository::in_memory().unwrap();
        let team = Team::new("Test Team".to_string(), "A test team".to_string());
        repo.create_team(&team).unwrap();

        let role = TeamRole::new(
            Some(team.id.clone()),
            "Developer".to_string(),
            "A developer role".to_string(),
            ModelConfig::default(),
            "You are a developer".to_string(),
            vec!["dev".to_string(), "开发".to_string()],
        );

        repo.create_role(&role).unwrap();
        let found = repo.find_role_by_id(&role.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Developer");

        let roles = repo.find_roles_by_team(&team.id).unwrap();
        assert_eq!(roles.len(), 1);
    }

    #[test]
    fn test_skill_assignment() {
        let repo = SqliteTeamRepository::in_memory().unwrap();
        let team = Team::new("Test Team".to_string(), "A test team".to_string());
        repo.create_team(&team).unwrap();

        let role = TeamRole::new(
            Some(team.id.clone()),
            "Developer".to_string(),
            "A developer role".to_string(),
            ModelConfig::default(),
            "You are a developer".to_string(),
            vec!["dev".to_string()],
        );
        repo.create_role(&role).unwrap();

        repo.assign_skill(&role.id, "skill-1", SkillPriority::High).unwrap();
        let skills = repo.find_skills_by_role(&role.id).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].skill_id, "skill-1");
        assert_eq!(skills[0].priority, SkillPriority::High);

        repo.remove_skill(&role.id, "skill-1").unwrap();
        let skills = repo.find_skills_by_role(&role.id).unwrap();
        assert!(skills.is_empty());
    }

    #[test]
    fn test_telegram_config() {
        let repo = SqliteTeamRepository::in_memory().unwrap();
        let team = Team::new("Test Team".to_string(), "A test team".to_string());
        repo.create_team(&team).unwrap();

        let role = TeamRole::new(
            Some(team.id.clone()),
            "Developer".to_string(),
            "A developer role".to_string(),
            ModelConfig::default(),
            "You are a developer".to_string(),
            vec!["dev".to_string()],
        );
        repo.create_role(&role).unwrap();

        let config = TelegramBotConfig::new(role.id.clone(), "test-token".to_string());
        repo.upsert_telegram_config(&config).unwrap();

        let found = repo.find_telegram_config_by_role(&role.id).unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.bot_token, "test-token");
    }
}