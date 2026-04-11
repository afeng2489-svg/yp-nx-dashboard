//! Semantic Search Types
//!
//! Types for semantic/vector-based search functionality.

use super::*;

/// Semantic search result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemanticSearchResult {
    /// Original query
    pub query: String,
    /// Search results
    pub results: Vec<SemanticSearchHit>,
    /// Total hits
    pub total_hits: usize,
    /// Search time in milliseconds
    pub search_time_ms: u64,
    /// Embedding model used
    pub model: Option<String>,
}

/// Semantic search hit
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemanticSearchHit {
    /// Chunk ID
    pub chunk_id: String,
    /// Document ID
    pub document_id: String,
    /// File path
    pub file: String,
    /// Start line number
    pub start_line: usize,
    /// End line number
    pub end_line: usize,
    /// Content snippet
    pub snippet: String,
    /// Semantic similarity score
    pub semantic_score: f32,
    /// Relevance score (after reranking)
    pub relevance_score: Option<f32>,
    /// Programming language
    pub language: Option<String>,
    /// Symbol context
    pub symbol_context: Option<SymbolContext>,
}

impl From<SemanticSearchHit> for SearchHit {
    fn from(hit: SemanticSearchHit) -> Self {
        Self {
            file: hit.file,
            line_number: hit.start_line,
            snippet: hit.snippet,
            score: hit.relevance_score.unwrap_or(hit.semantic_score),
            search_mode: SearchMode::Semantic,
            language: hit.language,
            symbol_context: hit.symbol_context,
        }
    }
}

/// Semantic search configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemanticConfig {
    /// Maximum results to return
    pub max_results: usize,
    /// Minimum similarity score
    pub min_score: f32,
    /// Enable reranking
    pub enable_rerank: bool,
    /// Reranking top-k parameter
    pub rerank_top_k: usize,
    /// Context window size
    pub context_window: usize,
    /// Include metadata
    pub include_metadata: bool,
    /// Embedding dimension
    pub embedding_dimension: usize,
}

impl Default for SemanticConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            min_score: 0.5,
            enable_rerank: true,
            rerank_top_k: 20,
            context_window: 512,
            include_metadata: true,
            embedding_dimension: 384,
        }
    }
}

/// Embedding provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingProviderType {
    /// OpenAI embeddings
    OpenAI,
    /// Local embedding model
    Local,
    /// Mock embeddings (for testing)
    Mock,
}

impl Default for EmbeddingProviderType {
    fn default() -> Self {
        EmbeddingProviderType::Mock
    }
}
