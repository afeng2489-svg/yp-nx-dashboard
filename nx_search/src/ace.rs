//! ACE (Augment Context Engine) 语义搜索
//!
//! 提供基于语义理解的代码搜索能力,通过 embedding 向量匹配找到语义相似的代码片段。

use serde::{Deserialize, Serialize};
use crate::index::{VectorIndex, Chunk, Document};
use crate::embedding::EmbeddingProvider;

/// ACE 搜索配置
#[derive(Debug, Clone)]
pub struct AceConfig {
    /// 最大返回结果数
    pub max_results: usize,
    /// 最小相似度分数
    pub min_score: f32,
    /// 是否启用 reranking
    pub enable_rerank: bool,
    /// reranking 参数
    pub rerank_top_k: usize,
    /// 上下文窗口大小
    pub context_window: usize,
    /// 是否包含文档元数据
    pub include_metadata: bool,
}

impl Default for AceConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            min_score: 0.5,
            enable_rerank: true,
            rerank_top_k: 20,
            context_window: 512,
            include_metadata: true,
        }
    }
}

/// ACE 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AceSearchResult {
    /// 查询
    pub query: String,
    /// 结果列表
    pub results: Vec<AceSearchHit>,
    /// 总结果数
    pub total_hits: usize,
    /// 搜索耗时（毫秒）
    pub search_time_ms: u64,
    /// 搜索模式
    pub mode: AceSearchMode,
}

/// ACE 搜索命中
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AceSearchHit {
    /// 块 ID
    pub chunk_id: String,
    /// 文档 ID
    pub document_id: String,
    /// 文件路径
    pub file_path: String,
    /// 内容片段
    pub content: String,
    /// 起始行
    pub start_line: usize,
    /// 结束行
    pub end_line: usize,
    /// 语义相似度分数
    pub semantic_score: f32,
    /// 相关性分数（reranking 后）
    pub relevance_score: Option<f32>,
    /// 语言
    pub language: Option<String>,
    /// 符号信息
    pub symbol_info: Option<SymbolContext>,
}

/// 符号上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolContext {
    /// 符号名称
    pub name: String,
    /// 符号类型
    pub kind: String,
    /// 签名
    pub signature: Option<String>,
    /// 父符号
    pub parent: Option<String>,
    /// 子符号
    pub children: Vec<String>,
}

/// ACE 搜索模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AceSearchMode {
    /// 语义搜索
    Semantic,
    /// 关键词搜索
    Keyword,
    /// 混合搜索
    Hybrid,
    /// 代码结构搜索
    Structural,
}

impl Default for AceSearchMode {
    fn default() -> Self {
        AceSearchMode::Semantic
    }
}

/// ACE 语义搜索引擎
pub struct AceEngine {
    /// 向量索引
    index: VectorIndex,
    /// Embedding 提供者
    embedding_provider: Option<Box<dyn EmbeddingProvider>>,
    /// 配置
    config: AceConfig,
}

impl AceEngine {
    /// 创建新的 ACE 引擎
    pub fn new(dimension: usize) -> Self {
        Self {
            index: VectorIndex::new(dimension),
            embedding_provider: None,
            config: AceConfig::default(),
        }
    }

    /// 创建带配置的 ACE 引擎
    pub fn with_config(config: AceConfig, dimension: usize) -> Self {
        Self {
            index: VectorIndex::new(dimension),
            embedding_provider: None,
            config,
        }
    }

    /// 设置 embedding 提供者
    pub fn set_embedding_provider(&mut self, provider: Box<dyn EmbeddingProvider>) {
        self.embedding_provider = Some(provider);
    }

    /// 获取配置
    pub fn config(&self) -> &AceConfig {
        &self.config
    }

    /// 更新配置
    pub fn update_config(&mut self, config: AceConfig) {
        self.config = config;
    }

    /// 语义搜索
    pub async fn semantic_search(&self, query: &str) -> Result<AceSearchResult, AceError> {
        let start = std::time::Instant::now();

        // 生成查询 embedding
        let query_vector = self.generate_query_embedding(query).await?;

        // 搜索向量索引
        let hits = self.index.search(&query_vector, self.config.rerank_top_k);

        // 构建结果
        let results: Vec<AceSearchHit> = hits
            .into_iter()
            .filter(|hit| hit.score >= self.config.min_score)
            .take(self.config.max_results)
            .map(|hit| {
                let doc = self.index.get_document(&hit.chunk.document_id);
                AceSearchHit {
                    chunk_id: hit.chunk.id.clone(),
                    document_id: hit.chunk.document_id.clone(),
                    file_path: doc.as_ref().map(|d| d.path.clone()).unwrap_or_default(),
                    content: hit.chunk.content.clone(),
                    start_line: hit.chunk.start_line,
                    end_line: hit.chunk.end_line,
                    semantic_score: hit.score,
                    relevance_score: None,
                    language: doc.and_then(|d| d.language.clone()),
                    symbol_info: hit.chunk.symbol_info.as_ref().map(|s| SymbolContext {
                        name: s.name.clone(),
                        kind: s.kind.clone(),
                        signature: s.signature.clone(),
                        parent: None,
                        children: Vec::new(),
                    }),
                }
            })
            .collect();

        let total_hits = results.len();

        Ok(AceSearchResult {
            query: query.to_string(),
            results,
            total_hits,
            search_time_ms: start.elapsed().as_millis() as u64,
            mode: AceSearchMode::Semantic,
        })
    }

    /// 生成查询 embedding
    async fn generate_query_embedding(&self, query: &str) -> Result<Vec<f32>, AceError> {
        if let Some(ref provider) = self.embedding_provider {
            let result = provider.embed(query).await
                .map_err(|e| AceError::EmbeddingError(e.to_string()))?;
            Ok(result.vector)
        } else {
            // 使用模拟 embedding
            Ok(self.mock_embed(query))
        }
    }

    /// 模拟 embedding
    fn mock_embed(&self, text: &str) -> Vec<f32> {
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

    /// 添加文档进行索引
    pub fn index_document(&self, doc: Document, chunks: Vec<Chunk>, vectors: Vec<Vec<f32>>) {
        self.index.add_document(doc);
        for (chunk, vector) in chunks.into_iter().zip(vectors.into_iter()) {
            self.index.add_chunk(chunk, vector);
        }
    }
}

/// ACE 错误类型
#[derive(Debug, thiserror::Error)]
pub enum AceError {
    #[error("Embedding 错误: {0}")]
    EmbeddingError(String),

    #[error("索引错误: {0}")]
    IndexError(String),

    #[error("搜索错误: {0}")]
    SearchError(String),

    #[error("不支持的操作: {0}")]
    Unsupported(String),
}

/// 简单的 RNG
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

/// 构建符号上下文图
pub struct SymbolGraph {
    /// 节点: 符号 ID -> 符号信息
    nodes: std::sync::RwLock<std::collections::HashMap<String, SymbolNode>>,
}

/// 符号节点
#[derive(Debug, Clone)]
pub struct SymbolNode {
    /// 符号 ID
    pub id: String,
    /// 符号名称
    pub name: String,
    /// 符号类型
    pub kind: String,
    /// 文档 ID
    pub document_id: String,
    /// 块 ID
    pub chunk_id: String,
    /// 父节点 ID
    pub parent_id: Option<String>,
    /// 子节点 ID
    pub children: Vec<String>,
}

impl SymbolGraph {
    /// 创建新的符号图
    pub fn new() -> Self {
        Self {
            nodes: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// 添加符号节点
    pub fn add_symbol(&self, node: SymbolNode) {
        let mut nodes = self.nodes.write().unwrap();
        nodes.insert(node.id.clone(), node);
    }

    /// 获取符号
    pub fn get_symbol(&self, id: &str) -> Option<SymbolNode> {
        let nodes = self.nodes.read().unwrap();
        nodes.get(id).cloned()
    }

    /// 获取子符号
    pub fn get_children(&self, id: &str) -> Vec<SymbolNode> {
        let nodes = self.nodes.read().unwrap();
        if let Some(node) = nodes.get(id) {
            node.children.iter()
                .filter_map(|child_id| nodes.get(child_id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 获取父符号
    pub fn get_parent(&self, id: &str) -> Option<SymbolNode> {
        let nodes = self.nodes.read().unwrap();
        if let Some(node) = nodes.get(id) {
            node.parent_id.as_ref()
                .and_then(|pid| nodes.get(pid).cloned())
        } else {
            None
        }
    }

    /// 搜索符号
    pub fn search_symbols(&self, query: &str) -> Vec<SymbolNode> {
        let nodes = self.nodes.read().unwrap();
        let query_lower = query.to_lowercase();

        nodes.values()
            .filter(|node| {
                node.name.to_lowercase().contains(&query_lower) ||
                node.kind.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }
}

impl Default for SymbolGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ace_config_default() {
        let config = AceConfig::default();
        assert_eq!(config.max_results, 10);
        assert_eq!(config.min_score, 0.5);
        assert!(config.enable_rerank);
    }

    #[tokio::test]
    async fn test_ace_semantic_search() {
        let engine = AceEngine::new(128);
        let result = engine.semantic_search("test query").await.unwrap();
        assert_eq!(result.mode, AceSearchMode::Semantic);
    }

    #[test]
    fn test_symbol_graph() {
        let graph = SymbolGraph::new();
        graph.add_symbol(SymbolNode {
            id: "1".to_string(),
            name: "TestFunction".to_string(),
            kind: "function".to_string(),
            document_id: "doc1".to_string(),
            chunk_id: "chunk1".to_string(),
            parent_id: None,
            children: Vec::new(),
        });

        let node = graph.get_symbol("1").unwrap();
        assert_eq!(node.name, "TestFunction");
    }
}
