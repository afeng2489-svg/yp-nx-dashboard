//! 工作流事件桥接器
//!
//! 将 WorkflowEvent 桥接到 ExecutionEvent，实现真正的 AI 执行与 API 的集成。

use std::sync::Arc;
use tokio::sync::broadcast;
use nexus_workflow::events::{EventEmitter, WorkflowEvent};
use crate::services::events::{ExecutionEvent, WorkflowOption};
use crate::services::ExecutionService;

/// 工作流事件桥接器
///
/// 将 WorkflowEngine 发出的 WorkflowEvent 转换为 ExecutionEvent，
/// 并通过 ExecutionService 的广播通道发送。
pub struct WorkflowEventBridge {
    execution_service: ExecutionService,
}

impl WorkflowEventBridge {
    /// 创建新的桥接器
    pub fn new(execution_service: ExecutionService) -> Self {
        Self { execution_service }
    }

    /// 将 WorkflowEvent 转换为 ExecutionEvent 并广播
    fn convert_and_broadcast(&self, event: WorkflowEvent) {
        let execution_event = match event {
            WorkflowEvent::WorkflowStarted { execution_id, workflow_id } => {
                Some(ExecutionEvent::Started {
                    execution_id: execution_id.to_string(),
                    workflow_id,
                })
            }
            WorkflowEvent::StageStarted { execution_id, stage_name, .. } => {
                Some(ExecutionEvent::StageStarted {
                    execution_id: execution_id.to_string(),
                    stage_name,
                })
            }
            WorkflowEvent::StageCompleted { execution_id, stage_name, outputs } => {
                Some(ExecutionEvent::StageCompleted {
                    execution_id: execution_id.to_string(),
                    stage_name,
                    output: serde_json::json!({ "outputs": outputs }),
                })
            }
            WorkflowEvent::AgentStarted { execution_id, agent_id, .. } => {
                Some(ExecutionEvent::Output {
                    execution_id: execution_id.to_string(),
                    line: format!("[Agent {}] Started", agent_id),
                })
            }
            WorkflowEvent::AgentMessage { execution_id, agent_id, message } => {
                Some(ExecutionEvent::Output {
                    execution_id: execution_id.to_string(),
                    line: format!("[Agent {}] {}", agent_id, message),
                })
            }
            WorkflowEvent::AgentCompleted { execution_id, agent_id, output } => {
                let preview: String = output.chars().take(100).collect();
                Some(ExecutionEvent::Output {
                    execution_id: execution_id.to_string(),
                    line: format!("[Agent {}] Completed: {}", agent_id, preview),
                })
            }
            WorkflowEvent::AgentFailed { execution_id, agent_id, error } => {
                Some(ExecutionEvent::Failed {
                    execution_id: execution_id.to_string(),
                    error: format!("Agent {} failed: {}", agent_id, error),
                })
            }
            WorkflowEvent::WorkflowCompleted { .. } => {
                // 完成状态由 ExecutionService 自行设置
                None
            }
            WorkflowEvent::WorkflowPaused { execution_id, stage_name, question, options } => {
                Some(ExecutionEvent::WorkflowPaused {
                    execution_id: execution_id.to_string(),
                    stage_name,
                    question,
                    options: options
                        .into_iter()
                        .map(|(label, value)| WorkflowOption { label, value })
                        .collect(),
                })
            }
            WorkflowEvent::WorkflowResumed { execution_id, stage_name, chosen_value } => {
                Some(ExecutionEvent::WorkflowResumed {
                    execution_id: execution_id.to_string(),
                    stage_name,
                    chosen_value,
                })
            }
            WorkflowEvent::WorkflowFailed { execution_id, error } => {
                Some(ExecutionEvent::Failed {
                    execution_id: execution_id.to_string(),
                    error: format!("Workflow failed: {}", error),
                })
            }
            WorkflowEvent::WorkflowCancelled { execution_id } => {
                Some(ExecutionEvent::Failed {
                    execution_id: execution_id.to_string(),
                    error: "Workflow cancelled".to_string(),
                })
            }
            WorkflowEvent::VariableSet { execution_id, key, value } => {
                Some(ExecutionEvent::Output {
                    execution_id: execution_id.to_string(),
                    line: format!("[Variable] {} = {}", key, value),
                })
            }
        };

        if let Some(event) = execution_event {
            self.execution_service.broadcast(event);
        }
    }
}

impl EventEmitter for WorkflowEventBridge {
    fn emit(&self, event: WorkflowEvent) {
        self.convert_and_broadcast(event);
    }

    fn subscribe(&self) -> tokio::sync::mpsc::Receiver<WorkflowEvent> {
        // 桥接器不需要订阅 WorkflowEvent，因为我们直接转换为 ExecutionEvent
        // 返回一个永远关闭的 receiver
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        drop(tx);
        rx
    }
}
