//! 文档索引

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 代码文档
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// 文档 ID
    pub id: String,
    /// 文件路径
    pub path: String,
    /// 内容
    pub content: String,
    /// 语言
    pub language: Option<String>,
    /// 元数据
    pub metadata: DocumentMetadata,
}

/// 文档元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 修改时间
    pub modified_at: DateTime<Utc>,
    /// 文件大小
    pub size_bytes: usize,
    /// 行数
    pub line_count: usize,
}

impl Document {
    /// 创建新文档
    pub fn new(id: String, path: String, content: String) -> Self {
        let now = Utc::now();
        let language = detect_language(&path);
        Self {
            id,
            path,
            content: content.clone(),
            language,
            metadata: DocumentMetadata {
                created_at: now,
                modified_at: now,
                size_bytes: content.len(),
                line_count: content.lines().count(),
            },
        }
    }
}

/// 代码块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// 块 ID
    pub id: String,
    /// 所属文档 ID
    pub document_id: String,
    /// 内容
    pub content: String,
    /// 起始行
    pub start_line: usize,
    /// 结束行
    pub end_line: usize,
    /// 块类型
    pub chunk_type: ChunkType,
    /// 符号信息（如果是函数/类等）
    pub symbol_info: Option<SymbolInfo>,
}

/// 块类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkType {
    /// 函数
    Function,
    /// 类
    Class,
    /// 方法
    Method,
    /// 模块
    Module,
    /// 文件（整个文件）
    File,
    /// 注释
    Comment,
    /// 代码块
    Code,
}

/// 符号信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    /// 符号名称
    pub name: String,
    /// 符号类型
    pub kind: String,
    /// 签名
    pub signature: Option<String>,
}

/// 向量索引
#[derive(Debug)]
pub struct VectorIndex {
    /// 文档存储
    documents: std::sync::RwLock<Vec<Document>>,
    /// 块存储
    chunks: std::sync::RwLock<Vec<Chunk>>,
    /// 向量存储 (简化实现：存储与块对应的向量)
    vectors: std::sync::RwLock<Vec<Vec<f32>>>,
    /// 维度
    dimension: usize,
}

impl Clone for VectorIndex {
    fn clone(&self) -> Self {
        Self {
            documents: std::sync::RwLock::new(self.documents.read().unwrap().clone()),
            chunks: std::sync::RwLock::new(self.chunks.read().unwrap().clone()),
            vectors: std::sync::RwLock::new(self.vectors.read().unwrap().clone()),
            dimension: self.dimension,
        }
    }
}

impl VectorIndex {
    /// 创建新的向量索引
    pub fn new(dimension: usize) -> Self {
        Self {
            documents: std::sync::RwLock::new(Vec::new()),
            chunks: std::sync::RwLock::new(Vec::new()),
            vectors: std::sync::RwLock::new(Vec::new()),
            dimension,
        }
    }

    /// 添加文档
    pub fn add_document(&self, doc: Document) {
        let mut documents = self.documents.write().unwrap();
        documents.push(doc);
    }

    /// 添加块
    pub fn add_chunk(&self, chunk: Chunk, vector: Vec<f32>) {
        let mut chunks = self.chunks.write().unwrap();
        let mut vectors = self.vectors.write().unwrap();

        assert_eq!(vector.len(), self.dimension, "向量维度不匹配");

        chunks.push(chunk);
        vectors.push(vector);
    }

    /// 获取文档
    pub fn get_document(&self, id: &str) -> Option<Document> {
        let documents = self.documents.read().unwrap();
        documents.iter().find(|d| d.id == id).cloned()
    }

    /// 获取块
    pub fn get_chunk(&self, id: &str) -> Option<Chunk> {
        let chunks = self.chunks.read().unwrap();
        chunks.iter().find(|c| c.id == id).cloned()
    }

    /// 获取块的数量
    pub fn chunk_count(&self) -> usize {
        self.chunks.read().unwrap().len()
    }

    /// 获取文档数量
    pub fn document_count(&self) -> usize {
        self.documents.read().unwrap().len()
    }

    /// 获取向量维度
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// 搜索最近邻（简化实现：余弦相似度）
    pub fn search(&self, query_vector: &[f32], k: usize) -> Vec<SearchHit> {
        let chunks = self.chunks.read().unwrap();
        let vectors = self.vectors.read().unwrap();

        if query_vector.len() != self.dimension || vectors.is_empty() {
            return Vec::new();
        }

        // 计算相似度
        let mut scores: Vec<(usize, f32)> = vectors
            .iter()
            .enumerate()
            .map(|(i, v)| (i, cosine_similarity(query_vector, v)))
            .collect();

        // 排序
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 取前 k 个
        scores
            .into_iter()
            .take(k)
            .filter_map(|(i, score)| {
                chunks.get(i).map(|chunk| SearchHit {
                    chunk: chunk.clone(),
                    score,
                })
            })
            .collect()
    }
}

/// 搜索命中
#[derive(Debug, Clone)]
pub struct SearchHit {
    /// 块
    pub chunk: Chunk,
    /// 相似度分数
    pub score: f32,
}

/// 计算余弦相似度
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        0.0
    } else {
        dot_product / (magnitude_a * magnitude_b)
    }
}

/// 根据文件扩展名检测语言
fn detect_language(path: &str) -> Option<String> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "rs" => Some("Rust".to_string()),
        "ts" | "tsx" => Some("TypeScript".to_string()),
        "js" | "jsx" => Some("JavaScript".to_string()),
        "py" => Some("Python".to_string()),
        "go" => Some("Go".to_string()),
        "java" => Some("Java".to_string()),
        "cpp" | "cc" | "cxx" | "c" | "h" | "hpp" => Some("C++".to_string()),
        "c" => Some("C".to_string()),
        "rb" => Some("Ruby".to_string()),
        "rs" => Some("Rust".to_string()),
        "sql" => Some("SQL".to_string()),
        "yaml" | "yml" => Some("YAML".to_string()),
        "json" => Some("JSON".to_string()),
        "toml" => Some("TOML".to_string()),
        "md" => Some("Markdown".to_string()),
        "html" | "htm" => Some("HTML".to_string()),
        "css" => Some("CSS".to_string()),
        _ => None,
    }
}