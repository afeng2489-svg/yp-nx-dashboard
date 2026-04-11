//! A2UI 消息定义

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// 普通文本消息
    Text,
    /// Markdown 格式消息
    Markdown,
    /// 代码块
    Code,
    /// 错误消息
    Error,
    /// 警告消息
    Warning,
    /// 成功消息
    Success,
    /// 信息消息
    Info,
    ///调试消息
    Debug,
}

/// 消息优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessagePriority {
    /// 低优先级
    Low,
    /// 普通优先级
    Normal,
    /// 高优先级
    High,
    /// 紧急优先级
    Urgent,
}

impl MessagePriority {
    /// 判断是否为高优先级
    pub fn is_high(&self) -> bool {
        matches!(self, MessagePriority::High | MessagePriority::Urgent)
    }
}

/// A2UI 消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2UMessage {
    /// 消息 ID
    pub id: String,
    /// 会话 ID
    pub session_id: String,
    /// 消息类型
    pub msg_type: MessageType,
    /// 优先级
    pub priority: MessagePriority,
    /// 内容
    pub content: String,
    /// 来源（智能体 ID 或系统）
    pub source: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 元数据
    pub metadata: MessageMetadata,
}

/// 消息元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// 关联的工作流 ID
    pub workflow_id: Option<String>,
    /// 关联的阶段 ID
    pub stage_id: Option<String>,
    /// 关联的智能体 ID
    pub agent_id: Option<String>,
    /// 原始消息 ID（用于回复）
    pub reply_to: Option<String>,
    /// 额外的数据
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl A2UMessage {
    /// 创建新的文本消息
    pub fn text(session_id: impl Into<String>, source: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            msg_type: MessageType::Text,
            priority: MessagePriority::Normal,
            content: content.into(),
            source: source.into(),
            timestamp: Utc::now(),
            metadata: MessageMetadata::default(),
        }
    }

    /// 创建 Markdown 消息
    pub fn markdown(session_id: impl Into<String>, source: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            msg_type: MessageType::Markdown,
            priority: MessagePriority::Normal,
            content: content.into(),
            source: source.into(),
            timestamp: Utc::now(),
            metadata: MessageMetadata::default(),
        }
    }

    /// 创建错误消息
    pub fn error(session_id: impl Into<String>, source: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            msg_type: MessageType::Error,
            priority: MessagePriority::High,
            content: content.into(),
            source: source.into(),
            timestamp: Utc::now(),
            metadata: MessageMetadata::default(),
        }
    }

    /// 创建代码块消息
    pub fn code(session_id: impl Into<String>, source: impl Into<String>, content: impl Into<String>, language: Option<String>) -> Self {
        let mut metadata = MessageMetadata::default();
        if let Some(lang) = language {
            metadata.extra.insert("language".to_string(), serde_json::Value::String(lang));
        }
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            msg_type: MessageType::Code,
            priority: MessagePriority::Normal,
            content: content.into(),
            source: source.into(),
            timestamp: Utc::now(),
            metadata,
        }
    }

    /// 设置工作流上下文
    pub fn with_workflow(mut self, workflow_id: impl Into<String>, stage_id: Option<String>, agent_id: Option<String>) -> Self {
        self.metadata.workflow_id = Some(workflow_id.into());
        self.metadata.stage_id = stage_id;
        self.metadata.agent_id = agent_id;
        self
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }

    /// 回复某条消息
    pub fn replying_to(mut self, original_id: impl Into<String>) -> Self {
        self.metadata.reply_to = Some(original_id.into());
        self
    }
}

impl Default for MessageMetadata {
    fn default() -> Self {
        Self {
            workflow_id: None,
            stage_id: None,
            agent_id: None,
            reply_to: None,
            extra: std::collections::HashMap::new(),
        }
    }
}
