//! A2UI 确认对话框

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 对话框选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogOptions {
    /// 对话框标题
    pub title: String,
    /// 对话框内容
    pub content: String,
    /// 可选的 Markdown 内容
    pub markdown_content: Option<String>,
    /// 确认按钮文本
    pub confirm_text: Option<String>,
    /// 取消按钮文本
    pub cancel_text: Option<String>,
    /// 是否允许取消
    pub allow_cancel: bool,
    /// 默认聚焦的按钮
    pub default_focus: DialogFocus,
    /// 对话框类型
    pub dialog_type: DialogType,
    /// 超时时间（秒）
    pub timeout_secs: Option<u64>,
}

impl DialogOptions {
    /// 创建新的对话框选项
    pub fn new(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: content.into(),
            markdown_content: None,
            confirm_text: None,
            cancel_text: None,
            allow_cancel: true,
            default_focus: DialogFocus::Cancel,
            dialog_type: DialogType::Default,
            timeout_secs: None,
        }
    }

    /// 创建警告对话框
    pub fn warning(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(title, content).with_type(DialogType::Warning)
    }

    /// 创建危险操作对话框
    pub fn danger(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(title, content)
            .with_type(DialogType::Danger)
            .with_confirm_text("确认删除")
    }

    /// 创建信息对话框
    pub fn info(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(title, content).with_type(DialogType::Info)
    }

    /// 设置 Markdown 内容
    pub fn with_markdown(mut self, markdown: impl Into<String>) -> Self {
        self.markdown_content = Some(markdown.into());
        self
    }

    /// 设置确认按钮文本
    pub fn with_confirm_text(mut self, text: impl Into<String>) -> Self {
        self.confirm_text = Some(text.into());
        self
    }

    /// 设置取消按钮文本
    pub fn with_cancel_text(mut self, text: impl Into<String>) -> Self {
        self.cancel_text = Some(text.into());
        self
    }

    /// 设置是否允许取消
    pub fn with_allow_cancel(mut self, allow: bool) -> Self {
        self.allow_cancel = allow;
        self
    }

    /// 设置对话框类型
    pub fn with_type(mut self, dialog_type: DialogType) -> Self {
        self.dialog_type = dialog_type;
        self
    }

    /// 设置超时时间
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    /// 设置默认聚焦
    pub fn with_default_focus(mut self, focus: DialogFocus) -> Self {
        self.default_focus = focus;
        self
    }
}

/// 对话框类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogType {
    /// 默认对话框
    Default,
    /// 信息对话框
    Info,
    /// 警告对话框
    Warning,
    /// 危险操作对话框
    Danger,
    /// 成功对话框
    Success,
}

/// 默认聚焦的按钮
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogFocus {
    /// 聚焦确认按钮
    Confirm,
    /// 聚焦取消按钮
    Cancel,
}

/// 确认对话框
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationDialog {
    /// 对话框 ID
    pub id: String,
    /// 选项
    pub options: DialogOptions,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 来源（智能体或系统）
    pub source: String,
    /// 会话 ID
    pub session_id: String,
    /// 状态
    pub status: DialogStatus,
}

impl ConfirmationDialog {
    /// 创建新的确认对话框
    pub fn new(
        options: DialogOptions,
        source: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            options,
            created_at: Utc::now(),
            source: source.into(),
            session_id: session_id.into(),
            status: DialogStatus::Pending,
        }
    }

    /// 创建标准确认对话框
    pub fn confirm(
        title: impl Into<String>,
        content: impl Into<String>,
        source: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Self {
        Self::new(DialogOptions::new(title, content), source, session_id)
    }

    /// 创建警告确认对话框
    pub fn warn(
        title: impl Into<String>,
        content: impl Into<String>,
        source: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Self {
        Self::new(DialogOptions::warning(title, content), source, session_id)
    }

    /// 创建危险操作确认对话框
    pub fn danger(
        title: impl Into<String>,
        content: impl Into<String>,
        source: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Self {
        Self::new(DialogOptions::danger(title, content), source, session_id)
    }
}

/// 对话框状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogStatus {
    /// 等待用户响应
    Pending,
    /// 用户已确认
    Confirmed,
    /// 用户已取消
    Cancelled,
    /// 对话框已超时
    TimedOut,
}

/// 对话框响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogResponse {
    /// 对话框 ID
    pub dialog_id: String,
    /// 响应状态
    pub response: DialogResponseType,
    /// 响应时间
    pub responded_at: DateTime<Utc>,
    /// 用户选择的值（用于复选框等）
    pub user_value: Option<serde_json::Value>,
}

impl DialogResponse {
    /// 创建确认响应
    pub fn confirm(dialog_id: impl Into<String>) -> Self {
        Self {
            dialog_id: dialog_id.into(),
            response: DialogResponseType::Confirmed,
            responded_at: Utc::now(),
            user_value: None,
        }
    }

    /// 创建取消响应
    pub fn cancel(dialog_id: impl Into<String>) -> Self {
        Self {
            dialog_id: dialog_id.into(),
            response: DialogResponseType::Cancelled,
            responded_at: Utc::now(),
            user_value: None,
        }
    }

    /// 创建超时响应
    pub fn timeout(dialog_id: impl Into<String>) -> Self {
        Self {
            dialog_id: dialog_id.into(),
            response: DialogResponseType::TimedOut,
            responded_at: Utc::now(),
            user_value: None,
        }
    }

    /// 创建带值的响应
    pub fn with_value(mut self, value: serde_json::Value) -> Self {
        self.user_value = Some(value);
        self
    }
}

/// 对话框响应类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogResponseType {
    /// 用户确认
    Confirmed,
    /// 用户取消
    Cancelled,
    /// 超时
    TimedOut,
}

impl DialogResponseType {
    /// 判断是否已确认
    pub fn is_confirmed(&self) -> bool {
        matches!(self, DialogResponseType::Confirmed)
    }
}

/// 对话框处理器
#[async_trait::async_trait]
pub trait DialogHandler: Send + Sync {
    /// 显示对话框并等待用户响应
    async fn show(&self, dialog: ConfirmationDialog) -> Result<DialogResponse, crate::A2uiError>;

    /// 异步发送对话框（不等待响应）
    async fn send(&self, dialog: ConfirmationDialog) -> Result<(), crate::A2uiError>;

    /// 获取对话框响应（轮询）
    async fn get_response(
        &self,
        dialog_id: &str,
    ) -> Result<Option<DialogResponse>, crate::A2uiError>;

    /// 取消对话框
    async fn cancel(&self, dialog_id: &str) -> Result<(), crate::A2uiError>;
}

/// 简单的内存对话框处理器
pub struct InMemoryDialogHandler {
    dialogs: std::sync::RwLock<std::collections::HashMap<String, ConfirmationDialog>>,
    responses: std::sync::RwLock<std::collections::HashMap<String, DialogResponse>>,
}

impl InMemoryDialogHandler {
    /// 创建新的内存对话框处理器
    pub fn new() -> Self {
        Self {
            dialogs: std::sync::RwLock::new(std::collections::HashMap::new()),
            responses: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemoryDialogHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DialogHandler for InMemoryDialogHandler {
    async fn show(&self, dialog: ConfirmationDialog) -> Result<DialogResponse, crate::A2uiError> {
        let dialog_id = dialog.id.clone();

        // 存储对话框
        {
            let mut dialogs = self.dialogs.write().unwrap();
            dialogs.insert(dialog_id.clone(), dialog);
        }

        // 注意: 实际实现应该使用通道或消息队列来等待响应
        // 这里简化处理，返回一个模拟响应
        // 真实实现需要集成到事件系统中

        Ok(DialogResponse::confirm(dialog_id))
    }

    async fn send(&self, dialog: ConfirmationDialog) -> Result<(), crate::A2uiError> {
        let mut dialogs = self.dialogs.write().unwrap();
        dialogs.insert(dialog.id.clone(), dialog);
        Ok(())
    }

    async fn get_response(
        &self,
        dialog_id: &str,
    ) -> Result<Option<DialogResponse>, crate::A2uiError> {
        let responses = self.responses.read().unwrap();
        Ok(responses.get(dialog_id).cloned())
    }

    async fn cancel(&self, dialog_id: &str) -> Result<(), crate::A2uiError> {
        let mut dialogs = self.dialogs.write().unwrap();
        if let Some(dialog) = dialogs.get_mut(dialog_id) {
            dialog.status = DialogStatus::Cancelled;
            Ok(())
        } else {
            Err(crate::A2uiError::InvalidOperation(format!(
                "对话框 {} 不存在",
                dialog_id
            )))
        }
    }
}
