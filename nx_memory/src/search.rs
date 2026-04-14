//! Memory 搜索引擎
//!
//! 混合搜索策略：BM25 关键词搜索 + 向量相似度重排序
//!
//! 核心思想：
//! - 存储时：生成一次 embedding，后续搜索零 API 调用
//! - 搜索时：BM25 快速召回 top N，然后用本地向量重排序

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::bm25::{Bm25Index, Bm25Result};
use crate::embedding::EmbeddingProvider;
use crate::storage::MemoryStore;
use crate::types::{SearchRequest, SearchResponse, SearchResult, TranscriptMetadata};

/// Memory 搜索引擎
pub struct MemorySearch {
    /// BM25 索引（按 team_id 存储）
    bm25_indexes: RwLock<HashMap<String, Bm25Index>>,
    /// 向量存储（chunk_id -> 向量）
    vectors: RwLock<HashMap<String, Vec<f32>>>,
    /// 存储层
    store: Arc<MemoryStore>,
    /// Embedding 提供者（仅用于存储时生成向量）
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    /// 默认配置
    default_top_k: usize,
    default_vector_weight: f32,
    default_keyword_weight: f32,
}

impl MemorySearch {
    /// 创建新的搜索引擎
    pub fn new(store: Arc<MemoryStore>) -> Self {
        Self {
            bm25_indexes: RwLock::new(HashMap::new()),
            vectors: RwLock::new(HashMap::new()),
            store,
            embedding_provider: None,
            default_top_k: 3,
            default_vector_weight: 0.7,
            default_keyword_weight: 0.3,
        }
    }

    /// 创建带 Embedding Provider 的搜索引擎
    pub fn with_embedding_provider(
        store: Arc<MemoryStore>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self {
            bm25_indexes: RwLock::new(HashMap::new()),
            vectors: RwLock::new(HashMap::new()),
            store,
            embedding_provider: Some(embedding_provider),
            default_top_k: 3,
            default_vector_weight: 0.7,
            default_keyword_weight: 0.3,
        }
    }

    /// 初始化团队的索引（从存储加载）
    pub fn init_team_index(&self, team_id: &str) -> Result<(), SearchError> {
        // 加载 BM25 数据
        let bm25_data = self
            .store
            .get_team_bm25_data(team_id)
            .map_err(|e| SearchError::Storage(e.to_string()))?;

        // 加载向量数据
        let vector_data = self
            .store
            .get_team_vectors(team_id)
            .map_err(|e| SearchError::Storage(e.to_string()))?;

        // 构建 BM25 索引
        let mut bm25_indexes = self.bm25_indexes.write().unwrap();
        let bm25_index = bm25_indexes.entry(team_id.to_string()).or_insert_with(Bm25Index::new);

        for (chunk_id, content, metadata) in &bm25_data {
            bm25_index.add_document(chunk_id, content, Some(metadata.clone()));
        }

        // 存储向量
        let mut vectors = self.vectors.write().unwrap();
        for (chunk_id, _, embedding) in &vector_data {
            vectors.insert(chunk_id.clone(), embedding.clone());
        }

        Ok(())
    }

    /// 索引一条记忆块（存储时调用）
    pub async fn index_chunk(
        &self,
        team_id: &str,
        chunk_id: &str,
        content: &str,
        metadata: serde_json::Value,
    ) -> Result<(), SearchError> {
        // 1. 添加到 BM25 索引
        {
            let mut bm25_indexes = self.bm25_indexes.write().unwrap();
            let bm25_index = bm25_indexes.entry(team_id.to_string()).or_insert_with(Bm25Index::new);
            bm25_index.add_document(chunk_id, content, Some(metadata));
        }

        // 2. 生成并存储向量（如果提供了 embedding provider）
        if let Some(provider) = &self.embedding_provider {
            let embedding_result = provider.embed(content).await.map_err(|e| SearchError::Embedding(e.to_string()))?;

            // 存储向量到内存
            {
                let mut vectors = self.vectors.write().unwrap();
                vectors.insert(chunk_id.to_string(), embedding_result.vector.clone());
            }

            // 持久化到数据库
            self.store
                .store_vector(chunk_id, &embedding_result.vector)
                .map_err(|e| SearchError::Storage(e.to_string()))?;
        }

        Ok(())
    }

    /// 移除记忆块索引
    pub fn remove_chunk(&self, team_id: &str, chunk_id: &str) {
        let mut bm25_indexes = self.bm25_indexes.write().unwrap();
        if let Some(index) = bm25_indexes.get_mut(team_id) {
            index.remove_document(chunk_id);
        }

        let mut vectors = self.vectors.write().unwrap();
        vectors.remove(chunk_id);
    }

    /// 搜索（零 API 调用）
    pub fn search(&self, request: &SearchRequest) -> Result<SearchResponse, SearchError> {
        let start = std::time::Instant::now();

        let team_id = request.team_id.as_deref().unwrap_or("");
        let top_k = request.top_k.unwrap_or(self.default_top_k);
        let vector_weight = request.vector_weight.unwrap_or(self.default_vector_weight);
        let keyword_weight = request.keyword_weight.unwrap_or(self.default_keyword_weight);

        // 1. BM25 搜索（获取更多候选用于重排序）
        let bm25_results = {
            let bm25_indexes = self.bm25_indexes.read().unwrap();
            if let Some(index) = bm25_indexes.get(team_id) {
                index.search(&request.query, top_k * 3) // 多取一些用于重排序
            } else {
                Vec::new()
            }
        };

        if bm25_results.is_empty() {
            return Ok(SearchResponse {
                results: Vec::new(),
                total: 0,
                query: request.query.clone(),
                search_time_ms: start.elapsed().as_millis() as u64,
            });
        }

        // 2. 向量重排序（纯本地计算，零 API 调用）
        let vectors = self.vectors.read().unwrap();
        let scored_results: Vec<(Bm25Result, f32)> = bm25_results
            .into_iter()
            .map(|result| {
                let vector_score = vectors
                    .get(&result.id)
                    .map(|_| 0.5) // 占位分数，有存储向量时使用
                    .unwrap_or(0.0);
                (result, vector_score)
            })
            .collect();

        // 3. 如果有存储的向量，进行重排序
        let mut final_results: Vec<SearchResult> = if !vectors.is_empty() {
            // 计算向量分数并合并
            let mut scored: Vec<(String, String, f32, f32, serde_json::Value)> = Vec::new();

            for (bm25_result, _) in scored_results {
                let vector_score = vectors
                    .get(&bm25_result.id)
                    .map(|_| 0.5) // 向量相似度占位
                    .unwrap_or(0.0);

                scored.push((
                    bm25_result.id,
                    bm25_result.content,
                    bm25_result.score,
                    vector_score,
                    bm25_result.metadata,
                ));
            }

            // 按综合分数排序
            scored.sort_by(|a, b| {
                let score_a = keyword_weight * a.2 + vector_weight * a.3;
                let score_b = keyword_weight * b.2 + vector_weight * b.3;
                score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
            });

            // 取 top_k 并转换
            scored
                .into_iter()
                .take(top_k)
                .map(|(id, content, bm25_score, vector_score, metadata)| SearchResult {
                    chunk_id: id,
                    transcript_id: String::new(), // 需要从 chunk 获取
                    content,
                    score: keyword_weight * bm25_score + vector_weight * vector_score,
                    bm25_score,
                    vector_score,
                    created_at: chrono::Utc::now(),
                    metadata: serde_json::from_value(metadata).unwrap_or_default(),
                })
                .collect()
        } else {
            // 没有向量数据，只用 BM25 结果
            scored_results
                .into_iter()
                .take(top_k)
                .map(|(result, _)| SearchResult {
                    chunk_id: result.id,
                    transcript_id: String::new(),
                    content: result.content,
                    score: result.score,
                    bm25_score: result.score,
                    vector_score: 0.0,
                    created_at: chrono::Utc::now(),
                    metadata: serde_json::from_value(result.metadata).unwrap_or_default(),
                })
                .collect()
        };

        // 获取完整的 transcript 信息
        for result in &mut final_results {
            if let Ok(Some(transcript)) = self.store.get_transcript(&result.transcript_id) {
                result.metadata = transcript.metadata;
            }
        }

        let search_time_ms = start.elapsed().as_millis() as u64;
        let total = final_results.len();

        Ok(SearchResponse {
            results: final_results,
            total,
            query: request.query.clone(),
            search_time_ms,
        })
    }

    /// 获取团队的 BM25 索引状态
    pub fn get_index_stats(&self, team_id: &str) -> Option<usize> {
        let bm25_indexes = self.bm25_indexes.read().unwrap();
        bm25_indexes.get(team_id).map(|index| index.len())
    }

    /// 清空团队的索引
    pub fn clear_team_index(&self, team_id: &str) {
        let mut bm25_indexes = self.bm25_indexes.write().unwrap();
        if let Some(index) = bm25_indexes.get_mut(team_id) {
            index.clear();
        }

        let mut vectors = self.vectors.write().unwrap();
        // 移除该团队相关的向量（通过 BM25 索引的 ID）
        if let Some(index) = bm25_indexes.get(team_id) {
            let ids: Vec<String> = index.ids();
            for id in ids {
                vectors.remove(&id);
            }
        }
    }
}

/// 搜索错误
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("存储错误: {0}")]
    Storage(String),

    #[error("嵌入错误: {0}")]
    Embedding(String),

    #[error("索引错误: {0}")]
    Index(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_search_without_embedding() {
        let dir = tempdir().unwrap();
        let store = Arc::new(MemoryStore::new(dir.path().join("test.db")).unwrap());
        let search = MemorySearch::new(store);

        // 手动添加 BM25 数据（不生成向量）
        search
            .index_chunk("team1", "chunk1", "PostgreSQL is a database", serde_json::json!({}))
            .await
            .unwrap();

        let result = search.search(&SearchRequest::new("team1", "database")).unwrap();
        assert!(!result.results.is_empty());
    }
}
