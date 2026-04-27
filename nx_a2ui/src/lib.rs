//! NexusFlow A2UI (Agent-to-User Interface)
//!
//! 智能体到用户通信接口,提供实时消息流、通知、确认对话框和进度回调。

pub mod dialog;
pub mod message;
pub mod notification;
pub mod progress;
pub mod streaming;

pub use dialog::{ConfirmationDialog, DialogOptions, DialogResponse};
pub use message::{A2UMessage, MessagePriority, MessageType};
pub use notification::{Notification, NotificationId, NotificationLevel};
pub use progress::{ProgressCallback, ProgressState, ProgressUpdate};
pub use streaming::{MessageStream, StreamConfig, StreamError};

use serde::{Deserialize, Serialize};

/// A2UI 错误类型
#[derive(Debug, thiserror::Error)]
pub enum A2uiError {
    #[error("流错误: {0}")]
    StreamError(String),

    #[error("通道已关闭")]
    ChannelClosed,

    #[error("超时: {0}")]
    Timeout(String),

    #[error("无效的操作: {0}")]
    InvalidOperation(String),
}

/// A2UI 会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2uiSession {
    /// 会话 ID
    pub session_id: String,
    /// 用户 ID
    pub user_id: Option<String>,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 元数据
    pub metadata: std::collections::HashMap<String, String>,
}

impl A2uiSession {
    /// 创建新的 A2UI 会话
    pub fn new() -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            user_id: None,
            created_at: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// 创建带有用户 ID 的会话
    pub fn with_user(user_id: impl Into<String>) -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            user_id: Some(user_id.into()),
            created_at: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

impl Default for A2uiSession {
    fn default() -> Self {
        Self::new()
    }
}
