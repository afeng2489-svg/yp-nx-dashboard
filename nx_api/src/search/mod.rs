//! NexusFlow Search API Types
//!
//! Unified search interface for FTS5, semantic, and hybrid search modes.

pub mod fts;
pub mod semantic;
pub mod hybrid;

pub use fts::*;
pub use semantic::*;
pub use hybrid::*;

/// Search mode enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    /// Full-text search mode
    #[serde(alias = "fts", alias = "full-text", alias = "fulltext")]
    FTS,
    /// Semantic search mode (vector similarity)
    #[serde(alias = "semantic", alias = "vector", alias = "embedding")]
    Semantic,
    /// Hybrid search mode (FTS + Semantic combined)
    #[serde(alias = "hybrid", alias = "combined")]
    Hybrid,
}

impl Default for SearchMode {
    fn default() -> Self {
        SearchMode::Hybrid
    }
}

impl std::fmt::Display for SearchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchMode::FTS => write!(f, "fts"),
            SearchMode::Semantic => write!(f, "semantic"),
            SearchMode::Hybrid => write!(f, "hybrid"),
        }
    }
}

impl std::str::FromStr for SearchMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fts" | "full-text" | "fulltext" => Ok(SearchMode::FTS),
            "semantic" | "vector" | "embedding" => Ok(SearchMode::Semantic),
            "hybrid" | "combined" => Ok(SearchMode::Hybrid),
            _ => Err(format!("Unknown search mode: {}", s)),
        }
    }
}

/// Unified search result structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    /// Query string
    pub query: String,
    /// Search results
    pub results: Vec<SearchHit>,
    /// Total number of results
    pub total_hits: usize,
    /// Search time in milliseconds
    pub search_time_ms: u64,
    /// Search mode used
    pub search_mode: SearchMode,
    /// Available search modes
    pub available_modes: Vec<SearchMode>,
}

/// Individual search hit
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchHit {
    /// File path
    pub file: String,
    /// Line number
    pub line_number: usize,
    /// Content snippet
    pub snippet: String,
    /// Relevance score
    pub score: f32,
    /// Search mode that produced this result
    pub search_mode: SearchMode,
    /// Programming language (if detected)
    pub language: Option<String>,
    /// Symbol context (for semantic results)
    pub symbol_context: Option<SymbolContext>,
}

/// Symbol context for code navigation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SymbolContext {
    /// Symbol name
    pub name: String,
    /// Symbol kind (function, class, etc.)
    pub kind: String,
    /// Function signature
    pub signature: Option<String>,
    /// Parent symbol
    pub parent: Option<String>,
}

/// Search options
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchOptions {
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Minimum score threshold
    pub min_score: Option<f32>,
    /// Language filter
    pub language_filter: Option<Vec<String>>,
    /// File path filter (glob patterns)
    pub path_filter: Option<Vec<String>>,
    /// Include context lines
    pub include_context: Option<bool>,
    /// Context line count
    pub context_lines: Option<usize>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: Some(20),
            min_score: Some(0.1),
            language_filter: None,
            path_filter: None,
            include_context: Some(true),
            context_lines: Some(2),
        }
    }
}

/// Index request for reindexing codebase
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndexRequest {
    /// Workspace path to index
    pub workspace_path: String,
    /// Programming languages to index
    pub languages: Option<Vec<String>>,
    /// Force reindex (clear existing index)
    pub force: Option<bool>,
}

/// Index response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndexResponse {
    /// Number of documents indexed
    pub documents_indexed: usize,
    /// Number of chunks indexed
    pub chunks_indexed: usize,
    /// Index size in bytes
    pub index_size_bytes: usize,
    /// Indexing time in milliseconds
    pub indexing_time_ms: u64,
}

/// Search modes response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchModesResponse {
    /// Available search modes
    pub modes: Vec<SearchModeInfo>,
}

/// Information about a search mode
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchModeInfo {
    /// Mode identifier
    pub mode: SearchMode,
    /// Human-readable name
    pub name: String,
    /// Description
    pub description: String,
    /// Whether this mode is available
    pub available: bool,
}

impl SearchModesResponse {
    /// Get all available search modes
    pub fn available_modes() -> Self {
        Self {
            modes: vec![
                SearchModeInfo {
                    mode: SearchMode::FTS,
                    name: "Full-Text Search".to_string(),
                    description: "Keyword-based search using FTS5 inverted index".to_string(),
                    available: true,
                },
                SearchModeInfo {
                    mode: SearchMode::Semantic,
                    name: "Semantic Search".to_string(),
                    description: "Vector similarity search using embeddings".to_string(),
                    available: true,
                },
                SearchModeInfo {
                    mode: SearchMode::Hybrid,
                    name: "Hybrid Search".to_string(),
                    description: "Combines FTS and semantic search for best results".to_string(),
                    available: true,
                },
            ],
        }
    }
}
