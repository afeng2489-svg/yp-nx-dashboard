//! 知识库检索
//!
//! 向量相似度检索：从 DB 加载 chunks → cosine similarity → top_k → 过滤阈值

use crate::services::knowledge::repository::KnowledgeRepository;
use std::sync::Arc;

/// 检索结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub chunk_id: String,
    pub document_id: String,
    pub content: String,
    pub score: f32,
    pub chunk_index: usize,
}

/// 检索错误
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("仓储错误: {0}")]
    Repository(#[from] crate::services::knowledge::repository::RepositoryError),
    #[error("Embedding 错误: {0}")]
    Embedding(String),
}

/// 在指定知识库中检索相关 chunks
pub fn search_similar(
    repo: &Arc<KnowledgeRepository>,
    kb_id: &str,
    query_embedding: &[f32],
    top_k: usize,
    threshold: f32,
) -> Result<Vec<SearchResult>, SearchError> {
    let chunks = repo.get_chunks_with_embeddings(kb_id)?;

    let mut scored: Vec<SearchResult> = chunks
        .into_iter()
        .filter_map(|chunk| {
            let embedding = chunk.embedding?;
            let score = cosine_similarity(query_embedding, &embedding);
            if score >= threshold {
                Some(SearchResult {
                    chunk_id: chunk.id,
                    document_id: chunk.document_id,
                    content: chunk.content,
                    score,
                    chunk_index: chunk.chunk_index,
                })
            } else {
                None
            }
        })
        .collect();

    // 按分数降序排序
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(top_k);

    Ok(scored)
}

/// 计算余弦相似度（从 nx_memory 复用）
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        0.0
    } else {
        dot / (mag_a * mag_b)
    }
}
