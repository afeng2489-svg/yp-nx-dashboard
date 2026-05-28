//! 工作流事件
//!
//! 工作流执行通知的事件系统。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn default_pause_kind() -> String {
    "user_input".to_string()
}

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
    /// user_input / approval stage 触发：工作流暂停等待用户选择
    WorkflowPaused {
        execution_id: Uuid,
        stage_name: String,
        question: String,
        /// Vec<(展示文字, 值)>
        options: Vec<(String, String)>,
        /// user_input | approval
        #[serde(default = "default_pause_kind")]
        pause_kind: String,
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
    /// Agent token 用量
    AgentTokenUsage {
        execution_id: Uuid,
        agent_id: String,
        input_tokens: u64,
        output_tokens: u64,
        /// claude_cli | api
        executor: String,
        provider: String,
        estimated_cost_usd: f64,
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
            WorkflowEvent::AgentTokenUsage { execution_id, .. } => Some(*execution_id),
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── EventCollector tests ──

    #[test]
    fn collector_new_is_empty() {
        let c = EventCollector::new();
        assert!(c.is_empty());
        assert_eq!(c.len(), 0);
    }

    #[test]
    fn collector_records_events() {
        let c = EventCollector::new();
        let eid = Uuid::new_v4();
        c.record(WorkflowEvent::WorkflowStarted {
            execution_id: eid,
            workflow_id: "test".into(),
        });
        assert_eq!(c.len(), 1);
        assert!(!c.is_empty());
    }

    #[test]
    fn collector_get_events_returns_all() {
        let c = EventCollector::new();
        let eid = Uuid::new_v4();
        c.record(WorkflowEvent::WorkflowStarted {
            execution_id: eid,
            workflow_id: "wf1".into(),
        });
        c.record(WorkflowEvent::WorkflowCompleted {
            execution_id: eid,
            final_state: "{}".into(),
        });
        let events = c.get_events();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn collector_clear_removes_all() {
        let c = EventCollector::new();
        c.record(WorkflowEvent::WorkflowCancelled {
            execution_id: Uuid::new_v4(),
        });
        c.clear();
        assert!(c.is_empty());
        assert_eq!(c.len(), 0);
    }

    #[test]
    fn collector_default_creates_empty() {
        let c = EventCollector::default();
        assert!(c.is_empty());
    }

    // ── EventEmitter for Arc<EventCollector> tests ──

    #[test]
    fn arc_collector_emit_records() {
        let c = Arc::new(EventCollector::new());
        let eid = Uuid::new_v4();
        c.emit(WorkflowEvent::AgentStarted {
            execution_id: eid,
            agent_id: "a1".into(),
            role: "dev".into(),
        });
        assert_eq!(c.len(), 1);
    }

    #[tokio::test]
    async fn arc_collector_subscribe_receives_events() {
        let c = Arc::new(EventCollector::new());
        let eid = Uuid::new_v4();
        c.emit(WorkflowEvent::StageStarted {
            execution_id: eid,
            stage_name: "s1".into(),
            stage_index: 0,
        });

        let mut rx = c.subscribe();
        let event = rx.recv().await;
        assert!(event.is_some());
        match event.unwrap() {
            WorkflowEvent::StageStarted {
                execution_id,
                stage_name,
                ..
            } => {
                assert_eq!(execution_id, eid);
                assert_eq!(stage_name, "s1");
            }
            _ => panic!("expected StageStarted"),
        }
    }

    // ── WorkflowEvent::execution_id tests ──

    #[test]
    fn execution_id_from_workflow_started() {
        let eid = Uuid::new_v4();
        let event = WorkflowEvent::WorkflowStarted {
            execution_id: eid,
            workflow_id: "wf".into(),
        };
        assert_eq!(event.execution_id(), Some(eid));
    }

    #[test]
    fn execution_id_from_stage_event() {
        let eid = Uuid::new_v4();
        let event = WorkflowEvent::StageCompleted {
            execution_id: eid,
            stage_name: "s1".into(),
            outputs: vec![],
            quality_gate_result: None,
        };
        assert_eq!(event.execution_id(), Some(eid));
    }

    #[test]
    fn execution_id_from_agent_event() {
        let eid = Uuid::new_v4();
        let event = WorkflowEvent::AgentFailed {
            execution_id: eid,
            agent_id: "a1".into(),
            error: "boom".into(),
        };
        assert_eq!(event.execution_id(), Some(eid));
    }

    #[test]
    fn execution_id_from_quality_gate_event() {
        let eid = Uuid::new_v4();
        let event = WorkflowEvent::QualityGateChecked {
            execution_id: eid,
            stage_name: "s1".into(),
            passed: true,
            retry_count: 0,
            checks_summary: "all good".into(),
        };
        assert_eq!(event.execution_id(), Some(eid));
    }

    #[test]
    fn execution_id_from_token_usage() {
        let eid = Uuid::new_v4();
        let event = WorkflowEvent::AgentTokenUsage {
            execution_id: eid,
            agent_id: "a1".into(),
            input_tokens: 100,
            output_tokens: 50,
            executor: "claude_cli".into(),
            provider: "anthropic".into(),
            estimated_cost_usd: 0.001,
        };
        assert_eq!(event.execution_id(), Some(eid));
    }

    // ── Event serialization round-trip ──

    #[test]
    fn event_serialization_roundtrip() {
        let eid = Uuid::new_v4();
        let event = WorkflowEvent::WorkflowPaused {
            execution_id: eid,
            stage_name: "choose".into(),
            question: "Yes or No?".into(),
            options: vec![("Yes".into(), "y".into()), ("No".into(), "n".into())],
            pause_kind: "user_input".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: WorkflowEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            WorkflowEvent::WorkflowPaused {
                stage_name, question, ..
            } => {
                assert_eq!(stage_name, "choose");
                assert_eq!(question, "Yes or No?");
            }
            _ => panic!("expected WorkflowPaused"),
        }
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
