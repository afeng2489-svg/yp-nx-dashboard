//! NexusFlow 搜索
//!
//! 代码搜索功能: ACE 语义搜索、CodexLens FTS、混合搜索。

pub mod embedding;
pub mod index;
pub mod searcher;
pub mod ace;
pub mod codexlens;
pub mod hybrid;

pub use embedding::{EmbeddingProvider, EmbeddingResult, AIEmbeddingAdapter};
pub use index::{Document, Chunk, VectorIndex};
pub use searcher::{SearchResult, SearchOptions, CodeSearcher};

// ACE 语义搜索
pub use ace::{
    AceEngine, AceConfig, AceSearchResult, AceSearchHit, AceSearchMode,
    AceError, SymbolGraph, SymbolContext,
};

// CodexLens FTS
pub use codexlens::{
    CodexLensEngine, CodexLensConfig, CodexLensResult, CodexLensHit,
    CodexLensStats, CodexLensSearchOptions, QueryType,
};

// Hybrid 混合搜索
pub use hybrid::{
    HybridSearchEngine, HybridConfig, HybridSearchResult, HybridSearchHit,
    HybridSearchMode, HybridError, ScoreBreakdown, MatchType,
};