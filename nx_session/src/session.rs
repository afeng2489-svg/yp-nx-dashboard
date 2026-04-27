//! 会话定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 会话 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    /// 创建新的会话 ID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 会话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// 等待中
    Pending,
    /// 运行中
    Running,
    /// 活跃
    Active,
    /// 空闲
    Idle,
    /// 已暂停
    Paused,
    /// 已终止
    Terminated,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Pending => write!(f, "pending"),
            SessionStatus::Running => write!(f, "running"),
            SessionStatus::Active => write!(f, "active"),
            SessionStatus::Idle => write!(f, "idle"),
            SessionStatus::Paused => write!(f, "paused"),
            SessionStatus::Terminated => write!(f, "terminated"),
        }
    }
}

/// 会话元数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// 工作流 ID
    pub workflow_id: Option<String>,
    /// 执行 ID
    pub execution_id: Option<String>,
    /// 恢复密钥
    pub resume_key: Option<String>,
    /// 用户指定的项目根目录
    pub project_root: Option<String>,
}

/// 会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// 会话 ID
    pub id: SessionId,
    /// 会话状态
    pub status: SessionStatus,
    /// 元数据
    pub metadata: SessionMetadata,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
    /// 终止时间（如果已终止）
    pub terminated_at: Option<DateTime<Utc>>,
}

impl Session {
    /// 创建新的会话
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            status: SessionStatus::Pending,
            metadata: SessionMetadata {
                workflow_id: None,
                execution_id: None,
                resume_key: Some(Uuid::new_v4().to_string()),
                project_root: None,
            },
            created_at: now,
            updated_at: now,
            terminated_at: None,
        }
    }

    /// 创建带工作流 ID 的会话
    pub fn with_workflow(workflow_id: String) -> Self {
        let mut session = Self::new();
        session.metadata.workflow_id = Some(workflow_id);
        session
    }

    /// 激活会话
    pub fn activate(&mut self) {
        self.status = SessionStatus::Active;
        self.updated_at = Utc::now();
    }

    /// 标记为空闲
    pub fn idle(&mut self) {
        self.status = SessionStatus::Idle;
        self.updated_at = Utc::now();
    }

    /// 暂停会话
    pub fn pause(&mut self) {
        self.status = SessionStatus::Paused;
        self.updated_at = Utc::now();
    }

    /// 终止会话
    pub fn terminate(&mut self) {
        self.status = SessionStatus::Terminated;
        self.terminated_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// 检查会话是否活跃
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            SessionStatus::Running | SessionStatus::Active | SessionStatus::Idle
        )
    }

    /// 检查会话是否可以恢复
    pub fn can_resume(&self) -> bool {
        self.is_active() || self.status == SessionStatus::Paused
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// 会话状态快照（用于序列化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub id: String,
    pub status: String,
    pub variables: serde_json::Value,
    pub current_stage: usize,
    pub stage_results: Vec<serde_json::Value>,
    pub agent_states: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}
