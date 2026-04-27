//! BM25 关键词搜索引擎
//!
//! 纯内存实现，零 API 调用，零 Token 消耗
//!
//! BM25 (Best Matching 25) 是一种经典的文本检索算法，
//! 用于衡量文档与查询之间的相关性评分。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// BM25 配置参数
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Bm25Config {
    /// k1 参数（词频饱和度）
    pub k1: f32,
    /// b 参数（文档长度归一化）
    pub b: f32,
    /// 平均文档长度
    pub avg_doc_len: f32,
}

impl Default for Bm25Config {
    fn default() -> Self {
        Self {
            k1: 1.2,
            b: 0.75,
            avg_doc_len: 100.0, // 假设平均 100 tokens
        }
    }
}

/// BM25 索引文档
#[derive(Debug, Clone)]
struct IndexDocument {
    id: String,
    content: String,
    terms: HashMap<String, u32>, // term -> frequency
    term_count: usize,
    metadata: serde_json::Value,
}

/// BM25 搜索结果
#[derive(Debug, Clone)]
pub struct Bm25Result {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub metadata: serde_json::Value,
}

/// BM25 搜索引擎
pub struct Bm25Index {
    config: Bm25Config,
    documents: HashMap<String, IndexDocument>,
    doc_frequency: HashMap<String, usize>, // term -> number of docs containing term
    total_docs: usize,
}

impl Bm25Index {
    /// 创建新的 BM25 索引
    pub fn new() -> Self {
        Self {
            config: Bm25Config::default(),
            documents: HashMap::new(),
            doc_frequency: HashMap::new(),
            total_docs: 0,
        }
    }

    /// 创建带配置的 BM25 索引
    pub fn with_config(config: Bm25Config) -> Self {
        Self {
            config,
            documents: HashMap::new(),
            doc_frequency: HashMap::new(),
            total_docs: 0,
        }
    }

    /// 添加文档到索引
    pub fn add_document(
        &mut self,
        id: impl Into<String>,
        content: impl Into<String>,
        metadata: Option<serde_json::Value>,
    ) {
        let id = id.into();
        let content = content.into();
        let terms = self.tokenize(&content);
        let term_count = terms.len();

        // 统计词频
        let mut term_freq = HashMap::new();
        for term in &terms {
            *term_freq.entry(term.clone()).or_insert(0) += 1;
        }

        // 更新文档频率
        for term in term_freq.keys() {
            *self.doc_frequency.entry(term.clone()).or_insert(0) += 1;
        }

        let doc = IndexDocument {
            id: id.clone(),
            content: content.clone(),
            terms: term_freq,
            term_count,
            metadata: metadata.unwrap_or(serde_json::json!({})),
        };

        self.documents.insert(id, doc);
        self.total_docs += 1;

        // 自动更新平均文档长度
        if self.total_docs > 0 {
            let total_terms: usize = self.documents.values().map(|d| d.term_count).sum();
            self.config.avg_doc_len = total_terms as f32 / self.total_docs as f32;
        }
    }

    /// 移除文档
    pub fn remove_document(&mut self, id: &str) -> bool {
        if let Some(doc) = self.documents.remove(id) {
            // 更新文档频率
            for term in doc.terms.keys() {
                if let Some(count) = self.doc_frequency.get_mut(term) {
                    if *count > 1 {
                        *count -= 1;
                    } else {
                        self.doc_frequency.remove(term);
                    }
                }
            }
            self.total_docs -= 1;

            // 自动更新平均文档长度
            if self.total_docs > 0 {
                let total_terms: usize = self.documents.values().map(|d| d.term_count).sum();
                self.config.avg_doc_len = total_terms as f32 / self.total_docs as f32;
            } else {
                self.config.avg_doc_len = 100.0;
            }

            true
        } else {
            false
        }
    }

    /// 搜索文档
    pub fn search(&self, query: &str, top_k: usize) -> Vec<Bm25Result> {
        if self.documents.is_empty() {
            return Vec::new();
        }

        let query_terms = self.tokenize(query);
        if query_terms.is_empty() {
            return Vec::new();
        }

        let mut scores: Vec<(String, f32)> = Vec::new();

        for (id, doc) in &self.documents {
            let mut score = 0.0f32;

            for term in &query_terms {
                let tf = doc.terms.get(term).copied().unwrap_or(0);
                if tf == 0 {
                    continue;
                }

                // 计算 IDF
                let df = self.doc_frequency.get(term).copied().unwrap_or(0);
                let idf = if df > 0 {
                    let n = self.total_docs as f32;
                    let df = df as f32;
                    ((n - df + 0.5) / (df + 0.5) + 1.0).ln()
                } else {
                    0.0
                };

                // BM25 公式
                let numerator = tf as f32 * (self.config.k1 + 1.0);
                let denominator = tf as f32
                    + self.config.k1
                        * (1.0 - self.config.b
                            + self.config.b * doc.term_count as f32 / self.config.avg_doc_len);

                score += idf * numerator / denominator;
            }

            if score > 0.0 {
                scores.push((id.clone(), score));
            }
        }

        // 按分数排序
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 转换为结果
        scores
            .into_iter()
            .take(top_k)
            .filter_map(|(id, score)| {
                self.documents.get(&id).map(|doc| Bm25Result {
                    id: doc.id.clone(),
                    content: doc.content.clone(),
                    score,
                    metadata: doc.metadata.clone(),
                })
            })
            .collect()
    }

    /// 获取文档数量
    pub fn len(&self) -> usize {
        self.total_docs
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// 清空索引
    pub fn clear(&mut self) {
        self.documents.clear();
        self.doc_frequency.clear();
        self.total_docs = 0;
    }

    /// 获取指定 ID 的文档
    pub fn get(&self, id: &str) -> Option<&IndexDocument> {
        self.documents.get(id)
    }

    /// 获取所有文档 ID
    pub fn ids(&self) -> Vec<String> {
        self.documents.keys().cloned().collect()
    }

    /// 更新平均文档长度
    pub fn update_avg_doc_len(&mut self, avg_len: f32) {
        self.config.avg_doc_len = avg_len;
    }

    /// 分词
    fn tokenize(&self, text: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        for word in text.to_lowercase().split_whitespace() {
            // 检查是否包含中文
            let has_chinese = word.chars().any(|c| c.len_utf8() > 1);
            let filtered: String = word
                .chars()
                .filter(|c| c.is_alphanumeric() || c.len_utf8() > 1)
                .collect();

            for term in filtered.split(|c: char| !c.is_alphanumeric() && c.len_utf8() == 1) {
                let lower = term.to_lowercase();
                if lower.is_empty() {
                    continue;
                }

                if has_chinese {
                    // 中文：使用 bigram 分词（每两个连续字符作为一个词）
                    let chars: Vec<char> = lower.chars().collect();
                    if chars.len() >= 2 {
                        for i in 0..chars.len() - 1 {
                            tokens.push(format!("{}{}", chars[i], chars[i + 1]));
                        }
                    } else if chars.len() == 1 {
                        tokens.push(lower);
                    }
                } else if lower.len() > 1 {
                    // 英文：保留长度 > 1 的词
                    tokens.push(lower);
                }
            }
        }
        tokens
    }
}

impl Default for Bm25Index {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bm25_basic() {
        let mut index = Bm25Index::new();

        index.add_document("1", "PostgreSQL is a database", None);
        index.add_document("2", "Redis is a cache", None);
        index.add_document("3", "PostgreSQL and Redis together", None);

        let results = index.search("PostgreSQL", 3);
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.id == "1"));
        assert!(results.iter().any(|r| r.id == "3"));
    }

    #[test]
    fn test_bm25_empty() {
        let index = Bm25Index::new();
        let results = index.search("test", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_bm25_tokenize() {
        let mut index = Bm25Index::new();
        index.add_document("1", "Hello World 你好世界", None);

        let results = index.search("world", 10);
        assert!(!results.is_empty());
    }
}
