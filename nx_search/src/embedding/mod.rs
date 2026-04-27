//! Embedding 提供者

mod provider_adapter;

pub use provider_adapter::AIEmbeddingAdapter;

use serde::{Deserialize, Serialize};
use std::pin::Pin;

/// Embedding 向量结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResult {
    /// 向量数据
    pub vector: Vec<f32>,
    /// 模型名称
    pub model: String,
    /// token 数
    pub token_count: usize,
}

/// Embedding 提供者特征
pub trait EmbeddingProvider: Send + Sync {
    /// 提供者名称
    fn name(&self) -> &str;

    /// 生成文本的 embedding
    fn embed(
        &self,
        text: &str,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<EmbeddingResult, EmbedError>> + Send + '_>>;

    /// 批量生成 embedding
    fn embed_batch(
        &self,
        texts: &[String],
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<Vec<EmbeddingResult>, EmbedError>> + Send + '_>,
    >;
}

/// Embedding 错误
#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("API 错误: {0}")]
    Api(String),

    #[error("网络错误: {0}")]
    Network(String),

    #[error("解析错误: {0}")]
    Parse(String),
}
