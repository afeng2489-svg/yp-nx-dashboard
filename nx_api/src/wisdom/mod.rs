//! Wisdom accumulation system
//!
//! Stores, queries, and manages accumulated knowledge across sessions.

pub mod entry;
pub mod query;
pub mod store;

use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

pub use entry::{
    CategorySummary, CreateWisdomRequest, QueryWisdomRequest, WisdomCategory, WisdomEntry,
    WisdomResponse,
};
pub use query::WisdomQueryService;
pub use store::{SqliteWisdomStore, StorageError, WisdomStore};

/// Wisdom service errors
#[derive(Error, Debug)]
pub enum WisdomError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Entry not found: {0}")]
    NotFound(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

/// Wisdom service for managing accumulated knowledge
#[derive(Clone)]
pub struct WisdomService {
    store: Arc<dyn WisdomStore>,
    query_service: WisdomQueryService,
}

impl WisdomService {
    /// Create a new wisdom service with SQLite storage
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, StorageError> {
        let store: Arc<dyn WisdomStore> = Arc::new(SqliteWisdomStore::new(db_path)?);
        let query_service = WisdomQueryService::new(Arc::clone(&store));
        Ok(Self {
            store,
            query_service,
        })
    }

    /// Create a wisdom service with a custom store
    pub fn with_store(store: Arc<dyn WisdomStore>) -> Self {
        let query_service = WisdomQueryService::new(Arc::clone(&store));
        Self {
            store,
            query_service,
        }
    }

    /// Add a new wisdom entry
    pub fn add(&self, request: CreateWisdomRequest) -> Result<WisdomEntry, WisdomError> {
        // Validate request
        if request.title.trim().is_empty() {
            return Err(WisdomError::InvalidRequest(
                "Title cannot be empty".to_string(),
            ));
        }
        if request.content.trim().is_empty() {
            return Err(WisdomError::InvalidRequest(
                "Content cannot be empty".to_string(),
            ));
        }

        let entry = WisdomEntry::new(
            request.category,
            request.title,
            request.content,
            request.tags,
            request.source_session,
            request.confidence,
        );

        self.store.save(&entry)?;
        Ok(entry)
    }

    /// Get a wisdom entry by ID
    pub fn get(&self, id: &str) -> Result<Option<WisdomEntry>, WisdomError> {
        Ok(self.query_service.get(id)?)
    }

    /// Query wisdom entries with filters
    pub fn query(&self, request: &QueryWisdomRequest) -> Result<WisdomResponse, WisdomError> {
        let limit = request.limit.unwrap_or(20).min(100);
        let offset = request.offset.unwrap_or(0);

        let (entries, total) = self.query_service.paginated(request)?;

        Ok(WisdomResponse {
            entries,
            total,
            offset,
            limit,
        })
    }

    /// Get entries by category
    pub fn by_category(&self, category: WisdomCategory) -> Result<Vec<WisdomEntry>, WisdomError> {
        Ok(self.query_service.by_category(category)?)
    }

    /// Search wisdom entries
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<WisdomEntry>, WisdomError> {
        Ok(self.query_service.search(query, limit)?)
    }

    /// Get category summaries
    pub fn categories(&self) -> Result<Vec<CategorySummary>, WisdomError> {
        Ok(self.query_service.category_summaries()?)
    }

    /// Delete a wisdom entry
    pub fn delete(&self, id: &str) -> Result<bool, WisdomError> {
        Ok(self.store.delete(id)?)
    }

    /// Update a wisdom entry
    pub fn update(&self, entry: WisdomEntry) -> Result<(), WisdomError> {
        Ok(self.store.update(&entry)?)
    }

    /// Get recent entries
    pub fn recent(&self, limit: usize) -> Result<Vec<WisdomEntry>, WisdomError> {
        Ok(self.query_service.recent(limit)?)
    }

    /// Get high confidence entries
    pub fn high_confidence(&self, min: f32, limit: usize) -> Result<Vec<WisdomEntry>, WisdomError> {
        Ok(self.query_service.high_confidence(min, limit)?)
    }

    /// Capture learning from a workflow execution
    pub fn capture_learning(
        &self,
        title: impl Into<String>,
        content: impl Into<String>,
        tags: Vec<String>,
        session_id: impl Into<String>,
        confidence: f32,
    ) -> Result<WisdomEntry, WisdomError> {
        let request = CreateWisdomRequest {
            category: WisdomCategory::Learning,
            title: title.into(),
            content: content.into(),
            tags,
            source_session: session_id.into(),
            confidence,
        };
        self.add(request)
    }

    /// Capture an architectural decision
    pub fn capture_decision(
        &self,
        title: impl Into<String>,
        content: impl Into<String>,
        tags: Vec<String>,
        session_id: impl Into<String>,
        confidence: f32,
    ) -> Result<WisdomEntry, WisdomError> {
        let request = CreateWisdomRequest {
            category: WisdomCategory::Decision,
            title: title.into(),
            content: content.into(),
            tags,
            source_session: session_id.into(),
            confidence,
        };
        self.add(request)
    }

    /// Capture a coding convention
    pub fn capture_convention(
        &self,
        title: impl Into<String>,
        content: impl Into<String>,
        tags: Vec<String>,
        session_id: impl Into<String>,
        confidence: f32,
    ) -> Result<WisdomEntry, WisdomError> {
        let request = CreateWisdomRequest {
            category: WisdomCategory::Convention,
            title: title.into(),
            content: content.into(),
            tags,
            source_session: session_id.into(),
            confidence,
        };
        self.add(request)
    }

    /// Capture a reusable pattern
    pub fn capture_pattern(
        &self,
        title: impl Into<String>,
        content: impl Into<String>,
        tags: Vec<String>,
        session_id: impl Into<String>,
        confidence: f32,
    ) -> Result<WisdomEntry, WisdomError> {
        let request = CreateWisdomRequest {
            category: WisdomCategory::Pattern,
            title: title.into(),
            content: content.into(),
            tags,
            source_session: session_id.into(),
            confidence,
        };
        self.add(request)
    }

    /// Capture a bug fix
    pub fn capture_fix(
        &self,
        title: impl Into<String>,
        content: impl Into<String>,
        tags: Vec<String>,
        session_id: impl Into<String>,
        confidence: f32,
    ) -> Result<WisdomEntry, WisdomError> {
        let request = CreateWisdomRequest {
            category: WisdomCategory::Fix,
            title: title.into(),
            content: content.into(),
            tags,
            source_session: session_id.into(),
            confidence,
        };
        self.add(request)
    }

    /// Suggest conventions based on context
    pub fn suggest_conventions(
        &self,
        context: &str,
        limit: usize,
    ) -> Result<Vec<WisdomEntry>, WisdomError> {
        let conventions = self.query_service.by_category(WisdomCategory::Convention)?;

        let context_lower = context.to_lowercase();
        let suggestions: Vec<WisdomEntry> = conventions
            .into_iter()
            .filter(|c| {
                c.title.to_lowercase().contains(&context_lower)
                    || c.tags
                        .iter()
                        .any(|t| context_lower.contains(&t.to_lowercase()))
            })
            .take(limit)
            .collect();

        Ok(suggestions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_service() -> WisdomService {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_wisdom.db");
        Box::leak(Box::new(dir));
        crate::migrations::run_all(path.to_str().unwrap()).unwrap();
        WisdomService::new(&path).unwrap()
    }

    #[test]
    fn test_add_and_get() {
        let service = test_service();

        let request = CreateWisdomRequest {
            category: WisdomCategory::Learning,
            title: "Test Wisdom".to_string(),
            content: "Test content about something important".to_string(),
            tags: vec!["test".to_string()],
            source_session: "test-session".to_string(),
            confidence: 0.9,
        };

        let entry = service.add(request).unwrap();
        assert!(!entry.id.is_empty());
        assert_eq!(entry.title, "Test Wisdom");

        let retrieved = service.get(&entry.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, entry.id);
    }

    #[test]
    fn test_query_by_category() {
        let service = test_service();

        service
            .add(CreateWisdomRequest {
                category: WisdomCategory::Pattern,
                title: "Pattern 1".to_string(),
                content: "Content".to_string(),
                tags: vec![],
                source_session: "session".to_string(),
                confidence: 0.8,
            })
            .unwrap();

        service
            .add(CreateWisdomRequest {
                category: WisdomCategory::Learning,
                title: "Learning 1".to_string(),
                content: "Content".to_string(),
                tags: vec![],
                source_session: "session".to_string(),
                confidence: 0.9,
            })
            .unwrap();

        let patterns = service.by_category(WisdomCategory::Pattern).unwrap();
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].title, "Pattern 1");
    }

    #[test]
    fn test_categories() {
        let service = test_service();

        for _ in 0..3 {
            service
                .add(CreateWisdomRequest {
                    category: WisdomCategory::Convention,
                    title: "Convention".to_string(),
                    content: "Content".to_string(),
                    tags: vec![],
                    source_session: "session".to_string(),
                    confidence: 0.8,
                })
                .unwrap();
        }

        let categories = service.categories().unwrap();
        assert!(!categories.is_empty());
    }

    #[test]
    fn test_invalid_request() {
        let service = test_service();

        let result = service.add(CreateWisdomRequest {
            category: WisdomCategory::Learning,
            title: "".to_string(), // Empty title should fail
            content: "Content".to_string(),
            tags: vec![],
            source_session: "session".to_string(),
            confidence: 0.9,
        });

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            WisdomError::InvalidRequest(_)
        ));
    }

    #[test]
    fn test_capture_methods() {
        let service = test_service();

        let entry = service
            .capture_learning(
                "New Learning",
                "Important content",
                vec!["tag1".to_string()],
                "session-1",
                0.95,
            )
            .unwrap();

        assert_eq!(entry.category, WisdomCategory::Learning);
        assert_eq!(entry.title, "New Learning");
        assert_eq!(entry.confidence, 0.95);
    }
}
