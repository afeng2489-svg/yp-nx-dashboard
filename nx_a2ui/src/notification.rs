//! A2UI 通知系统

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 通知 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NotificationId(pub String);

impl NotificationId {
    /// 创建新的通知 ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for NotificationId {
    fn default() -> Self {
        Self::new()
    }
}

/// 通知级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationLevel {
    /// 信息
    Info,
    /// 成功
    Success,
    /// 警告
    Warning,
    /// 错误
    Error,
    /// 调试
    Debug,
}

impl NotificationLevel {
    /// 获取级别优先级
    pub fn priority(&self) -> u8 {
        match self {
            NotificationLevel::Debug => 0,
            NotificationLevel::Info => 1,
            NotificationLevel::Success => 2,
            NotificationLevel::Warning => 3,
            NotificationLevel::Error => 4,
        }
    }

    /// 是否需要用户确认
    pub fn requires_ack(&self) -> bool {
        matches!(self, NotificationLevel::Error | NotificationLevel::Warning)
    }
}

/// 通知
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// 通知 ID
    pub id: NotificationId,
    /// 标题
    pub title: String,
    /// 内容
    pub body: String,
    /// 级别
    pub level: NotificationLevel,
    /// 来源
    pub source: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 是否已读
    pub read: bool,
    /// 是否已确认
    pub acknowledged: bool,
    /// 元数据
    pub metadata: NotificationMetadata,
}

/// 通知元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationMetadata {
    /// 关联的工作流 ID
    pub workflow_id: Option<String>,
    /// 关联的智能体 ID
    pub agent_id: Option<String>,
    /// 点击动作
    pub action_url: Option<String>,
    /// 过期时间
    pub expires_at: Option<DateTime<Utc>>,
    /// 额外数据
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl Notification {
    /// 创建新的通知
    pub fn new(
        title: impl Into<String>,
        body: impl Into<String>,
        level: NotificationLevel,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: NotificationId::new(),
            title: title.into(),
            body: body.into(),
            level,
            source: source.into(),
            timestamp: Utc::now(),
            read: false,
            acknowledged: false,
            metadata: NotificationMetadata::default(),
        }
    }

    /// 创建信息通知
    pub fn info(
        title: impl Into<String>,
        body: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self::new(title, body, NotificationLevel::Info, source)
    }

    /// 创建成功通知
    pub fn success(
        title: impl Into<String>,
        body: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self::new(title, body, NotificationLevel::Success, source)
    }

    /// 创建警告通知
    pub fn warning(
        title: impl Into<String>,
        body: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self::new(title, body, NotificationLevel::Warning, source)
    }

    /// 创建错误通知
    pub fn error(
        title: impl Into<String>,
        body: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self::new(title, body, NotificationLevel::Error, source)
    }

    /// 设置工作流上下文
    pub fn with_workflow(mut self, workflow_id: impl Into<String>) -> Self {
        self.metadata.workflow_id = Some(workflow_id.into());
        self
    }

    /// 设置智能体上下文
    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.metadata.agent_id = Some(agent_id.into());
        self
    }

    /// 设置点击动作 URL
    pub fn with_action(mut self, url: impl Into<String>) -> Self {
        self.metadata.action_url = Some(url.into());
        self
    }

    /// 设置过期时间
    pub fn expires_in(self, duration: chrono::Duration) -> Self {
        self.with_expiry(Utc::now() + duration)
    }

    /// 设置过期时间点
    pub fn with_expiry(mut self, expiry: DateTime<Utc>) -> Self {
        self.metadata.expires_at = Some(expiry);
        self
    }

    /// 标记为已读
    pub fn mark_read(&mut self) {
        self.read = true;
    }

    /// 确认通知
    pub fn acknowledge(&mut self) {
        self.acknowledged = true;
    }

    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        self.metadata
            .expires_at
            .map(|exp| Utc::now() > exp)
            .unwrap_or(false)
    }
}

/// 通知处理器
#[async_trait::async_trait]
pub trait NotificationHandler: Send + Sync {
    /// 发送通知
    async fn send(&self, notification: Notification) -> Result<(), crate::A2uiError>;

    /// 标记通知为已读
    async fn mark_read(&self, id: &NotificationId) -> Result<(), crate::A2uiError>;

    /// 确认通知
    async fn acknowledge(&self, id: &NotificationId) -> Result<(), crate::A2uiError>;

    /// 获取用户的所有通知
    async fn get_notifications(
        &self,
        user_id: &str,
        unread_only: bool,
    ) -> Result<Vec<Notification>, crate::A2uiError>;
}

/// 通知存储器（内存实现）
pub struct InMemoryNotificationStore {
    notifications: std::sync::RwLock<std::collections::HashMap<NotificationId, Notification>>,
    user_notifications: std::sync::RwLock<std::collections::HashMap<String, Vec<NotificationId>>>,
}

impl InMemoryNotificationStore {
    /// 创建新的内存通知存储
    pub fn new() -> Self {
        Self {
            notifications: std::sync::RwLock::new(std::collections::HashMap::new()),
            user_notifications: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// 添加通知到用户
    pub fn add_for_user(&self, user_id: &str, notification: Notification) {
        let id = notification.id.clone();
        let mut notifications = self.notifications.write().unwrap();
        let mut user_notifications = self.user_notifications.write().unwrap();

        notifications.insert(id.clone(), notification);
        user_notifications
            .entry(user_id.to_string())
            .or_default()
            .push(id);
    }
}

impl Default for InMemoryNotificationStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl NotificationHandler for InMemoryNotificationStore {
    async fn send(&self, notification: Notification) -> Result<(), crate::A2uiError> {
        // 通知存储不需要用户 ID，这里简化处理
        let id = notification.id.clone();
        let mut notifications = self.notifications.write().unwrap();
        notifications.insert(id, notification);
        Ok(())
    }

    async fn mark_read(&self, id: &NotificationId) -> Result<(), crate::A2uiError> {
        let mut notifications = self.notifications.write().unwrap();
        if let Some(n) = notifications.get_mut(id) {
            n.mark_read();
            Ok(())
        } else {
            Err(crate::A2uiError::InvalidOperation(format!(
                "通知 {} 不存在",
                id.0
            )))
        }
    }

    async fn acknowledge(&self, id: &NotificationId) -> Result<(), crate::A2uiError> {
        let mut notifications = self.notifications.write().unwrap();
        if let Some(n) = notifications.get_mut(id) {
            n.acknowledge();
            Ok(())
        } else {
            Err(crate::A2uiError::InvalidOperation(format!(
                "通知 {} 不存在",
                id.0
            )))
        }
    }

    async fn get_notifications(
        &self,
        _user_id: &str,
        unread_only: bool,
    ) -> Result<Vec<Notification>, crate::A2uiError> {
        let notifications = self.notifications.read().unwrap();
        let mut result: Vec<Notification> = notifications
            .values()
            .filter(|n| !n.is_expired())
            .cloned()
            .collect();

        if unread_only {
            result.retain(|n| !n.read);
        }

        // 按时间倒序
        result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(result)
    }
}
