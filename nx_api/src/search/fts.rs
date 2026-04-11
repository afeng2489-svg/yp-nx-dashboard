//! FTS5 Full-Text Search Types
//!
//! Types for FTS5-based full-text search functionality.

use super::*;

/// FTS5 search query types
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FtsQueryType {
    /// Simple keyword query
    Simple,
    /// Phrase query (exact phrase match)
    Phrase,
    /// Prefix query (word prefix matching)
    Prefix,
    /// Wildcard query
    Wildcard,
    /// Regular expression query
    Regex,
    /// Fuzzy query (edit distance)
    Fuzzy,
}

/// FTS5 search result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FtsSearchResult {
    /// Original query
    pub query: String,
    /// Search results
    pub results: Vec<FtsSearchHit>,
    /// Total hits
    pub total_hits: usize,
    /// Search time in milliseconds
    pub search_time_ms: u64,
    /// Query type used
    pub query_type: FtsQueryType,
}

/// FTS search hit
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FtsSearchHit {
    /// Document ID
    pub document_id: String,
    /// File path
    pub file: String,
    /// Line number
    pub line_number: usize,
    /// Content snippet
    pub snippet: String,
    /// Match score
    pub score: f32,
    /// Matched terms
    pub matched_terms: Vec<String>,
    /// Programming language
    pub language: Option<String>,
}

impl From<FtsSearchHit> for SearchHit {
    fn from(hit: FtsSearchHit) -> Self {
        Self {
            file: hit.file,
            line_number: hit.line_number,
            snippet: hit.snippet,
            score: hit.score,
            search_mode: SearchMode::FTS,
            language: hit.language,
            symbol_context: None,
        }
    }
}

/// FTS configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FtsConfig {
    /// Minimum word length
    pub min_word_length: usize,
    /// Maximum word length
    pub max_word_length: usize,
    /// Enable fuzzy matching
    pub enable_fuzzy: bool,
    /// Fuzzy tolerance (edit distance)
    pub fuzzy_tolerance: usize,
    /// Enable code-aware tokenization
    pub code_aware_tokenization: bool,
    /// Enable stop words filtering
    pub enable_stop_words: bool,
    /// Maximum results
    pub max_results: usize,
}

impl Default for FtsConfig {
    fn default() -> Self {
        Self {
            min_word_length: 2,
            max_word_length: 64,
            enable_fuzzy: true,
            fuzzy_tolerance: 2,
            code_aware_tokenization: true,
            enable_stop_words: true,
            max_results: 100,
        }
    }
}
