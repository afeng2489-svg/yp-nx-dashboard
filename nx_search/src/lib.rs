//! NexusFlow 搜索
//!
//! 代码搜索功能: ACE 语义搜索、CodexLens FTS、混合搜索。

pub mod ace;
pub mod codexlens;
pub mod embedding;
pub mod hybrid;
pub mod index;
pub mod searcher;

pub use embedding::{AIEmbeddingAdapter, EmbeddingProvider, EmbeddingResult};
pub use index::{Chunk, Document, VectorIndex};
pub use searcher::{CodeSearcher, SearchOptions, SearchResult};

// ACE 语义搜索
pub use ace::{
    AceConfig, AceEngine, AceError, AceSearchHit, AceSearchMode, AceSearchResult, SymbolContext,
    SymbolGraph,
};

// CodexLens FTS
pub use codexlens::{
    CodexLensConfig, CodexLensEngine, CodexLensHit, CodexLensResult, CodexLensSearchOptions,
    CodexLensStats, QueryType,
};

// Hybrid 混合搜索
pub use hybrid::{
    HybridConfig, HybridError, HybridSearchEngine, HybridSearchHit, HybridSearchMode,
    HybridSearchResult, MatchType, ScoreBreakdown,
};
