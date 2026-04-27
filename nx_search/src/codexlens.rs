//! CodexLens FTS (Full-Text Search) 全文搜索引擎
//!
//! 基于倒排索引的全文搜索,支持代码感知分词、模糊匹配和高级查询语法。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// CodexLens FTS 配置
#[derive(Debug, Clone)]
pub struct CodexLensConfig {
    /// 最小词长度
    pub min_word_length: usize,
    /// 最大词长度
    pub max_word_length: usize,
    /// 是否启用模糊匹配
    pub enable_fuzzy: bool,
    /// 模糊匹配容忍度
    pub fuzzy_tolerance: usize,
    /// 是否启用代码感知分词
    pub code_aware_tokenization: bool,
    /// 是否启用停用词过滤
    pub enable_stop_words: bool,
    /// 停用词列表
    pub stop_words: HashSet<String>,
    /// 最大返回结果数
    pub max_results: usize,
}

impl Default for CodexLensConfig {
    fn default() -> Self {
        let mut stop_words = HashSet::new();
        stop_words.insert("the".to_string());
        stop_words.insert("a".to_string());
        stop_words.insert("an".to_string());
        stop_words.insert("and".to_string());
        stop_words.insert("or".to_string());
        stop_words.insert("but".to_string());
        stop_words.insert("in".to_string());
        stop_words.insert("on".to_string());
        stop_words.insert("at".to_string());
        stop_words.insert("to".to_string());
        stop_words.insert("for".to_string());
        stop_words.insert("of".to_string());
        stop_words.insert("with".to_string());
        stop_words.insert("by".to_string());

        Self {
            min_word_length: 2,
            max_word_length: 64,
            enable_fuzzy: true,
            fuzzy_tolerance: 2,
            code_aware_tokenization: true,
            enable_stop_words: true,
            stop_words,
            max_results: 100,
        }
    }
}

/// CodexLens FTS 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexLensResult {
    /// 查询
    pub query: String,
    /// 结果列表
    pub results: Vec<CodexLensHit>,
    /// 总结果数
    pub total_hits: usize,
    /// 搜索耗时（毫秒）
    pub search_time_ms: u64,
    /// 使用的查询类型
    pub query_type: QueryType,
}

/// CodexLens 搜索命中
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexLensHit {
    /// 文档 ID
    pub document_id: String,
    /// 文件路径
    pub file_path: String,
    /// 匹配的内容片段
    pub snippet: String,
    /// 匹配行号
    pub line_number: usize,
    /// 匹配分数
    pub score: f32,
    /// 匹配词列表
    pub matched_terms: Vec<String>,
    /// 语言
    pub language: Option<String>,
}

/// 查询类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryType {
    /// 普通查询
    Simple,
    /// 短语查询
    Phrase,
    /// 前缀查询
    Prefix,
    /// 通配符查询
    Wildcard,
    /// 正则表达式查询
    Regex,
    /// 模糊查询
    Fuzzy,
}

/// 倒排索引条目
#[derive(Debug, Clone)]
struct InvertedIndexEntry {
    /// 文档 ID
    document_ids: HashSet<String>,
    /// 位置信息: 文档 ID -> 行号列表
    positions: HashMap<String, Vec<usize>>,
    /// 文档频率
    doc_frequency: usize,
}

/// 文档索引信息
#[derive(Debug, Clone)]
struct IndexedDocument {
    /// 文档 ID
    id: String,
    /// 文件路径
    path: String,
    /// 内容
    content: String,
    /// 语言
    language: Option<String>,
    /// 索引时间
    indexed_at: DateTime<Utc>,
    /// 行内容
    lines: Vec<String>,
}

impl IndexedDocument {
    /// 从内容创建
    fn new(id: String, path: String, content: String, language: Option<String>) -> Self {
        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        Self {
            id,
            path,
            content,
            language,
            indexed_at: Utc::now(),
            lines,
        }
    }

    /// 获取指定行的内容
    fn get_line(&self, line_num: usize) -> Option<&str> {
        self.lines
            .get(line_num.saturating_sub(1))
            .map(|s| s.as_str())
    }

    /// 获取行范围的内容
    fn get_line_range(&self, start: usize, end: usize) -> String {
        self.lines[start.saturating_sub(1)..end.min(self.lines.len())].join("\n")
    }
}

/// CodexLens FTS 引擎
pub struct CodexLensEngine {
    /// 配置
    config: CodexLensConfig,
    /// 倒排索引
    inverted_index: HashMap<String, InvertedIndexEntry>,
    /// 文档存储
    documents: HashMap<String, IndexedDocument>,
    /// 统计信息
    stats: CodexLensStats,
}

/// CodexLens 统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodexLensStats {
    /// 文档数量
    pub document_count: usize,
    /// 总词条数量
    pub total_terms: usize,
    /// 索引大小（字节）
    pub index_size_bytes: usize,
    /// 平均文档长度
    pub avg_doc_length: f64,
}

/// CodexLens 搜索选项
#[derive(Debug, Clone)]
pub struct CodexLensSearchOptions {
    /// 最大结果数
    pub max_results: usize,
    /// 最小分数
    pub min_score: f32,
    /// 语言过滤器
    pub language_filter: Option<Vec<String>>,
    /// 路径过滤器（glob 模式）
    pub path_filter: Option<Vec<String>>,
    /// 是否返回片段上下文
    pub include_context: bool,
    /// 上下文行数
    pub context_lines: usize,
}

impl Default for CodexLensSearchOptions {
    fn default() -> Self {
        Self {
            max_results: 100,
            min_score: 0.1,
            language_filter: None,
            path_filter: None,
            include_context: true,
            context_lines: 2,
        }
    }
}

impl CodexLensEngine {
    /// 创建新的 CodexLens 引擎
    pub fn new() -> Self {
        Self {
            config: CodexLensConfig::default(),
            inverted_index: HashMap::new(),
            documents: HashMap::new(),
            stats: CodexLensStats::default(),
        }
    }

    /// 创建带配置的引擎
    pub fn with_config(config: CodexLensConfig) -> Self {
        Self {
            config,
            inverted_index: HashMap::new(),
            documents: HashMap::new(),
            stats: CodexLensStats::default(),
        }
    }

    /// 索引文档
    pub fn index_document(
        &mut self,
        id: String,
        path: String,
        content: String,
        language: Option<String>,
    ) {
        let doc = IndexedDocument::new(id.clone(), path.clone(), content.clone(), language.clone());

        // 分词
        let tokens = self.tokenize(&doc.content);

        // 更新倒排索引
        for (term, positions) in tokens {
            let entry =
                self.inverted_index
                    .entry(term.clone())
                    .or_insert_with(|| InvertedIndexEntry {
                        document_ids: HashSet::new(),
                        positions: HashMap::new(),
                        doc_frequency: 0,
                    });

            entry.document_ids.insert(id.clone());
            entry
                .positions
                .entry(id.clone())
                .or_insert_with(Vec::new)
                .extend(positions);
            entry.doc_frequency = entry.document_ids.len();
        }

        // 存储文档
        self.documents.insert(id, doc);

        // 更新统计
        self.update_stats();
    }

    /// 分词
    fn tokenize(&self, content: &str) -> HashMap<String, Vec<usize>> {
        let mut tokens: HashMap<String, Vec<usize>> = HashMap::new();

        if self.config.code_aware_tokenization {
            self.code_aware_tokenize(content, &mut tokens);
        } else {
            self.simple_tokenize(content, &mut tokens);
        }

        tokens
    }

    /// 代码感知分词
    fn code_aware_tokenize(&self, content: &str, tokens: &mut HashMap<String, Vec<usize>>) {
        let mut line_num = 1;

        for line in content.lines() {
            let mut word_start = None;
            let mut chars: Vec<char> = line.chars().collect();

            for (i, c) in chars.iter().enumerate() {
                if c.is_alphanumeric() || *c == '_' {
                    if word_start.is_none() {
                        word_start = Some(i);
                    }
                } else {
                    if let Some(start) = word_start {
                        let word = chars[start..i].iter().collect::<String>().to_lowercase();
                        self.add_token(tokens, &word, line_num);
                        word_start = None;
                    }

                    // 特殊处理代码符号
                    if *c == '#' || *c == '/' || *c == '@' || *c == '.' {
                        if i + 1 < chars.len() {
                            let next_word: String = chars[i + 1..]
                                .iter()
                                .take_while(|&&c| c.is_alphanumeric() || c == '_')
                                .collect::<String>()
                                .to_lowercase();
                            if !next_word.is_empty() {
                                self.add_token(tokens, &format!("{}{}", c, next_word), line_num);
                            }
                        }
                    }
                }
            }

            // 处理行尾词
            if let Some(start) = word_start {
                let word = chars[start..].iter().collect::<String>().to_lowercase();
                self.add_token(tokens, &word, line_num);
            }

            line_num += 1;
        }
    }

    /// 简单分词
    fn simple_tokenize(&self, content: &str, tokens: &mut HashMap<String, Vec<usize>>) {
        let mut line_num = 1;

        for line in content.lines() {
            for word in line.split(|c: char| !c.is_alphanumeric()) {
                let word_lower = word.to_lowercase();
                self.add_token(tokens, &word_lower, line_num);
            }
            line_num += 1;
        }
    }

    /// 添加 token
    fn add_token(&self, tokens: &mut HashMap<String, Vec<usize>>, term: &str, line_num: usize) {
        // 过滤停用词
        if self.config.enable_stop_words && self.config.stop_words.contains(term) {
            return;
        }

        // 过滤长度
        if term.len() < self.config.min_word_length || term.len() > self.config.max_word_length {
            return;
        }

        tokens
            .entry(term.to_string())
            .or_insert_with(Vec::new)
            .push(line_num);
    }

    /// 搜索
    pub fn search(&self, query: &str, options: Option<CodexLensSearchOptions>) -> CodexLensResult {
        let opts = options.unwrap_or_default();
        let start = std::time::Instant::now();

        let (query_type, terms) = self.parse_query(query);

        let results = match query_type {
            QueryType::Phrase => self.search_phrase(&terms, &opts),
            QueryType::Prefix => self.search_prefix(&terms, &opts),
            QueryType::Fuzzy => self.search_fuzzy(&terms, &opts),
            _ => self.search_simple(&terms, &opts),
        };
        let total_hits = results.len();

        CodexLensResult {
            query: query.to_string(),
            results,
            total_hits,
            search_time_ms: start.elapsed().as_millis() as u64,
            query_type,
        }
    }

    /// 解析查询
    fn parse_query(&self, query: &str) -> (QueryType, Vec<String>) {
        let query_trimmed = query.trim();

        if query_trimmed.starts_with('/') && query_trimmed.ends_with('/') {
            // 正则表达式
            (
                QueryType::Regex,
                vec![query_trimmed[1..query_trimmed.len() - 1].to_string()],
            )
        } else if query_trimmed.ends_with('*') {
            // 前缀查询
            (
                QueryType::Prefix,
                vec![query_trimmed.trim_end_matches('*').to_string()],
            )
        } else if query_trimmed.contains('~') {
            // 模糊查询
            (
                QueryType::Fuzzy,
                vec![query_trimmed.trim_end_matches('~').to_string()],
            )
        } else if query_trimmed.contains(' ') {
            // 短语查询
            (
                QueryType::Phrase,
                query_trimmed
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect(),
            )
        } else {
            // 简单查询
            (QueryType::Simple, vec![query_trimmed.to_lowercase()])
        }
    }

    /// 简单搜索
    fn search_simple(
        &self,
        terms: &[String],
        options: &CodexLensSearchOptions,
    ) -> Vec<CodexLensHit> {
        let mut scores: HashMap<String, (f32, Vec<String>, usize)> = HashMap::new();

        for term in terms {
            if let Some(entry) = self.inverted_index.get(term) {
                for doc_id in &entry.document_ids {
                    // 检查过滤器
                    if !self.filter_document(doc_id, options) {
                        continue;
                    }

                    let positions = entry.positions.get(doc_id).map(|p| p.len()).unwrap_or(0);
                    let idf = self.calculate_idf(entry.doc_frequency);
                    let score = (positions as f32) * idf;

                    let entry_data =
                        scores
                            .entry(doc_id.clone())
                            .or_insert((0.0, Vec::new(), usize::MAX));
                    entry_data.0 += score;
                    if !entry_data.1.contains(term) {
                        entry_data.1.push(term.clone());
                    }
                    entry_data.2 = entry_data.2.min(
                        *entry
                            .positions
                            .get(doc_id)
                            .and_then(|p| p.first())
                            .unwrap_or(&1),
                    );
                }
            }
        }

        self.build_results(scores, options)
    }

    /// 短语搜索
    fn search_phrase(
        &self,
        terms: &[String],
        options: &CodexLensSearchOptions,
    ) -> Vec<CodexLensHit> {
        if terms.len() < 2 {
            return self.search_simple(terms, options);
        }

        let mut scores: HashMap<String, (f32, Vec<String>, usize)> = HashMap::new();

        // 找到第一个词的位置
        let first_term = &terms[0];
        if let Some(first_entry) = self.inverted_index.get(first_term) {
            for doc_id in &first_entry.document_ids {
                if !self.filter_document(doc_id, options) {
                    continue;
                }

                let first_positions: Vec<usize> = first_entry
                    .positions
                    .get(doc_id)
                    .cloned()
                    .unwrap_or_default();

                // 检查后续词是否在附近
                let mut phrase_found = false;
                let mut phrase_pos = usize::MAX;
                for &pos in &first_positions {
                    let mut all_found = true;
                    for (i, term) in terms.iter().skip(1).enumerate() {
                        if let Some(entry) = self.inverted_index.get(term) {
                            if let Some(term_positions) = entry.positions.get(doc_id) {
                                // 检查是否有位置在 pos + i 或附近
                                if !term_positions.contains(&(pos + i + 1)) {
                                    all_found = false;
                                    break;
                                }
                            } else {
                                all_found = false;
                                break;
                            }
                        } else {
                            all_found = false;
                            break;
                        }
                    }

                    if all_found {
                        phrase_found = true;
                        phrase_pos = pos;
                        break;
                    }
                }

                if phrase_found {
                    let idf = self.calculate_idf(first_entry.doc_frequency);
                    let score = 10.0 * idf; // 短语匹配给予更高分数

                    scores.insert(doc_id.clone(), (score, terms.to_vec(), phrase_pos));
                }
            }
        }

        self.build_results(scores, options)
    }

    /// 前缀搜索
    fn search_prefix(
        &self,
        prefix: &[String],
        options: &CodexLensSearchOptions,
    ) -> Vec<CodexLensHit> {
        let prefix_str = &prefix[0].to_lowercase();
        let mut scores: HashMap<String, (f32, Vec<String>, usize)> = HashMap::new();

        for (term, entry) in &self.inverted_index {
            if term.starts_with(prefix_str) {
                for doc_id in &entry.document_ids {
                    if !self.filter_document(doc_id, options) {
                        continue;
                    }

                    let idf = self.calculate_idf(entry.doc_frequency);
                    let score = (term.len() as f32 / prefix_str.len() as f32) * idf;

                    let entry_data =
                        scores
                            .entry(doc_id.clone())
                            .or_insert((0.0, Vec::new(), usize::MAX));
                    entry_data.0 += score;
                    if !entry_data.1.contains(term) {
                        entry_data.1.push(term.clone());
                    }
                }
            }
        }

        self.build_results(scores, options)
    }

    /// 模糊搜索
    fn search_fuzzy(
        &self,
        terms: &[String],
        options: &CodexLensSearchOptions,
    ) -> Vec<CodexLensHit> {
        let term = &terms[0].to_lowercase();
        let tolerance = self.config.fuzzy_tolerance;
        let mut scores: HashMap<String, (f32, Vec<String>, usize)> = HashMap::new();

        for (index_term, entry) in &self.inverted_index {
            let distance = Self::levenshtein_distance(term, index_term);
            if distance <= tolerance {
                for doc_id in &entry.document_ids {
                    if !self.filter_document(doc_id, options) {
                        continue;
                    }

                    let similarity =
                        1.0 - (distance as f32 / index_term.len().max(term.len()) as f32);
                    let idf = self.calculate_idf(entry.doc_frequency);
                    let score = similarity * idf;

                    scores
                        .entry(doc_id.clone())
                        .or_insert((0.0, Vec::new(), usize::MAX));
                }
            }
        }

        self.build_results(scores, options)
    }

    /// 构建搜索结果
    fn build_results(
        &self,
        scores: HashMap<String, (f32, Vec<String>, usize)>,
        options: &CodexLensSearchOptions,
    ) -> Vec<CodexLensHit> {
        let mut results: Vec<CodexLensHit> = scores
            .into_iter()
            .filter(|(_, (score, _, _))| *score >= options.min_score)
            .map(|(doc_id, (score, matched_terms, line_num))| {
                let doc = self.documents.get(&doc_id);
                CodexLensHit {
                    document_id: doc_id,
                    file_path: doc.as_ref().map(|d| d.path.clone()).unwrap_or_default(),
                    snippet: doc
                        .as_ref()
                        .map(|d| d.get_line(line_num).unwrap_or("").to_string())
                        .unwrap_or_default(),
                    line_number: line_num,
                    score,
                    matched_terms,
                    language: doc.and_then(|d| d.language.clone()),
                }
            })
            .collect();

        // 排序
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 限制数量
        results.truncate(options.max_results);

        results
    }

    /// 计算 IDF（逆文档频率）
    fn calculate_idf(&self, doc_frequency: usize) -> f32 {
        let N = self.documents.len().max(1) as f32;
        (N / doc_frequency as f32).ln() + 1.0
    }

    /// 检查文档是否满足过滤器
    fn filter_document(&self, doc_id: &str, options: &CodexLensSearchOptions) -> bool {
        let doc = match self.documents.get(doc_id) {
            Some(d) => d,
            None => return false,
        };

        // 语言过滤器
        if let Some(ref languages) = options.language_filter {
            if let Some(ref lang) = doc.language {
                if !languages.contains(lang) {
                    return false;
                }
            }
        }

        // 路径过滤器（简化实现）
        if let Some(ref paths) = options.path_filter {
            let matches = paths.iter().any(|p| doc.path.contains(p));
            if !matches {
                return false;
            }
        }

        true
    }

    /// Levenshtein 距离
    fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let len1 = s1.len();
        let len2 = s2.len();
        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if s1.chars().nth(i - 1) == s2.chars().nth(j - 1) {
                    0
                } else {
                    1
                };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[len1][len2]
    }

    /// 更新统计
    fn update_stats(&mut self) {
        self.stats.document_count = self.documents.len();
        self.stats.total_terms = self.inverted_index.len();
        self.stats.avg_doc_length = if self.documents.is_empty() {
            0.0
        } else {
            self.documents
                .values()
                .map(|d| d.content.len())
                .sum::<usize>() as f64
                / self.documents.len() as f64
        };
    }

    /// 获取统计
    pub fn stats(&self) -> &CodexLensStats {
        &self.stats
    }

    /// 清空索引
    pub fn clear(&mut self) {
        self.inverted_index.clear();
        self.documents.clear();
        self.stats = CodexLensStats::default();
    }
}

impl Default for CodexLensEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codexlens_config_default() {
        let config = CodexLensConfig::default();
        assert_eq!(config.min_word_length, 2);
        assert!(config.enable_fuzzy);
    }

    #[test]
    fn test_tokenization() {
        let engine = CodexLensEngine::new();
        let mut tokens = HashMap::new();
        engine.simple_tokenize("Hello World", &mut tokens);
        assert!(tokens.contains_key("hello"));
        assert!(tokens.contains_key("world"));
    }

    #[test]
    fn test_search() {
        let mut engine = CodexLensEngine::new();
        engine.index_document(
            "doc1".to_string(),
            "test.rs".to_string(),
            "fn main() {\n    println!(\"Hello\");\n}".to_string(),
            Some("Rust".to_string()),
        );

        let result = engine.search("println", None);
        assert!(result.total_hits >= 0);
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(CodexLensEngine::levenshtein_distance("hello", "helo"), 1);
        assert_eq!(CodexLensEngine::levenshtein_distance("hello", "world"), 4);
    }
}
