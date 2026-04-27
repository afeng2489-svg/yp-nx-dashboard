//! A2UI 进度回调

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 进度状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressState {
    /// 进度开始
    Started,
    /// 进行中
    InProgress,
    /// 进度完成
    Completed,
    /// 进度失败
    Failed,
    /// 进度取消
    Cancelled,
}

/// 进度更新
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    /// 进度 ID
    pub id: String,
    /// 任务名称
    pub task_name: String,
    /// 当前状态
    pub state: ProgressState,
    /// 当前步骤
    pub current_step: usize,
    /// 总步骤数
    pub total_steps: usize,
    /// 进度百分比 (0-100)
    pub percentage: u8,
    /// 进度消息
    pub message: Option<String>,
    /// 详细状态
    pub details: Option<String>,
    /// 开始时间
    pub started_at: DateTime<Utc>,
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
    /// 完成时间（如果已完成）
    pub completed_at: Option<DateTime<Utc>>,
    /// 关联的工作流 ID
    pub workflow_id: Option<String>,
    /// 关联的智能体 ID
    pub agent_id: Option<String>,
    /// 错误信息（如果失败）
    pub error: Option<String>,
}

impl ProgressUpdate {
    /// 创建新的进度更新
    pub fn new(id: impl Into<String>, task_name: impl Into<String>, total_steps: usize) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            task_name: task_name.into(),
            state: ProgressState::Started,
            current_step: 0,
            total_steps,
            percentage: 0,
            message: None,
            details: None,
            started_at: now,
            updated_at: now,
            completed_at: None,
            workflow_id: None,
            agent_id: None,
            error: None,
        }
    }

    /// 设置工作流上下文
    pub fn with_workflow(mut self, workflow_id: impl Into<String>) -> Self {
        self.workflow_id = Some(workflow_id.into());
        self
    }

    /// 设置智能体上下文
    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// 更新进度
    pub fn update(&mut self, current_step: usize, message: Option<impl Into<String>>) {
        self.current_step = current_step.min(self.total_steps);
        self.percentage = if self.total_steps > 0 {
            ((self.current_step as f32 / self.total_steps as f32) * 100.0) as u8
        } else {
            0
        };
        self.message = message.map(|m| m.into());
        self.state = if self.current_step >= self.total_steps {
            ProgressState::Completed
        } else {
            ProgressState::InProgress
        };
        self.updated_at = Utc::now();
    }

    /// 设置详细状态
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// 标记为完成
    pub fn complete(&mut self) {
        self.state = ProgressState::Completed;
        self.current_step = self.total_steps;
        self.percentage = 100;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// 标记为失败
    pub fn fail(&mut self, error: impl Into<String>) {
        self.state = ProgressState::Failed;
        self.error = Some(error.into());
        self.updated_at = Utc::now();
    }

    /// 标记为取消
    pub fn cancel(&mut self) {
        self.state = ProgressState::Cancelled;
        self.updated_at = Utc::now();
    }

    /// 计算剩余时间（秒）
    pub fn remaining_secs(&self) -> Option<f64> {
        if self.state == ProgressState::Completed || self.current_step == 0 {
            return None;
        }

        let elapsed = (self.updated_at - self.started_at).num_seconds() as f64;
        let per_step = elapsed / self.current_step as f64;
        let remaining_steps = self.total_steps - self.current_step;

        Some(per_step * remaining_steps as f64)
    }
}

/// 进度回调特征
#[async_trait::async_trait]
pub trait ProgressCallback: Send + Sync {
    /// 开始进度
    async fn start(&self, update: ProgressUpdate) -> Result<(), crate::A2uiError>;

    /// 更新进度
    async fn update(&self, update: ProgressUpdate) -> Result<(), crate::A2uiError>;

    /// 完成进度
    async fn complete(&self, id: &str) -> Result<(), crate::A2uiError>;

    /// 标记进度失败
    async fn fail(&self, id: &str, error: &str) -> Result<(), crate::A2uiError>;

    /// 取消进度
    async fn cancel(&self, id: &str) -> Result<(), crate::A2uiError>;

    /// 获取进度状态
    async fn get(&self, id: &str) -> Result<Option<ProgressUpdate>, crate::A2uiError>;

    /// 列出所有活动进度
    async fn list_active(&self) -> Result<Vec<ProgressUpdate>, crate::A2uiError>;
}

/// 进度跟踪器
pub struct ProgressTracker {
    updates: std::sync::RwLock<std::collections::HashMap<String, ProgressUpdate>>,
}

impl ProgressTracker {
    /// 创建新的进度跟踪器
    pub fn new() -> Self {
        Self {
            updates: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// 创建进度更新
    pub fn create_update(
        &self,
        task_name: impl Into<String>,
        total_steps: usize,
    ) -> ProgressUpdate {
        ProgressUpdate::new(uuid::Uuid::new_v4().to_string(), task_name, total_steps)
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ProgressCallback for ProgressTracker {
    async fn start(&self, update: ProgressUpdate) -> Result<(), crate::A2uiError> {
        let mut updates = self.updates.write().unwrap();
        updates.insert(update.id.clone(), update);
        Ok(())
    }

    async fn update(&self, mut update: ProgressUpdate) -> Result<(), crate::A2uiError> {
        let mut updates = self.updates.write().unwrap();
        update.updated_at = Utc::now();

        if let Some(existing) = updates.get_mut(&update.id) {
            existing.current_step = update.current_step;
            existing.percentage = update.percentage;
            existing.message = update.message;
            existing.details = update.details;
            existing.state = update.state;
            existing.updated_at = update.updated_at;
            existing.error = update.error;
        } else {
            updates.insert(update.id.clone(), update);
        }
        Ok(())
    }

    async fn complete(&self, id: &str) -> Result<(), crate::A2uiError> {
        let mut updates = self.updates.write().unwrap();
        if let Some(update) = updates.get_mut(id) {
            update.state = ProgressState::Completed;
            update.percentage = 100;
            update.current_step = update.total_steps;
            update.completed_at = Some(Utc::now());
            update.updated_at = Utc::now();
            Ok(())
        } else {
            Err(crate::A2uiError::InvalidOperation(format!(
                "进度 {} 不存在",
                id
            )))
        }
    }

    async fn fail(&self, id: &str, error: &str) -> Result<(), crate::A2uiError> {
        let mut updates = self.updates.write().unwrap();
        if let Some(update) = updates.get_mut(id) {
            update.state = ProgressState::Failed;
            update.error = Some(error.to_string());
            update.updated_at = Utc::now();
            Ok(())
        } else {
            Err(crate::A2uiError::InvalidOperation(format!(
                "进度 {} 不存在",
                id
            )))
        }
    }

    async fn cancel(&self, id: &str) -> Result<(), crate::A2uiError> {
        let mut updates = self.updates.write().unwrap();
        if let Some(update) = updates.get_mut(id) {
            update.state = ProgressState::Cancelled;
            update.updated_at = Utc::now();
            Ok(())
        } else {
            Err(crate::A2uiError::InvalidOperation(format!(
                "进度 {} 不存在",
                id
            )))
        }
    }

    async fn get(&self, id: &str) -> Result<Option<ProgressUpdate>, crate::A2uiError> {
        let updates = self.updates.read().unwrap();
        Ok(updates.get(id).cloned())
    }

    async fn list_active(&self) -> Result<Vec<ProgressUpdate>, crate::A2uiError> {
        let updates = self.updates.read().unwrap();
        let active: Vec<ProgressUpdate> = updates
            .values()
            .filter(|u| u.state == ProgressState::Started || u.state == ProgressState::InProgress)
            .cloned()
            .collect();
        Ok(active)
    }
}
