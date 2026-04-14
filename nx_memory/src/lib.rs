//! NexusFlow Memory 模块
//!
//! 团队对话记忆存储和检索系统
//!
//! 特性：
//! - BM25 关键词搜索（零 Token 消耗）
//! - 预计算向量存储（搜索零 Token 消耗）
//! - 混合搜索重排序
//! - Claude Embedding Provider 支持
//!
//! 架构：
//! ```
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      Memory 模块                            │
//! │                                                        │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │
//! │  │   Types    │  │   BM25     │  │  Storage    │   │
//! │  │  (类型定义) │  │   Index    │  │  (SQLite)   │   │
//! │  └─────────────┘  └─────────────┘  └─────────────┘   │
//! │                                                        │
//! │  ┌─────────────┐  ┌─────────────┐                      │
//! │  │  Embedding  │  │   Search   │                      │
//! │  │  Provider   │  │  Engine    │                      │
//! │  │  (嵌入)     │  │  (混合)     │                      │
//! │  └─────────────┘  └─────────────┘                      │
//! └─────────────────────────────────────────────────────────────┘
//! ```

pub mod types;
pub mod bm25;
pub mod storage;
pub mod embedding;
pub mod search;

// Re-exports
pub use types::*;
pub use bm25::Bm25Index;
pub use storage::MemoryStore;
pub use embedding::{EmbeddingProvider, EmbeddingResult, ClaudeEmbeddingProvider};
pub use search::MemorySearch;
