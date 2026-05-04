//! 工作流事件桥接器
//!
//! 将 WorkflowEvent 桥接到 ExecutionEvent，实现真正的 AI 执行与 API 的集成。

use crate::services::events::{ExecutionEvent, WorkflowOption};
use crate::services::ExecutionService;
use nexus_workflow::events::{EventEmitter, WorkflowEvent};
use std::sync::Arc;
use tokio::sync::broadcast;

/// 工作流事件桥接器
///
/// 将 WorkflowEngine 发出的 WorkflowEvent 转换为 ExecutionEvent，
/// 并通过 ExecutionService 的广播通道发送。
///
/// `api_exec_id` 是 ExecutionService 分配的 UUID，用于替换
/// WorkflowEngine 内部生成的 execution_id，保证事件能匹配到
/// 正确的 Execution 记录。
pub struct WorkflowEventBridge {
    execution_service: ExecutionService,
    api_exec_id: String,
    budget_limit_usd: Option<f64>,
}

impl WorkflowEventBridge {
    pub fn new(execution_service: ExecutionService, api_exec_id: String) -> Self {
        Self {
            execution_service,
            api_exec_id,
            budget_limit_usd: None,
        }
    }

    pub fn with_budget(mut self, limit: f64) -> Self {
        self.budget_limit_usd = Some(limit);
        self
    }

    /// 将 WorkflowEvent 转换为 ExecutionEvent 并广播
    ///
    /// 始终使用 `api_exec_id` 替换 WorkflowEngine 内部的 execution_id。
    fn convert_and_broadcast(&self, event: WorkflowEvent) {
        let id = self.api_exec_id.clone();
        let execution_event = match event {
            WorkflowEvent::WorkflowStarted { workflow_id, .. } => Some(ExecutionEvent::Started {
                execution_id: id,
                workflow_id,
            }),
            WorkflowEvent::StageStarted { stage_name, .. } => Some(ExecutionEvent::StageStarted {
                execution_id: id,
                stage_name,
            }),
            WorkflowEvent::StageCompleted {
                stage_name,
                outputs,
                quality_gate_result,
                ..
            } => {
                let mut output_obj =
                    serde_json::json!({ "stage": stage_name.clone(), "outputs": outputs });
                let qg_value = quality_gate_result
                    .as_ref()
                    .map(|qg| serde_json::to_value(qg).unwrap_or_default());
                if let Some(ref qg) = qg_value {
                    output_obj["quality_gate_result"] = qg.clone();
                }
                self.execution_service.add_stage_output_with_gate(
                    &id,
                    stage_name.clone(),
                    output_obj.clone(),
                    qg_value.clone(),
                );
                Some(ExecutionEvent::StageCompleted {
                    execution_id: id,
                    stage_name,
                    output: output_obj,
                    quality_gate_result: qg_value,
                })
            }
            WorkflowEvent::AgentStarted { agent_id, role, .. } => Some(ExecutionEvent::Output {
                execution_id: id,
                line: format!("[Agent {}] 开始执行（{}）", agent_id, role),
            }),
            WorkflowEvent::AgentMessage {
                agent_id, message, ..
            } => Some(ExecutionEvent::Output {
                execution_id: id,
                line: format!("[Agent {}] {}", agent_id, message),
            }),
            WorkflowEvent::AgentCompleted {
                agent_id, output, ..
            } => {
                self.execution_service.add_stage_output(
                    &id,
                    format!("agent:{}", agent_id),
                    serde_json::json!({ "agent_id": agent_id, "content": output }),
                );
                Some(ExecutionEvent::Output {
                    execution_id: id,
                    line: format!("[Agent {}]\n{}", agent_id, output),
                })
            }
            WorkflowEvent::AgentFailed {
                agent_id, error, ..
            } => Some(ExecutionEvent::Failed {
                execution_id: id,
                error: format!("Agent {} failed: {}", agent_id, error),
            }),
            WorkflowEvent::WorkflowCompleted { .. } => None,
            WorkflowEvent::WorkflowPaused {
                stage_name,
                question,
                options,
                ..
            } => Some(ExecutionEvent::WorkflowPaused {
                execution_id: id,
                stage_name,
                question,
                options: options
                    .into_iter()
                    .map(|(label, value)| WorkflowOption { label, value })
                    .collect(),
            }),
            WorkflowEvent::WorkflowResumed {
                stage_name,
                chosen_value,
                ..
            } => Some(ExecutionEvent::WorkflowResumed {
                execution_id: id,
                stage_name,
                chosen_value,
            }),
            WorkflowEvent::WorkflowFailed { error, .. } => Some(ExecutionEvent::Failed {
                execution_id: id,
                error: format!("Workflow failed: {}", error),
            }),
            WorkflowEvent::WorkflowCancelled { .. } => Some(ExecutionEvent::Failed {
                execution_id: id,
                error: "Workflow cancelled".to_string(),
            }),
            WorkflowEvent::VariableSet { key, value, .. } => Some(ExecutionEvent::Output {
                execution_id: id,
                line: format!("[Variable] {} = {}", key, value),
            }),
            WorkflowEvent::QualityGateChecked {
                stage_name,
                passed,
                retry_count,
                checks_summary,
                ..
            } => Some(ExecutionEvent::Output {
                execution_id: id,
                line: if passed {
                    format!("[质量门] {} ✅ 通过", stage_name)
                } else {
                    format!(
                        "[质量门] {} ❌ 失败 (重试 {}/{} ) — {}",
                        stage_name, retry_count, "max", checks_summary
                    )
                },
            }),
            WorkflowEvent::AgentTokenUsage {
                agent_id,
                input_tokens,
                output_tokens,
                ..
            } => {
                let total_tokens = (input_tokens + output_tokens) as i64;
                let cost_usd = (input_tokens as f64 * 3.0 / 1_000_000.0)
                    + (output_tokens as f64 * 15.0 / 1_000_000.0);
                self.execution_service.add_token_usage_with_budget(
                    &id,
                    total_tokens,
                    cost_usd,
                    self.budget_limit_usd,
                );
                Some(ExecutionEvent::Output {
                    execution_id: id,
                    line: format!(
                        "[TokenUsage] {} — input: {}, output: {}, cost: ${:.4}",
                        agent_id, input_tokens, output_tokens, cost_usd
                    ),
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
