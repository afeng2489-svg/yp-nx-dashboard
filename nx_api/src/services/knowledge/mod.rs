//! 知识库服务
//!
//! 门面层：文档上传 → 分块 → embedding → 持久化 + 检索

pub mod chunker;
pub mod kb_search;
pub mod repository;

use crate::services::knowledge::chunker::TextChunk;
use crate::services::knowledge::kb_search::SearchResult;
use crate::services::knowledge::repository::{
    KbChunk, KbDocument, KnowledgeBase, KnowledgeRepository,
};
use nx_memory::embedding::{create_provider_from_config, EmbeddingProvider};
use parking_lot::Mutex;
use std::sync::Arc;

/// 知识库服务错误
#[derive(Debug, thiserror::Error)]
pub enum KnowledgeError {
    #[error("仓储错误: {0}")]
    Repository(#[from] repository::RepositoryError),
    #[error("检索错误: {0}")]
    Search(String),
    #[error("知识库不存在: {0}")]
    NotFound(String),
    #[error("文档为空")]
    EmptyDocument,
}

/// 知识库服务
pub struct KnowledgeService {
    repo: Arc<KnowledgeRepository>,
    embedding_provider: Mutex<Option<Arc<dyn EmbeddingProvider>>>,
}

impl KnowledgeService {
    pub fn new(repo: Arc<KnowledgeRepository>) -> Self {
        Self {
            repo,
            embedding_provider: Mutex::new(None),
        }
    }

    /// 配置 embedding provider
    pub fn set_embedding_provider(&self, provider: Box<dyn EmbeddingProvider>) {
        *self.embedding_provider.lock() = Some(Arc::from(provider));
    }

    /// 从 API key 环境变量自动配置 OpenAI embedding
    pub fn configure_from_env(&self) {
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            if !api_key.is_empty() {
                match create_provider_from_config("openai", Some(&api_key), None, None) {
                    Ok(provider) => {
                        tracing::info!("[KnowledgeService] OpenAI embedding provider 已配置");
                        self.set_embedding_provider(provider);
                    }
                    Err(e) => {
                        tracing::warn!("[KnowledgeService] 配置 OpenAI embedding 失败: {}", e);
                    }
                }
            }
        } else {
            tracing::info!(
                "[KnowledgeService] 未找到 OPENAI_API_KEY，embedding 不可用，将使用纯文本模式"
            );
        }
    }

    /// Clone Arc 出 Mutex，避免跨 await 持锁
    fn get_provider(&self) -> Option<Arc<dyn EmbeddingProvider>> {
        self.embedding_provider.lock().as_ref().cloned()
    }
}

// ── KnowledgeBase 操作 ──

impl KnowledgeService {
    pub fn create_knowledge_base(
        &self,
        name: String,
        description: Option<String>,
        embedding_provider: String,
        embedding_model: String,
        embedding_dimension: i64,
        chunk_size: i64,
    ) -> Result<KnowledgeBase, KnowledgeError> {
        let now = chrono::Utc::now().to_rfc3339();
        let kb = KnowledgeBase {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            embedding_provider,
            embedding_model,
            embedding_dimension,
            chunk_size,
            created_at: now.clone(),
            updated_at: now,
        };
        self.repo.create_knowledge_base(&kb)?;
        Ok(kb)
    }

    pub fn list_knowledge_bases(&self) -> Result<Vec<KnowledgeBase>, KnowledgeError> {
        self.repo.list_knowledge_bases().map_err(Into::into)
    }

    pub fn get_knowledge_base(&self, id: &str) -> Result<KnowledgeBase, KnowledgeError> {
        self.repo
            .get_knowledge_base(id)?
            .ok_or_else(|| KnowledgeError::NotFound(id.to_string()))
    }

    pub fn delete_knowledge_base(&self, id: &str) -> Result<(), KnowledgeError> {
        self.repo.delete_knowledge_base(id)?;
        Ok(())
    }
}

// ── Document 操作 ──

impl KnowledgeService {
    /// 上传文档：分块 + embedding + 持久化
    pub async fn upload_document(
        &self,
        kb_id: &str,
        filename: String,
        content: String,
    ) -> Result<KbDocument, KnowledgeError> {
        // 验证知识库存在
        let kb = self
            .repo
            .get_knowledge_base(kb_id)?
            .ok_or_else(|| KnowledgeError::NotFound(kb_id.to_string()))?;

        if content.trim().is_empty() {
            return Err(KnowledgeError::EmptyDocument);
        }

        let doc_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let doc = KbDocument {
            id: doc_id.clone(),
            knowledge_base_id: kb_id.to_string(),
            filename: filename.clone(),
            content_type: content_type_from_filename(&filename),
            file_size: content.len() as i64,
            chunk_count: 0,
            status: "indexing".to_string(),
            error: None,
            created_at: now,
        };
        self.repo.insert_document(&doc)?;

        // 分块
        let text_chunks = chunker::chunk_text(&content, kb.chunk_size as usize);

        if text_chunks.is_empty() {
            self.repo
                .update_document_status(&doc_id, "failed", 0, Some("文档分块结果为空"))?;
            return Ok(doc);
        }

        // 生成 embedding（如果 provider 可用）
        let embeddings = self.generate_embeddings(&text_chunks).await;

        // 持久化 chunks
        let chunk_models: Vec<KbChunk> = text_chunks
            .into_iter()
            .enumerate()
            .map(|(i, tc)| {
                let embedding = embeddings.as_ref().and_then(|em| em.get(i).cloned());
                KbChunk {
                    id: uuid::Uuid::new_v4().to_string(),
                    document_id: doc_id.clone(),
                    knowledge_base_id: kb_id.to_string(),
                    chunk_index: tc.index,
                    content: tc.content,
                    token_count: tc.token_count as i64,
                    embedding,
                    created_at: chrono::Utc::now().to_rfc3339(),
                }
            })
            .collect();

        let chunk_count = chunk_models.len() as i64;

        match self.repo.insert_chunks_batch(&chunk_models) {
            Ok(()) => {
                self.repo
                    .update_document_status(&doc_id, "ready", chunk_count, None)?;
            }
            Err(e) => {
                self.repo.update_document_status(
                    &doc_id,
                    "failed",
                    0,
                    Some(&format!("持久化 chunks 失败: {}", e)),
                )?;
            }
        }

        // 重新获取更新后的文档
        let updated_doc = KbDocument {
            chunk_count,
            status: "ready".to_string(),
            ..doc
        };
        Ok(updated_doc)
    }

    pub fn list_documents(&self, kb_id: &str) -> Result<Vec<KbDocument>, KnowledgeError> {
        self.repo.list_documents(kb_id).map_err(Into::into)
    }

    pub fn delete_document(&self, doc_id: &str) -> Result<(), KnowledgeError> {
        self.repo.delete_document(doc_id)?;
        Ok(())
    }
}

// ── Search 操作 ──

impl KnowledgeService {
    /// 检索相关 chunks
    pub async fn search(
        &self,
        kb_id: &str,
        query: &str,
        top_k: usize,
        threshold: f32,
    ) -> Result<Vec<SearchResult>, KnowledgeError> {
        let query_embedding = if let Some(p) = self.get_provider() {
            let result = p
                .embed(query)
                .await
                .map_err(|e| KnowledgeError::Search(format!("生成 query embedding 失败: {}", e)))?;
            Some(result.vector)
        } else {
            None
        };

        match query_embedding {
            Some(qe) => kb_search::search_similar(&self.repo, kb_id, &qe, top_k, threshold)
                .map_err(|e| KnowledgeError::Search(e.to_string())),
            None => {
                // 没有 embedding provider，返回空结果
                tracing::warn!("[KnowledgeService] 无 embedding provider，无法执行向量检索");
                Ok(Vec::new())
            }
        }
    }

    /// 检索并返回纯文本（供工作流 RAG 注入用）
    pub async fn retrieve_texts(
        &self,
        kb_id: &str,
        query: &str,
        top_k: usize,
        threshold: f32,
    ) -> Result<Vec<String>, KnowledgeError> {
        let results = self.search(kb_id, query, top_k, threshold).await?;
        Ok(results.into_iter().map(|r| r.content).collect())
    }
}

// ── 内部辅助 ──

impl KnowledgeService {
    /// 批量生成 embedding
    async fn generate_embeddings(&self, chunks: &[TextChunk]) -> Option<Vec<Vec<f32>>> {
        let p = self.get_provider()?;

        let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();

        let batch_size = 100;
        let mut all_embeddings = Vec::with_capacity(chunks.len());

        for batch in texts.chunks(batch_size) {
            match p.embed_batch(&batch.to_vec()).await {
                Ok(results) => {
                    for r in results {
                        all_embeddings.push(r.vector);
                    }
                }
                Err(e) => {
                    tracing::warn!("[KnowledgeService] Embedding batch 失败: {}", e);
                    return None;
                }
            }
        }

        Some(all_embeddings)
    }
}

#[async_trait::async_trait]
impl nexus_workflow::watcher::RagProvider for KnowledgeService {
    async fn retrieve(
        &self,
        kb_id: &str,
        query: &str,
        top_k: usize,
        threshold: f32,
    ) -> Vec<String> {
        match self.retrieve_texts(kb_id, query, top_k, threshold).await {
            Ok(texts) => texts,
            Err(e) => {
                tracing::warn!("[RagProvider] 检索失败: {}", e);
                Vec::new()
            }
        }
    }
}

fn content_type_from_filename(filename: &str) -> String {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "md" | "markdown" => "text/markdown".to_string(),
        "txt" => "text/plain".to_string(),
        "pdf" => "application/pdf".to_string(),
        "html" | "htm" => "text/html".to_string(),
        "json" => "application/json".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}
