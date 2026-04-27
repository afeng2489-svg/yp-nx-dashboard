//! Wisdom query interface
//!
//! Provides a clean interface for querying wisdom entries.

use std::sync::Arc;

use super::entry::{CategorySummary, QueryWisdomRequest, WisdomCategory, WisdomEntry};
use super::store::{StorageError, WisdomStore};

/// Query service for wisdom entries
#[derive(Clone)]
pub struct WisdomQueryService {
    store: Arc<dyn WisdomStore>,
}

impl WisdomQueryService {
    /// Create a new query service
    pub fn new(store: Arc<dyn WisdomStore>) -> Self {
        Self { store }
    }

    /// Get wisdom entry by ID
    pub fn get(&self, id: &str) -> Result<Option<WisdomEntry>, StorageError> {
        self.store.find_by_id(id)
    }

    /// Query wisdom entries with filters
    pub fn query(&self, request: &QueryWisdomRequest) -> Result<Vec<WisdomEntry>, StorageError> {
        self.store.query(request)
    }

    /// Get entries by category
    pub fn by_category(&self, category: WisdomCategory) -> Result<Vec<WisdomEntry>, StorageError> {
        self.store.find_by_category(category)
    }

    /// Search wisdom entries
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<WisdomEntry>, StorageError> {
        self.store.search(query, limit)
    }

    /// Get all category summaries
    pub fn category_summaries(&self) -> Result<Vec<CategorySummary>, StorageError> {
        self.store.category_counts()
    }

    /// Get paginated results with total count
    pub fn paginated(
        &self,
        request: &QueryWisdomRequest,
    ) -> Result<(Vec<WisdomEntry>, usize), StorageError> {
        let entries = self.store.query(request)?;
        let total = self.store.count(request)?;
        Ok((entries, total))
    }

    /// Get recent wisdom entries
    pub fn recent(&self, limit: usize) -> Result<Vec<WisdomEntry>, StorageError> {
        let request = QueryWisdomRequest {
            limit: Some(limit),
            ..Default::default()
        };
        self.store.query(&request)
    }

    /// Get high confidence entries
    pub fn high_confidence(
        &self,
        min_confidence: f32,
        limit: usize,
    ) -> Result<Vec<WisdomEntry>, StorageError> {
        let request = QueryWisdomRequest {
            min_confidence: Some(min_confidence),
            limit: Some(limit),
            ..Default::default()
        };
        self.store.query(&request)
    }

    /// Get related entries by tags
    pub fn related_by_tags(
        &self,
        tags: &[String],
        exclude_id: &str,
        limit: usize,
    ) -> Result<Vec<WisdomEntry>, StorageError> {
        let all_entries = self.store.query(&QueryWisdomRequest {
            limit: Some(100),
            ..Default::default()
        })?;

        let mut entries_with_scores: Vec<(WisdomEntry, usize)> = all_entries
            .into_iter()
            .filter(|e| e.id != exclude_id)
            .filter_map(|e| {
                let score = e
                    .tags
                    .iter()
                    .filter(|t| tags.iter().any(|rt| rt.to_lowercase() == t.to_lowercase()))
                    .count();
                if score > 0 {
                    Some((e, score))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending, then by confidence descending
        entries_with_scores.sort_by(|a, b| b.1.cmp(&a.1));
        let sorted: Vec<WisdomEntry> = entries_with_scores
            .into_iter()
            .map(|(e, _)| e)
            .take(limit)
            .collect();

        Ok(sorted)
    }
}

#[cfg(test)]
mod tests {
    use super::super::store::SqliteWisdomStore;
    use super::*;

    #[test]
    fn test_recent_entries() {
        let store = SqliteWisdomStore::in_memory().unwrap();
        let query_service = WisdomQueryService::new(Arc::new(store));

        for i in 0..5 {
            let entry = WisdomEntry::new(
                WisdomCategory::Learning,
                format!("Title {}", i),
                "Content",
                vec![],
                "session",
                0.9,
            );
            query_service.store.save(&entry).unwrap();
        }

        let recent = query_service.recent(3).unwrap();
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_high_confidence() {
        let store = SqliteWisdomStore::in_memory().unwrap();
        let query_service = WisdomQueryService::new(Arc::new(store));

        query_service
            .store
            .save(&WisdomEntry::new(
                WisdomCategory::Pattern,
                "High Confidence",
                "Content",
                vec![],
                "session",
                0.95,
            ))
            .unwrap();

        query_service
            .store
            .save(&WisdomEntry::new(
                WisdomCategory::Pattern,
                "Low Confidence",
                "Content",
                vec![],
                "session",
                0.5,
            ))
            .unwrap();

        let high_conf = query_service.high_confidence(0.9, 10).unwrap();
        assert_eq!(high_conf.len(), 1);
        assert_eq!(high_conf[0].title, "High Confidence");
    }

    #[test]
    fn test_category_summaries() {
        let store = SqliteWisdomStore::in_memory().unwrap();
        let query_service = WisdomQueryService::new(Arc::new(store));

        for _ in 0..2 {
            query_service
                .store
                .save(&WisdomEntry::new(
                    WisdomCategory::Decision,
                    "Title",
                    "Content",
                    vec![],
                    "session",
                    0.9,
                ))
                .unwrap();
        }

        let summaries = query_service.category_summaries().unwrap();
        assert!(!summaries.is_empty());
        assert_eq!(summaries[0].category, WisdomCategory::Decision);
        assert_eq!(summaries[0].count, 2);
    }
}
