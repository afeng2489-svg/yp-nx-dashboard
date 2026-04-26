//! 崩溃检测器 — 启动时扫描未完成 checkpoint，发送 CrashRecovery 事件
//!
//! 集成到 AppState::new()，应用启动后调用 detect()。

use std::sync::Arc;
use tokio::sync::broadcast;

use crate::ws::agent_execution::AgentExecutionEvent;
use super::resume_service::ResumeService;
use super::error::TeamEvolutionError;

pub struct CrashDetector {
    resume_service: Arc<ResumeService>,
    event_tx: broadcast::Sender<AgentExecutionEvent>,
}

impl CrashDetector {
    pub fn new(
        resume_service: Arc<ResumeService>,
        event_tx: broadcast::Sender<AgentExecutionEvent>,
    ) -> Self {
        Self { resume_service, event_tx }
    }

    /// 启动时检测崩溃：查找所有 interrupted checkpoint 并广播事件
    pub fn detect(&self) -> Result<Vec<CrashRecoveryInfo>, TeamEvolutionError> {
        let interrupted = self.resume_service.find_interrupted()?;

        let mut recoveries = Vec::new();

        for checkpoint in &interrupted {
            // 广播 CrashRecovery 事件
            let _ = self.event_tx.send(AgentExecutionEvent::CrashRecovery {
                execution_id: checkpoint.execution_id.clone(),
                last_output: if checkpoint.accumulated_output.len() > 500 {
                    checkpoint.accumulated_output[..500].to_string()
                } else {
                    checkpoint.accumulated_output.clone()
                },
            });

            recoveries.push(CrashRecoveryInfo {
                execution_id: checkpoint.execution_id.clone(),
                project_id: checkpoint.project_id.clone(),
                role_id: checkpoint.role_id.clone(),
                task_prompt: checkpoint.task_prompt.clone(),
                pipeline_step_id: checkpoint.pipeline_step_id.clone(),
                interrupted_at: checkpoint.last_heartbeat.clone(),
            });
        }

        if !recoveries.is_empty() {
            tracing::warn!(
                "[CrashDetector] 检测到 {} 个中断的执行任务",
                recoveries.len()
            );
        }

        Ok(recoveries)
    }
}

/// 崩溃恢复信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrashRecoveryInfo {
    pub execution_id: String,
    pub project_id: String,
    pub role_id: String,
    pub task_prompt: String,
    pub pipeline_step_id: Option<String>,
    pub interrupted_at: String,
}
