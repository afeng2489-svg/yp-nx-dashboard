//! Hybrid Search Types
//!
//! Types for combining FTS and semantic search.

use super::*;

/// Hybrid search mode variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HybridSearchMode {
    /// Auto-select best mode
    Auto,
    /// Semantic search only
    SemanticOnly,
    /// Keyword search only
    KeywordOnly,
    /// Combined hybrid search
    Hybrid,
    /// Semantic first, fallback to keyword
    SemanticFirst,
    /// Keyword first, fallback to semantic
    KeywordFirst,
}

impl Default for HybridSearchMode {
    fn default() -> Self {
        HybridSearchMode::Auto
    }
}

/// Hybrid search result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HybridSearchResult {
    /// Original query
    pub query: String,
    /// Combined search results
    pub results: Vec<HybridSearchHit>,
    /// Total hits
    pub total_hits: usize,
    /// Search time in milliseconds
    pub search_time_ms: u64,
    /// Mode used for this search
    pub mode_used: HybridSearchMode,
    /// Semantic search result (if applicable)
    pub semantic_result: Option<SemanticSearchResult>,
    /// Keyword search result (if applicable)
    pub keyword_result: Option<FtsSearchResult>,
    /// Score breakdown
    pub score_breakdown: ScoreBreakdown,
}

/// Hybrid search hit with score composition
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HybridSearchHit {
    /// Document ID
    pub document_id: String,
    /// File path
    pub file: String,
    /// Content snippet
    pub snippet: String,
    /// Start line
    pub start_line: usize,
    /// End line
    pub end_line: usize,
    /// Combined score
    pub score: f32,
    /// Semantic component score
    pub semantic_score: f32,
    /// Keyword component score
    pub keyword_score: f32,
    /// Programming language
    pub language: Option<String>,
    /// Match types
    pub match_types: Vec<MatchType>,
}

/// Match type indicators
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    /// Semantic match
    Semantic,
    /// Exact match
    Exact,
    /// Prefix match
    Prefix,
    /// Fuzzy match
    Fuzzy,
}

/// Score breakdown for hybrid results
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScoreBreakdown {
    /// Total combined score
    pub total_score: f32,
    /// Semantic weight
    pub semantic_weight: f32,
    /// Keyword weight
    pub keyword_weight: f32,
    /// Number of semantic results
    pub semantic_count: usize,
    /// Number of keyword results
    pub keyword_count: usize,
}

impl Default for ScoreBreakdown {
    fn default() -> Self {
        Self {
            total_score: 0.0,
            semantic_weight: 0.5,
            keyword_weight: 0.5,
            semantic_count: 0,
            keyword_count: 0,
        }
    }
}

impl From<HybridSearchHit> for SearchHit {
    fn from(hit: HybridSearchHit) -> Self {
        Self {
            file: hit.file,
            line_number: hit.start_line,
            snippet: hit.snippet,
            score: hit.score,
            search_mode: SearchMode::Hybrid,
            language: hit.language,
            symbol_context: None,
        }
    }
}

/// Hybrid search configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HybridConfig {
    /// ACE semantic search config
    pub ace_config: SemanticConfig,
    /// CodexLens FTS config
    pub fts_config: FtsConfig,
    /// Hybrid search mode
    pub mode: HybridSearchMode,
    /// Semantic weight (0.0 to 1.0)
    pub semantic_weight: f32,
    /// Keyword weight (0.0 to 1.0)
    pub keyword_weight: f32,
    /// Auto-select mode based on query
    pub auto_select_mode: bool,
    /// Auto-select threshold
    pub auto_select_threshold: f32,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            ace_config: SemanticConfig::default(),
            fts_config: FtsConfig::default(),
            mode: HybridSearchMode::Auto,
            semantic_weight: 0.5,
            keyword_weight: 0.5,
            auto_select_mode: true,
            auto_select_threshold: 0.3,
        }
    }
}
