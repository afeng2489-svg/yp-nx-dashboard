//! Group Chat Repository
//!
//! SQLite-based storage for group discussion sessions.

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::sync::Arc;

use crate::models::group_chat::{
    ConsensusStrategy, GroupConclusion, GroupMessage, GroupParticipant, GroupSession, GroupStatus,
    SpeakingStrategy, ToolCall,
};

/// Repository error
#[derive(Debug, thiserror::Error)]
pub enum GroupChatRepositoryError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Group chat repository trait
pub trait GroupChatRepository: Send + Sync {
    // Session operations
    fn create_session(&self, session: &GroupSession) -> Result<(), GroupChatRepositoryError>;
    fn get_session(&self, id: &str) -> Result<Option<GroupSession>, GroupChatRepositoryError>;
    fn get_sessions_by_team(
        &self,
        team_id: &str,
    ) -> Result<Vec<GroupSession>, GroupChatRepositoryError>;
    fn get_all_sessions(&self) -> Result<Vec<GroupSession>, GroupChatRepositoryError>;
    fn update_session(&self, session: &GroupSession) -> Result<(), GroupChatRepositoryError>;
    fn delete_session(&self, id: &str) -> Result<(), GroupChatRepositoryError>;

    // Message operations
    fn create_message(&self, message: &GroupMessage) -> Result<(), GroupChatRepositoryError>;
    fn get_messages_by_session(
        &self,
        session_id: &str,
        limit: Option<u32>,
        before: Option<&str>,
    ) -> Result<Vec<GroupMessage>, GroupChatRepositoryError>;
    fn get_message_count(&self, session_id: &str) -> Result<u32, GroupChatRepositoryError>;

    // Participant operations
    fn add_participant(
        &self,
        session_id: &str,
        participant: &GroupParticipant,
    ) -> Result<(), GroupChatRepositoryError>;
    fn get_participants(
        &self,
        session_id: &str,
    ) -> Result<Vec<GroupParticipant>, GroupChatRepositoryError>;
    fn update_participant(
        &self,
        session_id: &str,
        participant: &GroupParticipant,
    ) -> Result<(), GroupChatRepositoryError>;

    // Conclusion operations
    fn save_conclusion(&self, conclusion: &GroupConclusion)
        -> Result<(), GroupChatRepositoryError>;
    fn get_conclusion(
        &self,
        session_id: &str,
    ) -> Result<Option<GroupConclusion>, GroupChatRepositoryError>;
}

/// SQLite implementation
pub struct SqliteGroupChatRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteGroupChatRepository {
    pub fn new<P: AsRef<std::path::Path>>(db_path: P) -> Result<Self, GroupChatRepositoryError> {
        let conn = Connection::open(db_path)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn init_tables(&self) -> Result<(), GroupChatRepositoryError> {
        self.conn.lock().execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS group_sessions (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL,
                name TEXT NOT NULL,
                topic TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                speaking_strategy TEXT NOT NULL DEFAULT 'free',
                consensus_strategy TEXT NOT NULL DEFAULT 'majority',
                moderator_role_id TEXT,
                max_turns INTEGER NOT NULL DEFAULT 10,
                current_turn INTEGER NOT NULL DEFAULT 0,
                turn_policy TEXT NOT NULL DEFAULT 'all',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS group_messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role_id TEXT NOT NULL,
                role_name TEXT NOT NULL,
                content TEXT NOT NULL,
                tool_calls TEXT NOT NULL DEFAULT '[]',
                reply_to TEXT,
                turn_number INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES group_sessions(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS group_participants (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role_id TEXT NOT NULL,
                role_name TEXT NOT NULL,
                joined_at TEXT NOT NULL,
                last_spoke_at TEXT,
                message_count INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (session_id) REFERENCES group_sessions(id) ON DELETE CASCADE,
                UNIQUE(session_id, role_id)
            );

            CREATE TABLE IF NOT EXISTS group_conclusions (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL UNIQUE,
                content TEXT NOT NULL,
                consensus_level REAL NOT NULL DEFAULT 0.0,
                participant_scores TEXT NOT NULL DEFAULT '{}',
                agreed_by TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES group_sessions(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_group_messages_session ON group_messages(session_id);
            CREATE INDEX IF NOT EXISTS idx_group_participants_session ON group_participants(session_id);
            "#,
        )?;
        Ok(())
    }
}

impl GroupChatRepository for SqliteGroupChatRepository {
    fn create_session(&self, session: &GroupSession) -> Result<(), GroupChatRepositoryError> {
        self.conn.lock().execute(
            r#"INSERT INTO group_sessions
               (id, team_id, name, topic, status, speaking_strategy, consensus_strategy,
                moderator_role_id, max_turns, current_turn, turn_policy, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"#,
            params![
                session.id,
                session.team_id,
                session.name,
                session.topic,
                session.status.as_str(),
                session.speaking_strategy.as_str(),
                session.consensus_strategy.as_str(),
                session.moderator_role_id,
                session.max_turns,
                session.current_turn,
                session.turn_policy,
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn get_session(&self, id: &str) -> Result<Option<GroupSession>, GroupChatRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, team_id, name, topic, status, speaking_strategy, consensus_strategy,
                    moderator_role_id, max_turns, current_turn, turn_policy, created_at, updated_at
             FROM group_sessions WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(GroupSession {
                id: row.get(0)?,
                team_id: row.get(1)?,
                name: row.get(2)?,
                topic: row.get(3)?,
                status: GroupStatus::from_str(&row.get::<_, String>(4)?),
                speaking_strategy: SpeakingStrategy::from_str(&row.get::<_, String>(5)?),
                consensus_strategy: ConsensusStrategy::from_str(&row.get::<_, String>(6)?),
                moderator_role_id: row.get(7)?,
                max_turns: row.get(8)?,
                current_turn: row.get(9)?,
                turn_policy: row.get(10)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            }))
        } else {
            Ok(None)
        }
    }

    fn get_sessions_by_team(
        &self,
        team_id: &str,
    ) -> Result<Vec<GroupSession>, GroupChatRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, team_id, name, topic, status, speaking_strategy, consensus_strategy,
                    moderator_role_id, max_turns, current_turn, turn_policy, created_at, updated_at
             FROM group_sessions WHERE team_id = ?1 ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map(params![team_id], |row| {
            Ok(GroupSession {
                id: row.get(0)?,
                team_id: row.get(1)?,
                name: row.get(2)?,
                topic: row.get(3)?,
                status: GroupStatus::from_str(&row.get::<_, String>(4)?),
                speaking_strategy: SpeakingStrategy::from_str(&row.get::<_, String>(5)?),
                consensus_strategy: ConsensusStrategy::from_str(&row.get::<_, String>(6)?),
                moderator_role_id: row.get(7)?,
                max_turns: row.get(8)?,
                current_turn: row.get(9)?,
                turn_policy: row.get(10)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        let mut sessions = Vec::new();
        for session in rows {
            sessions.push(session?);
        }
        Ok(sessions)
    }

    fn get_all_sessions(&self) -> Result<Vec<GroupSession>, GroupChatRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, team_id, name, topic, status, speaking_strategy, consensus_strategy,
                    moderator_role_id, max_turns, current_turn, turn_policy, created_at, updated_at
             FROM group_sessions ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(GroupSession {
                id: row.get(0)?,
                team_id: row.get(1)?,
                name: row.get(2)?,
                topic: row.get(3)?,
                status: GroupStatus::from_str(&row.get::<_, String>(4)?),
                speaking_strategy: SpeakingStrategy::from_str(&row.get::<_, String>(5)?),
                consensus_strategy: ConsensusStrategy::from_str(&row.get::<_, String>(6)?),
                moderator_role_id: row.get(7)?,
                max_turns: row.get(8)?,
                current_turn: row.get(9)?,
                turn_policy: row.get(10)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        let mut sessions = Vec::new();
        for session in rows {
            sessions.push(session?);
        }
        Ok(sessions)
    }

    fn update_session(&self, session: &GroupSession) -> Result<(), GroupChatRepositoryError> {
        self.conn.lock().execute(
            r#"UPDATE group_sessions SET
               name = ?2, topic = ?3, status = ?4, speaking_strategy = ?5,
               consensus_strategy = ?6, moderator_role_id = ?7, max_turns = ?8,
               current_turn = ?9, turn_policy = ?10, updated_at = ?11
               WHERE id = ?1"#,
            params![
                session.id,
                session.name,
                session.topic,
                session.status.as_str(),
                session.speaking_strategy.as_str(),
                session.consensus_strategy.as_str(),
                session.moderator_role_id,
                session.max_turns,
                session.current_turn,
                session.turn_policy,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn delete_session(&self, id: &str) -> Result<(), GroupChatRepositoryError> {
        self.conn
            .lock()
            .execute("DELETE FROM group_sessions WHERE id = ?1", params![id])?;
        Ok(())
    }

    fn create_message(&self, message: &GroupMessage) -> Result<(), GroupChatRepositoryError> {
        let tool_calls_json = serde_json::to_string(&message.tool_calls)?;

        self.conn.lock().execute(
            r#"INSERT INTO group_messages
               (id, session_id, role_id, role_name, content, tool_calls, reply_to, turn_number, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            params![
                message.id,
                message.session_id,
                message.role_id,
                message.role_name,
                message.content,
                tool_calls_json,
                message.reply_to,
                message.turn_number,
                message.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn get_messages_by_session(
        &self,
        session_id: &str,
        limit: Option<u32>,
        before: Option<&str>,
    ) -> Result<Vec<GroupMessage>, GroupChatRepositoryError> {
        let limit = limit.unwrap_or(50);

        let query = if before.is_some() {
            "SELECT id, session_id, role_id, role_name, content, tool_calls, reply_to, turn_number, created_at
             FROM group_messages WHERE session_id = ?1 AND id < ?2 ORDER BY id DESC LIMIT ?3"
        } else {
            "SELECT id, session_id, role_id, role_name, content, tool_calls, reply_to, turn_number, created_at
             FROM group_messages WHERE session_id = ?1 ORDER BY id DESC LIMIT ?2"
        };

        let conn = self.conn.lock();
        let mut stmt = conn.prepare(query)?;

        let rows = if let Some(before_id) = before {
            stmt.query_map(params![session_id, before_id, limit], Self::row_to_message)?
        } else {
            stmt.query_map(params![session_id, limit], Self::row_to_message)?
        };

        let mut messages = Vec::new();
        for message in rows {
            messages.push(message?);
        }
        // Reverse to get chronological order
        messages.reverse();
        Ok(messages)
    }

    fn get_message_count(&self, session_id: &str) -> Result<u32, GroupChatRepositoryError> {
        let count: i64 = self.conn.lock().query_row(
            "SELECT COUNT(*) FROM group_messages WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;
        Ok(count as u32)
    }

    fn add_participant(
        &self,
        session_id: &str,
        participant: &GroupParticipant,
    ) -> Result<(), GroupChatRepositoryError> {
        self.conn.lock().execute(
            r#"INSERT OR REPLACE INTO group_participants
               (session_id, role_id, role_name, joined_at, last_spoke_at, message_count)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
            params![
                session_id,
                participant.role_id,
                participant.role_name,
                participant.joined_at.to_rfc3339(),
                participant.last_spoke_at.map(|dt| dt.to_rfc3339()),
                participant.message_count,
            ],
        )?;
        Ok(())
    }

    fn get_participants(
        &self,
        session_id: &str,
    ) -> Result<Vec<GroupParticipant>, GroupChatRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT role_id, role_name, joined_at, last_spoke_at, message_count
             FROM group_participants WHERE session_id = ?1",
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            Ok(GroupParticipant {
                role_id: row.get(0)?,
                role_name: row.get(1)?,
                joined_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                last_spoke_at: row
                    .get::<_, Option<String>>(3)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                message_count: row.get(4)?,
            })
        })?;

        let mut participants = Vec::new();
        for p in rows {
            participants.push(p?);
        }
        Ok(participants)
    }

    fn update_participant(
        &self,
        session_id: &str,
        participant: &GroupParticipant,
    ) -> Result<(), GroupChatRepositoryError> {
        self.conn.lock().execute(
            r#"UPDATE group_participants SET
               last_spoke_at = ?3, message_count = ?4
               WHERE session_id = ?1 AND role_id = ?2"#,
            params![
                session_id,
                participant.role_id,
                participant.last_spoke_at.map(|dt| dt.to_rfc3339()),
                participant.message_count,
            ],
        )?;
        Ok(())
    }

    fn save_conclusion(
        &self,
        conclusion: &GroupConclusion,
    ) -> Result<(), GroupChatRepositoryError> {
        let scores_json = serde_json::to_string(&conclusion.participant_scores)?;

        self.conn.lock().execute(
            r#"INSERT OR REPLACE INTO group_conclusions
               (id, session_id, content, consensus_level, participant_scores, agreed_by, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            params![
                conclusion.id,
                conclusion.session_id,
                conclusion.content,
                conclusion.consensus_level,
                scores_json,
                serde_json::to_string(&conclusion.agreed_by)?,
                conclusion.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn get_conclusion(
        &self,
        session_id: &str,
    ) -> Result<Option<GroupConclusion>, GroupChatRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, content, consensus_level, participant_scores, agreed_by, created_at
             FROM group_conclusions WHERE session_id = ?1",
        )?;

        let mut rows = stmt.query(params![session_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(GroupConclusion {
                id: row.get(0)?,
                session_id: row.get(1)?,
                content: row.get(2)?,
                consensus_level: row.get(3)?,
                participant_scores: serde_json::from_str(&row.get::<_, String>(4)?)?,
                agreed_by: serde_json::from_str(&row.get::<_, String>(5)?)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            }))
        } else {
            Ok(None)
        }
    }
}

impl SqliteGroupChatRepository {
    fn row_to_message(row: &rusqlite::Row) -> Result<GroupMessage, rusqlite::Error> {
        let tool_calls_str: String = row.get(5)?;
        let tool_calls: Vec<ToolCall> = serde_json::from_str(&tool_calls_str).unwrap_or_default();

        Ok(GroupMessage {
            id: row.get(0)?,
            session_id: row.get(1)?,
            role_id: row.get(2)?,
            role_name: row.get(3)?,
            content: row.get(4)?,
            tool_calls,
            reply_to: row.get(6)?,
            turn_number: row.get(7)?,
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_repo() -> SqliteGroupChatRepository {
        let repo = SqliteGroupChatRepository::new(":memory:").unwrap();
        repo.init_tables().unwrap();
        repo
    }

    #[test]
    fn test_create_and_get_session() {
        let repo = make_repo();

        let session = GroupSession::new(
            "team-1".to_string(),
            "Test Session".to_string(),
            "Test Topic".to_string(),
            SpeakingStrategy::Free,
            ConsensusStrategy::Majority,
            None,
            10,
            "all".to_string(),
        );

        repo.create_session(&session).unwrap();
        let retrieved = repo.get_session(&session.id).unwrap();

        assert!(retrieved.is_some());
        let s = retrieved.unwrap();
        assert_eq!(s.name, "Test Session");
        assert_eq!(s.topic, "Test Topic");
    }
}
