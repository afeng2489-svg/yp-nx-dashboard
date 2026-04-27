//! 代码搜索器

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{embedding::EmbeddingProvider, index::VectorIndex};

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 查询
    pub query: String,
    /// 结果
    pub hits: Vec<SearchHitResult>,
    /// 总结果数
    pub total_hits: usize,
    /// 搜索耗时（毫秒）
    pub search_time_ms: u64,
}

/// 搜索命中结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHitResult {
    /// 块 ID
    pub chunk_id: String,
    /// 文档 ID
    pub document_id: String,
    /// 文件路径
    pub file_path: String,
    /// 内容片段
    pub content_snippet: String,
    /// 起始行
    pub start_line: usize,
    /// 结束行
    pub end_line: usize,
    /// 相似度分数
    pub score: f32,
    /// 语言
    pub language: Option<String>,
}

/// 搜索选项
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// 最大结果数
    pub limit: usize,
    /// 最小分数阈值
    pub min_score: f32,
    /// 语言过滤器
    pub language_filter: Option<Vec<String>>,
    /// 文件路径过滤器（glob 模式）
    pub path_filter: Option<Vec<String>>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            min_score: 0.5,
            language_filter: None,
            path_filter: None,
        }
    }
}

/// 代码搜索器
pub struct CodeSearcher {
    /// 向量索引
    index: VectorIndex,
    /// 默认搜索选项
    default_options: SearchOptions,
    /// Embedding 提供者（可选）
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    /// 向量维度
    #[allow(dead_code)]
    dimension: usize,
}

impl CodeSearcher {
    /// 创建新的代码搜索器
    pub fn new(dimension: usize) -> Self {
        Self {
            index: VectorIndex::new(dimension),
            default_options: SearchOptions::default(),
            embedding_provider: None,
            dimension,
        }
    }

    /// 使用 embedding 提供者创建代码搜索器
    pub fn with_embedding_provider(mut self, provider: Arc<dyn EmbeddingProvider>) -> Self {
        self.embedding_provider = Some(provider);
        self
    }

    /// 设置 embedding 提供者
    pub fn set_embedding_provider(&mut self, provider: Arc<dyn EmbeddingProvider>) {
        self.embedding_provider = Some(provider);
    }

    /// 搜索
    pub fn search(&self, query: &str, options: Option<SearchOptions>) -> SearchResult {
        let start = std::time::Instant::now();
        let opts = options.unwrap_or_else(|| self.default_options.clone());

        let query_vector = self.generate_embedding(query);

        let hits = self.index.search(&query_vector, opts.limit);

        let results: Vec<SearchHitResult> = hits
            .into_iter()
            .filter(|hit| hit.score >= opts.min_score)
            .map(|hit| {
                let doc = self.index.get_document(&hit.chunk.document_id);
                SearchHitResult {
                    chunk_id: hit.chunk.id,
                    document_id: hit.chunk.document_id,
                    file_path: doc.as_ref().map(|d| d.path.clone()).unwrap_or_default(),
                    content_snippet: truncate_content(&hit.chunk.content, 200),
                    start_line: hit.chunk.start_line,
                    end_line: hit.chunk.end_line,
                    score: hit.score,
                    language: doc.and_then(|d| d.language.clone()),
                }
            })
            .filter(|result| {
                // 应用语言过滤器
                if let Some(ref languages) = opts.language_filter {
                    if let Some(ref lang) = result.language {
                        if !languages.contains(lang) {
                            return false;
                        }
                    }
                }
                true
            })
            .collect();

        let total_hits = results.len();

        SearchResult {
            query: query.to_string(),
            hits: results,
            total_hits,
            search_time_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// 生成查询 embedding
    ///
    /// 优先使用配置的 embedding 提供者，如果不可用则回退到模拟实现
    fn generate_embedding(&self, text: &str) -> Vec<f32> {
        if let Some(ref provider) = self.embedding_provider {
            // 使用 tokio runtime 执行异步 embedding
            let rt = tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime for embedding");

            match rt.block_on(provider.embed(text)) {
                Ok(result) => result.vector,
                Err(e) => {
                    tracing::warn!(
                        "Embedding provider '{}' failed, using mock: {}",
                        provider.name(),
                        e
                    );
                    self.mock_embed(text)
                }
            }
        } else {
            tracing::debug!("No embedding provider configured, using mock");
            self.mock_embed(text)
        }
    }

    /// 模拟 embedding（仅作为 fallback）
    fn mock_embed(&self, text: &str) -> Vec<f32> {
        // 生成随机向量作为占位
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let seed = hasher.finish();

        let mut rng = SimpleRng::new(seed);
        (0..self.index.dimension())
            .map(|_| rng.next_f32())
            .collect()
    }
}

/// 简单的 RNG（用于模拟）
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_f32(&mut self) -> f32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.state >> 33) as f32 / u32::MAX as f32
    }
}

/// 截断内容
fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        content.to_string()
    } else {
        format!("{}...", &content[..max_len])
    }
}
