//! Feature Flag Repository — SQLite 持久化
//!
//! 新表 `feature_flags`，独立于核心表。

use std::sync::Arc;
use parking_lot::Mutex;
use rusqlite::Connection;
use chrono::Utc;

use crate::models::feature_flag::{FeatureFlag, FeatureFlagState};
use super::error::TeamEvolutionError;

pub struct SqliteFeatureFlagRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteFeatureFlagRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Result<Self, TeamEvolutionError> {
        let repo = Self { conn };
        repo.initialize_tables()?;
        Ok(repo)
    }

    fn initialize_tables(&self) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS feature_flags (
                key TEXT PRIMARY KEY,
                state TEXT NOT NULL DEFAULT 'off',
                circuit_breaker INTEGER NOT NULL DEFAULT 0,
                error_count INTEGER NOT NULL DEFAULT 0,
                error_threshold INTEGER NOT NULL DEFAULT 5,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );"
        )?;
        Ok(())
    }

    pub fn upsert(&self, flag: &FeatureFlag) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO feature_flags (key, state, circuit_breaker, error_count, error_threshold, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(key) DO UPDATE SET
                state = excluded.state,
                circuit_breaker = excluded.circuit_breaker,
                error_count = excluded.error_count,
                error_threshold = excluded.error_threshold,
                updated_at = excluded.updated_at",
            rusqlite::params![
                flag.key,
                flag.state.as_str(),
                flag.circuit_breaker as i32,
                flag.error_count,
                flag.error_threshold,
                flag.created_at.to_rfc3339(),
                flag.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn find_by_key(&self, key: &str) -> Result<Option<FeatureFlag>, TeamEvolutionError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT key, state, circuit_breaker, error_count, error_threshold, created_at, updated_at
             FROM feature_flags WHERE key = ?1"
        )?;

        let result = stmt.query_row(rusqlite::params![key], |row| {
            Ok(FeatureFlag {
                key: row.get(0)?,
                state: FeatureFlagState::from_str(&row.get::<_, String>(1)?)
                    .unwrap_or(FeatureFlagState::Off),
                circuit_breaker: row.get::<_, i32>(2)? != 0,
                error_count: row.get(3)?,
                error_threshold: row.get(4)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        });

        match result {
            Ok(flag) => Ok(Some(flag)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(TeamEvolutionError::from(e)),
        }
    }

    pub fn find_all(&self) -> Result<Vec<FeatureFlag>, TeamEvolutionError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT key, state, circuit_breaker, error_count, error_threshold, created_at, updated_at
             FROM feature_flags ORDER BY key"
        )?;

        let flags = stmt.query_map([], |row| {
            Ok(FeatureFlag {
                key: row.get(0)?,
                state: FeatureFlagState::from_str(&row.get::<_, String>(1)?)
                    .unwrap_or(FeatureFlagState::Off),
                circuit_breaker: row.get::<_, i32>(2)? != 0,
                error_count: row.get(3)?,
                error_threshold: row.get(4)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(flags)
    }

    pub fn increment_error(&self, key: &str) -> Result<FeatureFlag, TeamEvolutionError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE feature_flags SET error_count = error_count + 1, updated_at = ?1 WHERE key = ?2",
            rusqlite::params![now, key],
        )?;

        // Check if circuit breaker should trip
        let mut stmt = conn.prepare(
            "SELECT key, state, circuit_breaker, error_count, error_threshold, created_at, updated_at
             FROM feature_flags WHERE key = ?1"
        )?;

        let flag = stmt.query_row(rusqlite::params![key], |row| {
            Ok(FeatureFlag {
                key: row.get(0)?,
                state: FeatureFlagState::from_str(&row.get::<_, String>(1)?)
                    .unwrap_or(FeatureFlagState::Off),
                circuit_breaker: row.get::<_, i32>(2)? != 0,
                error_count: row.get(3)?,
                error_threshold: row.get(4)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        }).map_err(|_| TeamEvolutionError::FlagNotFound(key.to_string()))?;

        // Auto-trip circuit breaker
        if flag.should_trip() && !flag.circuit_breaker {
            conn.execute(
                "UPDATE feature_flags SET circuit_breaker = 1, state = 'off', updated_at = ?1 WHERE key = ?2",
                rusqlite::params![now, key],
            )?;

            return Ok(FeatureFlag {
                circuit_breaker: true,
                state: FeatureFlagState::Off,
                ..flag
            });
        }

        Ok(flag)
    }

    pub fn reset_error_count(&self, key: &str) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE feature_flags SET error_count = 0, circuit_breaker = 0, updated_at = ?1 WHERE key = ?2",
            rusqlite::params![now, key],
        )?;
        Ok(())
    }

    pub fn delete(&self, key: &str) -> Result<(), TeamEvolutionError> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM feature_flags WHERE key = ?1", rusqlite::params![key])?;
        Ok(())
    }
}
