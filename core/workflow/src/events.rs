//! 工作流事件
//!
//! 工作流执行通知的事件系统。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 工作流事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WorkflowEvent {
    /// 工作流已开始
    WorkflowStarted {
        execution_id: Uuid,
        workflow_id: String,
    },
    /// 阶段已开始
    StageStarted {
        execution_id: Uuid,
        stage_name: String,
        stage_index: usize,
    },
    /// 阶段已完成
    StageCompleted {
        execution_id: Uuid,
        stage_name: String,
        outputs: Vec<super::StageOutput>,
        quality_gate_result: Option<super::QualityGateResult>,
    },
    /// 智能体已开始
    AgentStarted {
        execution_id: Uuid,
        agent_id: String,
        role: String,
    },
    /// 智能体消息
    AgentMessage {
        execution_id: Uuid,
        agent_id: String,
        message: String,
    },
    /// 智能体已完成
    AgentCompleted {
        execution_id: Uuid,
        agent_id: String,
        output: String,
    },
    /// 智能体失败
    AgentFailed {
        execution_id: Uuid,
        agent_id: String,
        error: String,
    },
    /// 工作流已完成
    WorkflowCompleted {
        execution_id: Uuid,
        final_state: String,
    },
    /// 工作流失败
    WorkflowFailed { execution_id: Uuid, error: String },
    /// 工作流已取消
    WorkflowCancelled { execution_id: Uuid },
    /// 变量已设置
    VariableSet {
        execution_id: Uuid,
        key: String,
        value: serde_json::Value,
    },
    /// user_input stage 触发：工作流暂停等待用户选择
    WorkflowPaused {
        execution_id: Uuid,
        stage_name: String,
        question: String,
        /// Vec<(展示文字, 值)>
        options: Vec<(String, String)>,
    },
    /// 工作流从暂停中恢复
    WorkflowResumed {
        execution_id: Uuid,
        stage_name: String,
        chosen_value: String,
    },
    /// 质量门检查完成
    QualityGateChecked {
        execution_id: Uuid,
        stage_name: String,
        passed: bool,
        retry_count: usize,
        checks_summary: String,
    },
}

impl WorkflowEvent {
    /// 获取事件时间戳
    pub fn timestamp(&self) -> DateTime<Utc> {
        Utc::now()
    }

    /// 获取执行 ID
    pub fn execution_id(&self) -> Option<Uuid> {
        match self {
            WorkflowEvent::WorkflowStarted { execution_id, .. } => Some(*execution_id),
            WorkflowEvent::StageStarted { execution_id, .. } => Some(*execution_id),
            WorkflowEvent::StageCompleted { execution_id, .. } => Some(*execution_id),
            WorkflowEvent::AgentStarted { execution_id, .. } => Some(*execution_id),
            WorkflowEvent::AgentMessage { execution_id, .. } => Some(*execution_id),
            WorkflowEvent::AgentCompleted { execution_id, .. } => Some(*execution_id),
            WorkflowEvent::AgentFailed { execution_id, .. } => Some(*execution_id),
            WorkflowEvent::WorkflowCompleted { execution_id, .. } => Some(*execution_id),
            WorkflowEvent::WorkflowFailed { execution_id, .. } => Some(*execution_id),
            WorkflowEvent::WorkflowCancelled { execution_id } => Some(*execution_id),
            WorkflowEvent::VariableSet { execution_id, .. } => Some(*execution_id),
            WorkflowEvent::WorkflowPaused { execution_id, .. } => Some(*execution_id),
            WorkflowEvent::WorkflowResumed { execution_id, .. } => Some(*execution_id),
            WorkflowEvent::QualityGateChecked { execution_id, .. } => Some(*execution_id),
        }
    }
}

/// 事件发射器 trait
pub trait EventEmitter: Send + Sync {
    /// 发射事件
    fn emit(&self, event: WorkflowEvent);

    /// 获取事件接收通道
    fn subscribe(&self) -> tokio::sync::mpsc::Receiver<WorkflowEvent>;
}

/// 简单的内存事件发射器
pub struct InMemoryEventEmitter {
    sender: tokio::sync::broadcast::Sender<WorkflowEvent>,
}

impl InMemoryEventEmitter {
    /// 创建新的内存事件发射器
    pub fn new() -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(100);
        Self { sender }
    }
}

impl Default for InMemoryEventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl EventEmitter for InMemoryEventEmitter {
    fn emit(&self, event: WorkflowEvent) {
        let _ = self.sender.send(event);
    }

    fn subscribe(&self) -> tokio::sync::mpsc::Receiver<WorkflowEvent> {
        let mut receiver = self.sender.subscribe();
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            while let Ok(event) = receiver.recv().await {
                if tx.send(event).await.is_err() {
                    break;
                }
            }
        });

        rx
    }
}

use parking_lot::RwLock;
use std::sync::Arc;

/// 用于测试和调试的事件收集器
pub struct EventCollector {
    events: Arc<RwLock<Vec<WorkflowEvent>>>,
}

impl EventCollector {
    /// 创建新的事件收集器
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 记录事件
    pub fn record(&self, event: WorkflowEvent) {
        let mut events = self.events.write();
        events.push(event);
    }

    /// 获取所有事件
    pub fn get_events(&self) -> Vec<WorkflowEvent> {
        self.events.read().clone()
    }

    /// 清空事件
    pub fn clear(&self) {
        self.events.write().clear();
    }

    /// 获取事件数量
    pub fn len(&self) -> usize {
        self.events.read().len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.events.read().is_empty()
    }
}

impl Default for EventCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl EventEmitter for Arc<EventCollector> {
    fn emit(&self, event: WorkflowEvent) {
        self.record(event);
    }

    fn subscribe(&self) -> tokio::sync::mpsc::Receiver<WorkflowEvent> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let collector = self.clone();

        tokio::spawn(async move {
            let events = collector.get_events();
            for event in events {
                if tx.send(event).await.is_err() {
                    break;
                }
            }
        });

        rx
    }
}
