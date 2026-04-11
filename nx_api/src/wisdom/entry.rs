//! Wisdom entry types
//!
//! Defines the core data structures for wisdom entries.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Wisdom categories for organizing different types of learnings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WisdomCategory {
    /// Learns from execution results and outcomes
    Learning,
    /// Architectural decisions and their rationale
    Decision,
    /// Coding conventions and style guidelines
    Convention,
    /// Reusable patterns and best practices
    Pattern,
    /// Bug fixes and their solutions
    Fix,
}

impl WisdomCategory {
    /// Convert category to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            WisdomCategory::Learning => "learning",
            WisdomCategory::Decision => "decision",
            WisdomCategory::Convention => "convention",
            WisdomCategory::Pattern => "pattern",
            WisdomCategory::Fix => "fix",
        }
    }

    /// Parse category from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "learning" => Some(WisdomCategory::Learning),
            "decision" => Some(WisdomCategory::Decision),
            "convention" => Some(WisdomCategory::Convention),
            "pattern" => Some(WisdomCategory::Pattern),
            "fix" => Some(WisdomCategory::Fix),
            _ => None,
        }
    }

    /// Get all category values
    pub fn all() -> Vec<WisdomCategory> {
        vec![
            WisdomCategory::Learning,
            WisdomCategory::Decision,
            WisdomCategory::Convention,
            WisdomCategory::Pattern,
            WisdomCategory::Fix,
        ]
    }
}

impl std::fmt::Display for WisdomCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A wisdom entry representing accumulated knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WisdomEntry {
    /// Unique identifier
    pub id: String,
    /// Category of the wisdom entry
    pub category: WisdomCategory,
    /// Brief title describing the wisdom
    pub title: String,
    /// Detailed content or description
    pub content: String,
    /// Tags for categorization and search
    pub tags: Vec<String>,
    /// Session this wisdom was captured from
    pub source_session: String,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl WisdomEntry {
    /// Create a new wisdom entry with generated ID and timestamp
    pub fn new(
        category: WisdomCategory,
        title: impl Into<String>,
        content: impl Into<String>,
        tags: Vec<String>,
        source_session: impl Into<String>,
        confidence: f32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            category,
            title: title.into(),
            content: content.into(),
            tags,
            source_session: source_session.into(),
            confidence: confidence.clamp(0.0, 1.0),
            created_at: Utc::now(),
        }
    }

    /// Create a wisdom entry with specific ID (for testing or import)
    #[allow(dead_code)]
    pub fn with_id(
        id: impl Into<String>,
        category: WisdomCategory,
        title: impl Into<String>,
        content: impl Into<String>,
        tags: Vec<String>,
        source_session: impl Into<String>,
        confidence: f32,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: id.into(),
            category,
            title: title.into(),
            content: content.into(),
            tags,
            source_session: source_session.into(),
            confidence: confidence.clamp(0.0, 1.0),
            created_at,
        }
    }
}

/// Request to create a new wisdom entry
#[derive(Debug, Clone, Deserialize)]
pub struct CreateWisdomRequest {
    pub category: WisdomCategory,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub source_session: String,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
}

fn default_confidence() -> f32 {
    0.8
}

/// Request to query wisdom entries
#[derive(Debug, Clone, Deserialize, Default)]
pub struct QueryWisdomRequest {
    #[serde(default)]
    pub category: Option<WisdomCategory>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub min_confidence: Option<f32>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
}

/// Response containing wisdom entries with metadata
#[derive(Debug, Clone, Serialize)]
pub struct WisdomResponse {
    pub entries: Vec<WisdomEntry>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

/// Category summary for listing
#[derive(Debug, Clone, Serialize)]
pub struct CategorySummary {
    pub category: WisdomCategory,
    pub count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wisdom_entry_creation() {
        let entry = WisdomEntry::new(
            WisdomCategory::Learning,
            "Test Title",
            "Test Content",
            vec!["rust".to_string(), "testing".to_string()],
            "session-123",
            0.9,
        );

        assert!(!entry.id.is_empty());
        assert_eq!(entry.category, WisdomCategory::Learning);
        assert_eq!(entry.title, "Test Title");
        assert_eq!(entry.content, "Test Content");
        assert_eq!(entry.tags.len(), 2);
        assert_eq!(entry.source_session, "session-123");
        assert_eq!(entry.confidence, 0.9);
    }

    #[test]
    fn test_category_conversion() {
        assert_eq!(WisdomCategory::Learning.as_str(), "learning");
        assert_eq!(WisdomCategory::from_str("learning"), Some(WisdomCategory::Learning));
        assert_eq!(WisdomCategory::from_str("unknown"), None);
    }

    #[test]
    fn test_confidence_clamping() {
        let entry = WisdomEntry::new(
            WisdomCategory::Pattern,
            "Title",
            "Content",
            vec![],
            "session",
            1.5, // Should be clamped to 1.0
        );
        assert_eq!(entry.confidence, 1.0);

        let entry2 = WisdomEntry::new(
            WisdomCategory::Pattern,
            "Title",
            "Content",
            vec![],
            "session",
            -0.5, // Should be clamped to 0.0
        );
        assert_eq!(entry2.confidence, 0.0);
    }
}
