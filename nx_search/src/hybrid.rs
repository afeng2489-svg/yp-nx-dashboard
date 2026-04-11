//! Hybrid Search 混合搜索
//!
//! 结合 ACE 语义搜索和 CodexLens FTS 的混合搜索模式,
//! 根据查询类型自动选择最佳搜索策略。

use serde::{Deserialize, Serialize};
use crate::ace::{AceEngine, AceSearchResult, AceSearchHit, AceSearchMode};
use crate::codexlens::{CodexLensEngine, CodexLensResult, CodexLensHit, CodexLensConfig};
use crate::index::VectorIndex;

/// 混合搜索配置
#[derive(Debug, Clone)]
pub struct HybridConfig {
    /// ACE 配置
    pub ace_config: crate::ace::AceConfig,
    /// CodexLens 配置
    pub codexlens_config: CodexLensConfig,
    /// 混合搜索模式
    pub mode: HybridSearchMode,
    /// 语义搜索权重
    pub semantic_weight: f32,
    /// 关键词搜索权重
    pub keyword_weight: f32,
    /// 是否自动选择搜索模式
    pub auto_select_mode: bool,
    /// 自动选择阈值
    pub auto_select_threshold: f32,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            ace_config: crate::ace::AceConfig::default(),
            codexlens_config: CodexLensConfig::default(),
            mode: HybridSearchMode::Auto,
            semantic_weight: 0.5,
            keyword_weight: 0.5,
            auto_select_mode: true,
            auto_select_threshold: 0.3,
        }
    }
}

/// 混合搜索模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HybridSearchMode {
    /// 自动选择最佳模式
    Auto,
    /// 仅语义搜索
    SemanticOnly,
    /// 仅关键词搜索
    KeywordOnly,
    /// 混合搜索
    Hybrid,
    /// 以语义为主
    SemanticFirst,
    /// 以关键词为主
    KeywordFirst,
}

impl Default for HybridSearchMode {
    fn default() -> Self {
        HybridSearchMode::Auto
    }
}

/// 混合搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResult {
    /// 原始查询
    pub query: String,
    /// 结果列表
    pub results: Vec<HybridSearchHit>,
    /// 总结果数
    pub total_hits: usize,
    /// 搜索耗时（毫秒）
    pub search_time_ms: u64,
    /// 使用的模式
    pub mode_used: HybridSearchMode,
    /// 语义搜索结果
    pub semantic_result: Option<AceSearchResult>,
    /// 关键词搜索结果
    pub keyword_result: Option<CodexLensResult>,
    /// 混合分数详情
    pub score_breakdown: ScoreBreakdown,
}

/// 混合搜索命中
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchHit {
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
    /// 综合分数
    pub score: f32,
    /// 语义分数
    pub semantic_score: f32,
    /// 关键词分数
    pub keyword_score: f32,
    /// 语言
    pub language: Option<String>,
    /// 匹配类型
    pub match_types: Vec<MatchType>,
}

/// 匹配类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    /// 语义匹配
    Semantic,
    /// 精确匹配
    Exact,
    /// 前缀匹配
    Prefix,
    /// 模糊匹配
    Fuzzy,
}

/// 分数明细
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// 总分数
    pub total_score: f32,
    /// 语义权重
    pub semantic_weight: f32,
    /// 关键词权重
    pub keyword_weight: f32,
    /// 结果数量
    pub semantic_count: usize,
    /// 关键词结果数量
    pub keyword_count: usize,
}

/// 混合搜索引擎
pub struct HybridSearchEngine {
    /// ACE 引擎
    ace_engine: AceEngine,
    /// CodexLens 引擎
    codexlens_engine: CodexLensEngine,
    /// 配置
    config: HybridConfig,
}

impl HybridSearchEngine {
    /// 创建新的混合搜索引擎
    pub fn new(dimension: usize) -> Self {
        Self {
            ace_engine: AceEngine::new(dimension),
            codexlens_engine: CodexLensEngine::new(),
            config: HybridConfig::default(),
        }
    }

    /// 创建带配置的引擎
    pub fn with_config(config: HybridConfig, dimension: usize) -> Self {
        Self {
            ace_engine: AceEngine::with_config(config.ace_config.clone(), dimension),
            codexlens_engine: CodexLensEngine::with_config(config.codexlens_config.clone()),
            config,
        }
    }

    /// 获取 ACE 引擎引用
    pub fn ace_engine(&self) -> &AceEngine {
        &self.ace_engine
    }

    /// 获取 CodexLens 引擎引用
    pub fn codexlens_engine(&self) -> &CodexLensEngine {
        &self.codexlens_engine
    }

    /// 获取配置
    pub fn config(&self) -> &HybridConfig {
        &self.config
    }

    /// 更新配置
    pub fn update_config(&mut self, config: HybridConfig) {
        let ace_config = config.ace_config.clone();
        self.config = config;
        self.ace_engine.update_config(ace_config);
    }

    /// 索引文档（同时用于语义和关键词搜索）
    pub fn index_document(&mut self, id: String, path: String, content: String, language: Option<String>, vectors: Vec<Vec<f32>>, chunks: Vec<crate::index::Chunk>) {
        // 索引到 CodexLens（关键词搜索）
        self.codexlens_engine.index_document(id.clone(), path.clone(), content.clone(), language.clone());

        // 索引到 ACE（语义搜索）
        let doc = crate::index::Document::new(id.clone(), path, content);
        self.ace_engine.index_document(doc, chunks, vectors);
    }

    /// 搜索
    pub async fn search(&self, query: &str) -> Result<HybridSearchResult, HybridError> {
        let start = std::time::Instant::now();

        // 自动选择模式
        let mode = if self.config.auto_select_mode {
            self.detect_search_mode(query)
        } else {
            self.config.mode
        };

        // 根据模式执行搜索
        let (semantic_result, keyword_result) = match mode {
            HybridSearchMode::SemanticOnly | HybridSearchMode::SemanticFirst => {
                let semantic = Some(self.ace_engine.semantic_search(query).await?);
                let keyword = if mode == HybridSearchMode::SemanticFirst {
                    Some(self.codexlens_engine.search(query, None))
                } else {
                    None
                };
                (semantic, keyword)
            }
            HybridSearchMode::KeywordOnly | HybridSearchMode::KeywordFirst => {
                let keyword = self.codexlens_engine.search(query, None);
                let semantic = if mode == HybridSearchMode::KeywordFirst {
                    Some(self.ace_engine.semantic_search(query).await?)
                } else {
                    None
                };
                (semantic, Some(keyword))
            }
            HybridSearchMode::Hybrid => {
                // 并行执行两种搜索
                let semantic = self.ace_engine.semantic_search(query).await?;
                let keyword = self.codexlens_engine.search(query, None);
                (Some(semantic), Some(keyword))
            }
            HybridSearchMode::Auto => {
                // 根据查询特征决定
                let query_analysis = self.analyze_query(query);

                if query_analysis.has_code_syntax || query_analysis.is_structural {
                    // 代码相关查询，使用语义搜索
                    (Some(self.ace_engine.semantic_search(query).await?), None)
                } else if query_analysis.is_exact_phrase {
                    // 精确短语，使用关键词搜索
                    (None, Some(self.codexlens_engine.search(query, None)))
                } else {
                    // 默认使用混合搜索
                    let semantic = self.ace_engine.semantic_search(query).await?;
                    let keyword = self.codexlens_engine.search(query, None);
                    (Some(semantic), Some(keyword))
                }
            }
        };

        // 合并结果
        let (results, score_breakdown) = self.merge_results(semantic_result.as_ref(), keyword_result.as_ref());
        let total_hits = results.len();

        let search_time_ms = start.elapsed().as_millis() as u64;

        Ok(HybridSearchResult {
            query: query.to_string(),
            results,
            total_hits,
            search_time_ms,
            mode_used: mode,
            semantic_result,
            keyword_result,
            score_breakdown,
        })
    }

    /// 检测搜索模式
    fn detect_search_mode(&self, query: &str) -> HybridSearchMode {
        let analysis = self.analyze_query(query);

        if analysis.has_code_syntax && analysis.is_structural {
            HybridSearchMode::SemanticFirst
        } else if analysis.is_exact_phrase {
            HybridSearchMode::KeywordOnly
        } else if analysis.has_wildcard || analysis.is_regex {
            HybridSearchMode::KeywordOnly
        } else {
            HybridSearchMode::Hybrid
        }
    }

    /// 分析查询
    fn analyze_query(&self, query: &str) -> QueryAnalysis {
        let query_lower = query.to_lowercase();

        // 检测代码语法
        let code_indicators = [
            "fn ", "function ", "def ", "class ", "struct ", "enum ",
            "impl ", "pub ", "private ", "public ", "static ",
            "import ", "use ", "require ", "include ",
            "if ", "else ", "for ", "while ", "loop ",
            "->", "=>", "::", "=>", ".", "(",
        ];
        let has_code_syntax = code_indicators.iter().any(|&ind| query.contains(ind));

        // 检测结构化查询
        let structural_indicators = ["继承", "implements", "extends", "trait", "interface"];
        let is_structural = structural_indicators.iter().any(|&ind| query_lower.contains(ind));

        // 检测精确短语
        let is_exact_phrase = query.contains('"') || query.contains('\'');

        // 检测通配符
        let has_wildcard = query.contains('*') || query.contains('?');

        // 检测正则
        let is_regex = query.starts_with('/') && query.ends_with('/');

        QueryAnalysis {
            has_code_syntax,
            is_structural,
            is_exact_phrase,
            has_wildcard,
            is_regex,
        }
    }

    /// 合并搜索结果
    fn merge_results(&self, semantic: Option<&AceSearchResult>, keyword: Option<&CodexLensResult>) -> (Vec<HybridSearchHit>, ScoreBreakdown) {
        let mut hit_map: HashMap<String, HybridSearchHit> = HashMap::new();

        let mut semantic_count = 0;
        let mut keyword_count = 0;

        // 处理语义搜索结果
        if let Some(sem) = semantic {
            semantic_count = sem.results.len();
            for hit in &sem.results {
                let combined_score = hit.semantic_score * self.config.semantic_weight;
                hit_map.insert(hit.document_id.clone(), HybridSearchHit {
                    document_id: hit.document_id.clone(),
                    file_path: hit.file_path.clone(),
                    content: hit.content.clone(),
                    start_line: hit.start_line,
                    end_line: hit.end_line,
                    score: combined_score,
                    semantic_score: hit.semantic_score,
                    keyword_score: 0.0,
                    language: hit.language.clone(),
                    match_types: vec![MatchType::Semantic],
                });
            }
        }

        // 处理关键词搜索结果
        if let Some(kw) = keyword {
            keyword_count = kw.results.len();
            for hit in &kw.results {
                let keyword_score = hit.score * self.config.keyword_weight;

                if let Some(existing) = hit_map.get_mut(&hit.document_id) {
                    // 合并分数
                    existing.score += keyword_score;
                    existing.keyword_score = hit.score;
                    existing.match_types.push(MatchType::Exact);
                } else {
                    hit_map.insert(hit.document_id.clone(), HybridSearchHit {
                        document_id: hit.document_id.clone(),
                        file_path: hit.file_path.clone(),
                        content: hit.snippet.clone(),
                        start_line: hit.line_number,
                        end_line: hit.line_number,
                        score: keyword_score,
                        semantic_score: 0.0,
                        keyword_score: hit.score,
                        language: hit.language.clone(),
                        match_types: vec![MatchType::Exact],
                    });
                }
            }
        }

        // 排序
        let mut results: Vec<HybridSearchHit> = hit_map.into_values().collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        let score_breakdown = ScoreBreakdown {
            total_score: results.iter().map(|h| h.score).sum(),
            semantic_weight: self.config.semantic_weight,
            keyword_weight: self.config.keyword_weight,
            semantic_count,
            keyword_count,
        };

        (results, score_breakdown)
    }
}

/// 查询分析结果
#[derive(Debug, Clone, Default)]
struct QueryAnalysis {
    has_code_syntax: bool,
    is_structural: bool,
    is_exact_phrase: bool,
    has_wildcard: bool,
    is_regex: bool,
}

/// 混合搜索错误
#[derive(Debug, thiserror::Error)]
pub enum HybridError {
    #[error("语义搜索错误: {0}")]
    SemanticError(String),

    #[error("关键词搜索错误: {0}")]
    KeywordError(String),

    #[error("合并结果错误: {0}")]
    MergeError(String),
}

impl From<crate::ace::AceError> for HybridError {
    fn from(err: crate::ace::AceError) -> Self {
        HybridError::SemanticError(err.to_string())
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_config_default() {
        let config = HybridConfig::default();
        assert_eq!(config.semantic_weight, 0.5);
        assert!(config.auto_select_mode);
    }

    #[test]
    fn test_query_analysis() {
        let engine = HybridSearchEngine::new(128);
        let query = "fn main()";
        let analysis = engine.analyze_query(query);
        assert!(analysis.has_code_syntax);
    }

    #[test]
    fn test_detect_search_mode() {
        let engine = HybridSearchEngine::new(128);
        let mode = engine.detect_search_mode("fn main() -> ()");
        assert_eq!(mode, HybridSearchMode::SemanticFirst);
    }
}
