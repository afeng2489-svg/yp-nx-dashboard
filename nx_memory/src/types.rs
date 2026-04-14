//! Memory 模块类型定义
//!
//! 核心数据结构：Transcript（记忆条目）、MemoryChunk（分块）、SearchResult（搜索结果）

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 消息角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "user" => Some(MessageRole::User),
            "assistant" => Some(MessageRole::Assistant),
            "system" => Some(MessageRole::System),
            _ => None,
        }
    }
}

/// 记忆条目（存储最小单位）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    /// 唯一 ID
    pub id: String,
    /// 团队 ID
    pub team_id: String,
    /// 会话 ID（可选）
    pub session_id: Option<String>,
    /// 发言用户 ID
    pub user_id: String,
    /// 角色
    pub role: MessageRole,
    /// 内容
    pub content: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 额外元数据
    pub metadata: TranscriptMetadata,
}

/// 记忆条目元数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TranscriptMetadata {
    /// 项目 ID
    pub project_id: Option<String>,
    /// 用户名称
    pub user_name: Option<String>,
    /// 消息 Token 数（估算）
    pub token_count: Option<usize>,
    /// 对话轮次
    pub conversation_turn: Option<u32>,
}

impl Transcript {
    /// 创建新的记忆条目
    pub fn new(
        team_id: impl Into<String>,
        user_id: impl Into<String>,
        role: MessageRole,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            team_id: team_id.into(),
            session_id: None,
            user_id: user_id.into(),
            role,
            content: content.into(),
            created_at: Utc::now(),
            metadata: TranscriptMetadata::default(),
        }
    }

    /// 设置会话 ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 设置元数据
    pub fn with_metadata(mut self, metadata: TranscriptMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// 估算 Token 数（简单估算：中文 2 字/token，英文 4 字符/token）
    pub fn estimate_tokens(&self) -> usize {
        let cjk = self.content.chars().filter(|c| c.len_utf8() > 1).count();
        let ascii = self.content.len() - cjk * 2;
        (cjk / 2) + (ascii / 4)
    }
}

/// 记忆块（分块存储单位）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryChunk {
    /// 唯一 ID
    pub id: String,
    /// 关联的记忆条目 ID
    pub transcript_id: String,
    /// 内容
    pub content: String,
    /// 块索引
    pub chunk_index: u32,
    /// Token 数
    pub token_count: usize,
    /// 预计算的向量（存储用）
    pub embedding: Option<Vec<f32>>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

impl MemoryChunk {
    /// 从记忆条目创建
    pub fn from_transcript(transcript: &Transcript, content: String, index: u32) -> Self {
        let token_count = Self::estimate_tokens(&content);
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            transcript_id: transcript.id.clone(),
            content,
            chunk_index: index,
            token_count,
            embedding: None,
            created_at: transcript.created_at,
        }
    }

    fn estimate_tokens(content: &str) -> usize {
        let cjk = content.chars().filter(|c| c.len_utf8() > 1).count();
        let ascii = content.len() - cjk * 2;
        (cjk / 2) + (ascii / 4)
    }
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 记忆块 ID
    pub chunk_id: String,
    /// 记忆条目 ID
    pub transcript_id: String,
    /// 内容
    pub content: String,
    /// 综合分数
    pub score: f32,
    /// BM25 分数
    pub bm25_score: f32,
    /// 向量相似度分数
    pub vector_score: f32,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 元数据
    pub metadata: TranscriptMetadata,
}

/// 搜索请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// 团队 ID (可选, 如果未提供则使用 URL 路径中的 team_id)
    #[serde(default)]
    pub team_id: Option<String>,
    /// 查询内容
    pub query: String,
    /// 返回数量
    pub top_k: Option<usize>,
    /// 向量权重
    pub vector_weight: Option<f32>,
    /// 关键词权重
    pub keyword_weight: Option<f32>,
    /// 会话 ID 过滤
    pub session_id: Option<String>,
}

impl SearchRequest {
    pub fn new(team_id: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            team_id: Some(team_id.into()),
            query: query.into(),
            top_k: Some(3),
            vector_weight: Some(0.7),
            keyword_weight: Some(0.3),
            session_id: None,
        }
    }
}

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            team_id: None,
            query: String::new(),
            top_k: Some(3),
            vector_weight: Some(0.7),
            keyword_weight: Some(0.3),
            session_id: None,
        }
    }
}

/// 搜索响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// 结果列表
    pub results: Vec<SearchResult>,
    /// 总数
    pub total: usize,
    /// 查询内容
    pub query: String,
    /// 搜索耗时（毫秒）
    pub search_time_ms: u64,
}

/// 存储请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreRequest {
    /// 团队 ID (可选, 如果未提供则使用 URL 路径中的 team_id)
    #[serde(default)]
    pub team_id: Option<String>,
    /// 会话 ID
    pub session_id: Option<String>,
    /// 发言用户 ID
    pub user_id: String,
    /// 发言用户名称
    pub user_name: Option<String>,
    /// 角色
    pub role: MessageRole,
    /// 内容
    pub content: String,
}

impl StoreRequest {
    pub fn new(
        team_id: impl Into<String>,
        user_id: impl Into<String>,
        role: MessageRole,
        content: impl Into<String>,
    ) -> Self {
        Self {
            team_id: Some(team_id.into()),
            session_id: None,
            user_id: user_id.into(),
            user_name: None,
            role,
            content: content.into(),
        }
    }
}

/// 批量存储请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStoreRequest {
    pub items: Vec<StoreRequest>,
}

/// 统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    /// 团队 ID
    pub team_id: String,
    /// 记忆条目总数
    pub transcript_count: usize,
    /// 记忆块总数
    pub chunk_count: usize,
    /// 向量化条目数
    pub embedded_count: usize,
    /// 总 Token 数（估算）
    pub total_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcript_creation() {
        let t = Transcript::new("team1", "user1", MessageRole::User, "Hello world");
        assert_eq!(t.team_id, "team1");
        assert_eq!(t.user_id, "user1");
        assert_eq!(t.role, MessageRole::User);
        assert_eq!(t.content, "Hello world");
        assert!(!t.id.is_empty());
    }

    #[test]
    fn test_search_request() {
        let req = SearchRequest::new("team1", "database");
        assert_eq!(req.team_id, "team1");
        assert_eq!(req.query, "database");
        assert_eq!(req.top_k, Some(3));
    }
}
