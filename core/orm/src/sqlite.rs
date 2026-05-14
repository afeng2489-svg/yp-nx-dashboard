//! Generic SQLite storage backend using `rusqlite`.
//!
//! Wraps `Arc<Mutex<Connection>>` with helpers for table management,
//! parameterized CRUD, and JSON column serialization.
//!
//! Intended as the default implementation of [`Repository`](crate::Repository),
//! but can also be used standalone via the raw query helpers.

use crate::error::OrmError;
use parking_lot::Mutex;
use rusqlite::{params_from_iter, types::Value, Connection};
use std::path::Path;
use std::sync::Arc;

/// Reusable SQLite connection wrapper with schema management.
pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStore {
    /// Open (or create) a SQLite database at `path`.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, OrmError> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create an in-memory database (primarily for tests).
    pub fn in_memory() -> Result<Self, OrmError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Return a clone of the inner `Arc<Mutex<Connection>>` for use in
    /// repository impls that need direct rusqlite access.
    pub fn conn(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }

    /// Idempotent: run a `CREATE TABLE IF NOT EXISTS ...` statement.
    pub fn ensure_table(&self, ddl: &str) -> Result<(), OrmError> {
        self.conn.lock().execute_batch(ddl)?;
        Ok(())
    }

    // ── convenience helpers ──────────────────────────────────────────

    /// Execute a write statement with owned params.
    pub fn execute(&self, sql: &str, params: &[Value]) -> Result<usize, OrmError> {
        let conn = self.conn.lock();
        let affected = conn.execute(sql, params_from_iter(params.iter()))?;
        Ok(affected)
    }

    /// Execute a write statement with no params.
    pub fn execute_simple(&self, sql: &str) -> Result<usize, OrmError> {
        let conn = self.conn.lock();
        let affected = conn.execute(sql, [])?;
        Ok(affected)
    }

    /// Query a single row, mapped through a closure.
    ///
    /// Returns `Ok(None)` when no rows match.
    pub fn query_one<T, F>(
        &self,
        sql: &str,
        params: &[Value],
        mut map: F,
    ) -> Result<Option<T>, OrmError>
    where
        F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(sql)?;
        let mut rows = stmt.query_map(params_from_iter(params.iter()), &mut map)?;
        match rows.next() {
            Some(Ok(val)) => Ok(Some(val)),
            Some(Err(e)) => Err(OrmError::Database(e)),
            None => Ok(None),
        }
    }

    /// Query multiple rows, each mapped through the closure.
    pub fn query_all<T, F>(&self, sql: &str, params: &[Value], map: F) -> Result<Vec<T>, OrmError>
    where
        F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt
            .query_map(params_from_iter(params.iter()), map)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// Serialize an entity to JSON and store it in a `TEXT` column.
    ///
    /// This is a convenience for the common "entity as JSON blob" pattern
    /// used by both `nx_session::persistence` and `orchestrator::session_store`.
    pub fn save_as_json<T: serde::Serialize>(
        &self,
        sql: &str,
        params: &[Value],
        entity: &T,
        json_param_index: usize,
    ) -> Result<(), OrmError> {
        let json = serde_json::to_string(entity)?;
        let mut owned_params: Vec<Value> = params.to_vec();
        while owned_params.len() <= json_param_index {
            owned_params.push(Value::Null);
        }
        owned_params[json_param_index] = Value::Text(json);
        self.execute(sql, &owned_params)?;
        Ok(())
    }

    /// Query all rows and deserialize a JSON column into `T`.
    pub fn query_json_all<T: serde::de::DeserializeOwned>(
        &self,
        sql: &str,
        params: &[Value],
        json_col: usize,
    ) -> Result<Vec<T>, OrmError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(sql)?;
        let rows: Vec<T> = stmt
            .query_map(params_from_iter(params.iter()), |row| {
                let json_str: String = row.get(json_col)?;
                Ok(json_str)
            })?
            .filter_map(|r| r.ok())
            .filter_map(|json_str| serde_json::from_str(&json_str).ok())
            .collect();
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    fn text(s: &str) -> Value {
        Value::Text(s.to_string())
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestEntity {
        id: String,
        value: i32,
    }

    #[test]
    fn open_and_ensure_table() {
        let store = SqliteStore::in_memory().unwrap();
        store
            .ensure_table(
                "CREATE TABLE IF NOT EXISTS test (
                    id TEXT PRIMARY KEY,
                    data TEXT NOT NULL
                )",
            )
            .unwrap();
        store
            .execute(
                "INSERT INTO test (id, data) VALUES (?1, ?2)",
                &[text("a"), text("42")],
            )
            .unwrap();
        let count: usize = store
            .query_one("SELECT COUNT(*) FROM test", &[], |row| row.get(0))
            .unwrap()
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn save_and_load_json() {
        let store = SqliteStore::in_memory().unwrap();
        store
            .ensure_table(
                "CREATE TABLE IF NOT EXISTS items (
                    id TEXT PRIMARY KEY,
                    payload TEXT NOT NULL
                )",
            )
            .unwrap();

        let entity = TestEntity {
            id: "x".into(),
            value: 7,
        };
        store
            .save_as_json(
                "INSERT OR REPLACE INTO items (id, payload) VALUES (?1, ?2)",
                &[text("x")],
                &entity,
                1,
            )
            .unwrap();

        let loaded: Vec<TestEntity> = store
            .query_json_all(
                "SELECT id, payload FROM items WHERE id = ?1",
                &[text("x")],
                1,
            )
            .unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0], entity);
    }

    #[test]
    fn query_one_none_when_missing() {
        let store = SqliteStore::in_memory().unwrap();
        store
            .ensure_table("CREATE TABLE IF NOT EXISTS empty (id TEXT)")
            .unwrap();
        let result: Option<String> = store
            .query_one(
                "SELECT id FROM empty WHERE id = ?1",
                &[text("nope")],
                |row| row.get(0),
            )
            .unwrap();
        assert!(result.is_none());
    }
}
