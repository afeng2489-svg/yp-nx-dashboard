//! Wisdom storage
//!
//! SQLite implementation for persisting wisdom entries.

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

use super::entry::{CategorySummary, QueryWisdomRequest, WisdomCategory, WisdomEntry};

/// Storage error types
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Wisdom entry not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid category: {0}")]
    InvalidCategory(String),
}

/// Trait for wisdom storage operations
pub trait WisdomStore: Send + Sync {
    /// Save a new wisdom entry
    fn save(&self, entry: &WisdomEntry) -> Result<(), StorageError>;

    /// Find a wisdom entry by ID
    fn find_by_id(&self, id: &str) -> Result<Option<WisdomEntry>, StorageError>;

    /// Query wisdom entries with filters
    fn query(&self, request: &QueryWisdomRequest) -> Result<Vec<WisdomEntry>, StorageError>;

    /// Count entries matching the query
    fn count(&self, request: &QueryWisdomRequest) -> Result<usize, StorageError>;

    /// Get entries by category
    fn find_by_category(&self, category: WisdomCategory) -> Result<Vec<WisdomEntry>, StorageError>;

    /// Get category counts
    fn category_counts(&self) -> Result<Vec<CategorySummary>, StorageError>;

    /// Search entries by text query
    fn search(&self, query: &str, limit: usize) -> Result<Vec<WisdomEntry>, StorageError>;

    /// Delete a wisdom entry
    fn delete(&self, id: &str) -> Result<bool, StorageError>;

    /// Update an existing entry
    fn update(&self, entry: &WisdomEntry) -> Result<(), StorageError>;
}

/// SQLite implementation of wisdom storage
#[derive(Debug, Clone)]
pub struct SqliteWisdomStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteWisdomStore {
    /// Create a new SQLite wisdom store
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, StorageError> {
        let conn = Connection::open(db_path)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create an in-memory store for testing
    #[allow(dead_code)]
    pub fn in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(crate::migrations::WISDOM_SCHEMA)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Serialize tags to JSON string
    fn serialize_tags(tags: &[String]) -> String {
        serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string())
    }

    /// Deserialize tags from JSON string
    fn deserialize_tags(s: &str) -> Vec<String> {
        serde_json::from_str(s).unwrap_or_default()
    }

    /// Deserialize a row into a WisdomEntry
    fn deserialize_row(
        id: String,
        category: String,
        title: String,
        content: String,
        tags: String,
        source_session: String,
        confidence: f64,
        created_at: String,
    ) -> Result<WisdomEntry, StorageError> {
        let category = WisdomCategory::from_str(&category)
            .ok_or_else(|| StorageError::InvalidCategory(category.clone()))?;

        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(WisdomEntry {
            id,
            category,
            title,
            content,
            tags: Self::deserialize_tags(&tags),
            source_session,
            confidence: confidence as f32,
            created_at,
        })
    }
}

impl WisdomStore for SqliteWisdomStore {
    fn save(&self, entry: &WisdomEntry) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute(
            r#"INSERT INTO wisdom (id, category, title, content, tags, source_session, confidence, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
            params![
                entry.id,
                entry.category.as_str(),
                entry.title,
                entry.content,
                Self::serialize_tags(&entry.tags),
                entry.source_session,
                entry.confidence,
                entry.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn find_by_id(&self, id: &str) -> Result<Option<WisdomEntry>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, category, title, content, tags, source_session, confidence, created_at
             FROM wisdom WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, f64>(6)?,
                row.get::<_, String>(7)?,
            ))
        });

        match result {
            Ok((id, category, title, content, tags, source_session, confidence, created_at)) => {
                Ok(Some(Self::deserialize_row(
                    id,
                    category,
                    title,
                    content,
                    tags,
                    source_session,
                    confidence,
                    created_at,
                )?))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn query(&self, request: &QueryWisdomRequest) -> Result<Vec<WisdomEntry>, StorageError> {
        let conn = self.conn.lock();
        let limit = request.limit.unwrap_or(50).min(100);
        let offset = request.offset.unwrap_or(0);

        let mut sql = String::from(
            "SELECT id, category, title, content, tags, source_session, confidence, created_at FROM wisdom WHERE 1=1",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref cat) = request.category {
            sql.push_str(" AND category = ?");
            params_vec.push(Box::new(cat.as_str().to_string()));
        }

        if let Some(min_conf) = request.min_confidence {
            sql.push_str(" AND confidence >= ?");
            params_vec.push(Box::new(min_conf as f64));
        }

        if !request.tags.is_empty() {
            for tag in &request.tags {
                sql.push_str(" AND tags LIKE ?");
                params_vec.push(Box::new(format!("%\"{} \"%", tag)));
            }
        }

        sql.push_str(" ORDER BY created_at DESC");
        sql.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, f64>(6)?,
                row.get::<_, String>(7)?,
            ))
        })?;

        let mut entries = Vec::new();
        for row in rows {
            let (id, category, title, content, tags, source_session, confidence, created_at) = row?;
            entries.push(Self::deserialize_row(
                id,
                category,
                title,
                content,
                tags,
                source_session,
                confidence,
                created_at,
            )?);
        }

        // Filter by text query if specified
        if let Some(ref query) = request.query {
            let query_lower = query.to_lowercase();
            entries.retain(|e| {
                e.title.to_lowercase().contains(&query_lower)
                    || e.content.to_lowercase().contains(&query_lower)
                    || e.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
            });
        }

        Ok(entries)
    }

    fn count(&self, request: &QueryWisdomRequest) -> Result<usize, StorageError> {
        let entries = self.query(request)?;
        Ok(entries.len())
    }

    fn find_by_category(&self, category: WisdomCategory) -> Result<Vec<WisdomEntry>, StorageError> {
        let request = QueryWisdomRequest {
            category: Some(category),
            ..Default::default()
        };
        self.query(&request)
    }

    fn category_counts(&self) -> Result<Vec<CategorySummary>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT category, COUNT(*) as count FROM wisdom GROUP BY category ORDER BY count DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            let category_str: String = row.get(0)?;
            let count: usize = row.get(1)?;
            Ok((category_str, count))
        })?;

        let mut summaries = Vec::new();
        for row in rows {
            let (category_str, count) = row?;
            if let Some(category) = WisdomCategory::from_str(&category_str) {
                summaries.push(CategorySummary { category, count });
            }
        }

        Ok(summaries)
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<WisdomEntry>, StorageError> {
        let request = QueryWisdomRequest {
            query: Some(query.to_string()),
            limit: Some(limit),
            ..Default::default()
        };
        self.query(&request)
    }

    fn delete(&self, id: &str) -> Result<bool, StorageError> {
        let conn = self.conn.lock();
        let affected = conn.execute("DELETE FROM wisdom WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    fn update(&self, entry: &WisdomEntry) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            r#"UPDATE wisdom
               SET category = ?1, title = ?2, content = ?3, tags = ?4,
                   source_session = ?5, confidence = ?6
               WHERE id = ?7"#,
            params![
                entry.category.as_str(),
                entry.title,
                entry.content,
                Self::serialize_tags(&entry.tags),
                entry.source_session,
                entry.confidence,
                entry.id,
            ],
        )?;
        if affected == 0 {
            return Err(StorageError::NotFound(entry.id.clone()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_find() {
        let store = SqliteWisdomStore::in_memory().unwrap();
        let entry = WisdomEntry::new(
            WisdomCategory::Learning,
            "Rust Error Handling",
            "Use anyhow::Error for application errors",
            vec!["rust".to_string(), "errors".to_string()],
            "session-1",
            0.95,
        );

        store.save(&entry).unwrap();

        let found = store.find_by_id(&entry.id).unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, entry.id);
        assert_eq!(found.category, WisdomCategory::Learning);
        assert_eq!(found.title, "Rust Error Handling");
    }

    #[test]
    fn test_query_by_category() {
        let store = SqliteWisdomStore::in_memory().unwrap();

        let entry1 = WisdomEntry::new(
            WisdomCategory::Learning,
            "Title 1",
            "Content",
            vec![],
            "session",
            0.9,
        );
        let entry2 = WisdomEntry::new(
            WisdomCategory::Pattern,
            "Title 2",
            "Content",
            vec![],
            "session",
            0.8,
        );

        store.save(&entry1).unwrap();
        store.save(&entry2).unwrap();

        let learning_entries = store.find_by_category(WisdomCategory::Learning).unwrap();
        assert_eq!(learning_entries.len(), 1);
        assert_eq!(learning_entries[0].title, "Title 1");
    }

    #[test]
    fn test_search() {
        let store = SqliteWisdomStore::in_memory().unwrap();

        let entry = WisdomEntry::new(
            WisdomCategory::Convention,
            "Naming Conventions",
            "Use snake_case for functions in Rust",
            vec!["rust".to_string(), "naming".to_string()],
            "session-1",
            0.9,
        );

        store.save(&entry).unwrap();

        let results = store.search("rust", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Naming Conventions");
    }

    #[test]
    fn test_delete() {
        let store = SqliteWisdomStore::in_memory().unwrap();
        let entry = WisdomEntry::new(
            WisdomCategory::Fix,
            "Memory Leak",
            "Fixed by adding drop()",
            vec![],
            "session",
            1.0,
        );

        store.save(&entry).unwrap();

        let deleted = store.delete(&entry.id).unwrap();
        assert!(deleted);

        let found = store.find_by_id(&entry.id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_category_counts() {
        let store = SqliteWisdomStore::in_memory().unwrap();

        for _ in 0..3 {
            store
                .save(&WisdomEntry::new(
                    WisdomCategory::Learning,
                    "Title",
                    "Content",
                    vec![],
                    "session",
                    0.9,
                ))
                .unwrap();
        }

        for _ in 0..2 {
            store
                .save(&WisdomEntry::new(
                    WisdomCategory::Pattern,
                    "Title",
                    "Content",
                    vec![],
                    "session",
                    0.8,
                ))
                .unwrap();
        }

        let counts = store.category_counts().unwrap();
        assert_eq!(counts.len(), 2);
        // Should be ordered by count descending
        assert_eq!(counts[0].category, WisdomCategory::Learning);
        assert_eq!(counts[0].count, 3);
    }
}
