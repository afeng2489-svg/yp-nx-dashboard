//! 工作流引擎
//!
//! 工作流的核心执行引擎。

use std::sync::Arc;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::{WorkflowDefinition, WorkflowState, WorkflowStatus, StageOutput, AgentState, AgentStatus};
use crate::events::{EventEmitter, WorkflowEvent};
use crate::parser::WorkflowError as ParserWorkflowError;
use nexus_ai::ChatMessage;

/// 工作流执行引擎
pub struct WorkflowEngine {
    /// 事件发射器
    event_emitter: Arc<dyn EventEmitter>,
}

impl WorkflowEngine {
    /// 创建新的工作流引擎
    pub fn new(event_emitter: Arc<dyn EventEmitter>) -> Self {
        Self {
            event_emitter,
        }
    }

    /// 执行工作流
    pub async fn execute(&self, workflow: &WorkflowDefinition) -> Result<WorkflowResult, WorkflowError> {
        let mut state = WorkflowState::new(&workflow.name);

        self.event_emitter.emit(WorkflowEvent::WorkflowStarted {
            execution_id: state.execution_id,
            workflow_id: workflow.name.clone(),
        });

        state.start();

        // 从工作流定义初始化变量
        for (key, value) in &workflow.variables {
            state.set_var(key, value.clone());
        }

        // 顺序执行阶段
        for (stage_idx, stage) in workflow.stages.iter().enumerate() {
            if state.should_stop() {
                break;
            }

            self.event_emitter.emit(WorkflowEvent::StageStarted {
                execution_id: state.execution_id,
                stage_name: stage.name.clone(),
                stage_index: stage_idx,
            });

            // 从状态解析阶段输出
            let outputs = self.execute_stage(&state, stage, &workflow.agents).await?;
            state.record_stage(&stage.name, outputs);

            self.event_emitter.emit(WorkflowEvent::StageCompleted {
                execution_id: state.execution_id,
                stage_name: stage.name.clone(),
                outputs: state.stage_results.last().unwrap().outputs.clone(),
            });
        }

        if state.status == WorkflowStatus::Running {
            state.complete();
            self.event_emitter.emit(WorkflowEvent::WorkflowCompleted {
                execution_id: state.execution_id,
                final_state: serde_json::to_string(&state.variables).unwrap_or_default(),
            });
        }

        Ok(WorkflowResult {
            execution_id: state.execution_id,
            status: state.status,
            variables: state.variables,
            stage_results: state.stage_results,
        })
    }

    /// 执行单个阶段
    async fn execute_stage(
        &self,
        state: &WorkflowState,
        stage: &crate::parser::StageDefinition,
        agents: &[crate::parser::AgentDefinition],
    ) -> Result<Vec<StageOutput>, WorkflowError> {
        if stage.parallel {
            // 并行执行智能体
            let mut handles = Vec::new();
            for agent_id in &stage.agents {
                let agent = agents.iter().find(|a| &a.id == agent_id)
                    .ok_or_else(|| WorkflowError::Validation(format!("未找到智能体: {}", agent_id)))?;

                // 检查依赖
                if !self.check_dependencies(agent, state)? {
                    continue;
                }

                let state_clone = state.clone();
                let agent_clone = agent.clone();
                let engine = self.clone();

                handles.push(tokio::spawn(async move {
                    engine.execute_agent(&state_clone, &agent_clone).await
                }));
            }

            let mut outputs = Vec::new();
            for handle in handles {
                if let Ok(output) = handle.await {
                    if let Ok(outputs_result) = output {
                        outputs.extend(outputs_result);
                    }
                }
            }

            Ok(outputs)
        } else {
            // 顺序执行智能体
            let mut outputs = Vec::new();
            for agent_id in &stage.agents {
                let agent = agents.iter().find(|a| &a.id == agent_id)
                    .ok_or_else(|| WorkflowError::Validation(format!("未找到智能体: {}", agent_id)))?;

                // 检查依赖
                if !self.check_dependencies(agent, state)? {
                    continue;
                }

                let agent_outputs = self.execute_agent(state, agent).await?;
                outputs.extend(agent_outputs);
            }

            Ok(outputs)
        }
    }

    /// 检查所有依赖是否满足
    fn check_dependencies(&self, agent: &crate::parser::AgentDefinition, state: &WorkflowState) -> Result<bool, WorkflowError> {
        for dep_id in &agent.depends_on {
            if let Some(dep_state) = state.agent_states.get(dep_id) {
                if dep_state.status != AgentStatus::Completed {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// 执行单个智能体
    async fn execute_agent(
        &self,
        state: &WorkflowState,
        agent: &crate::parser::AgentDefinition,
    ) -> Result<Vec<StageOutput>, WorkflowError> {
        let mut agent_state = AgentState {
            agent_id: agent.id.clone(),
            role: agent.role.clone(),
            status: AgentStatus::Running,
            last_message: None,
            updated_at: chrono::Utc::now(),
        };

        self.event_emitter.emit(WorkflowEvent::AgentStarted {
            execution_id: state.execution_id,
            agent_id: agent.id.clone(),
            role: agent.role.clone(),
        });

        // 使用解析后的变量构建提示词
        let resolved_prompt = state.resolve_template(&agent.prompt);

        // Auto-yes prefix to skip confirmation prompts
        let auto_yes_prefix = "You are operating in auto-yes mode. If you ask any question requiring confirmation (yes/no, continue?, proceed?, etc.), always assume the answer is YES and proceed automatically. Never ask for confirmation.";

        // 构建 prompt（Claude CLI 格式）
        let full_prompt = format!(
            "{}\n\n<system>\n你扮演 {}. 请仔细遵循你的指示。\n</system>\n\n<user>\n{}\n</user>",
            auto_yes_prefix, agent.role, resolved_prompt
        );

        // 通过 Claude CLI 执行（Claude Switch 切换后自动使用新模型）
        let output = self.call_claude_cli(&full_prompt).await;

        match output {
            Ok(response) => {
                agent_state.status = AgentStatus::Completed;
                agent_state.last_message = Some(response.clone());

                self.event_emitter.emit(WorkflowEvent::AgentCompleted {
                    execution_id: state.execution_id,
                    agent_id: agent.id.clone(),
                    output: response.clone(),
                });

                Ok(vec![StageOutput {
                    path: format!("agent://{}/output", agent.id),
                    content: Some(response),
                    agent_id: Some(agent.id.clone()),
                }])
            }
            Err(e) => {
                agent_state.status = AgentStatus::Failed;

                self.event_emitter.emit(WorkflowEvent::AgentFailed {
                    execution_id: state.execution_id,
                    agent_id: agent.id.clone(),
                    error: e.to_string(),
                });

                Err(WorkflowError::Execution(format!("智能体 {} 失败: {}", agent.id, e)))
            }
        }
    }

    /// 调用 Claude CLI
    async fn call_claude_cli(&self, prompt: &str) -> Result<String, WorkflowError> {
        let mut child = Command::new("claude")
            .args(["-p", prompt])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| WorkflowError::Execution(format!("Failed to spawn Claude CLI: {}", e)))?;

        let output = child.wait_with_output().await
            .map_err(|e| WorkflowError::Execution(format!("Claude CLI error: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WorkflowError::Execution(format!("Claude CLI error: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

impl Clone for WorkflowEngine {
    fn clone(&self) -> Self {
        Self {
            event_emitter: self.event_emitter.clone(),
        }
    }
}

/// 工作流执行结果
#[derive(Debug, Clone)]
pub struct WorkflowResult {
    pub execution_id: uuid::Uuid,
    pub status: WorkflowStatus,
    pub variables: std::collections::HashMap<String, serde_json::Value>,
    pub stage_results: Vec<crate::StageResult>,
}

/// 工作流执行错误
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("解析错误: {0}")]
    Parse(String),

    #[error("验证错误: {0}")]
    Validation(String),

    #[error("执行错误: {0}")]
    Execution(String),

    #[error("IO 错误: {0}")]
    Io(String),
}

impl From<ParserWorkflowError> for WorkflowError {
    fn from(e: ParserWorkflowError) -> Self {
        match e {
            ParserWorkflowError::Parse(s) => WorkflowError::Parse(s),
            ParserWorkflowError::Validation(s) => WorkflowError::Validation(s),
            ParserWorkflowError::Io(s) => WorkflowError::Io(s),
        }
    }
}

impl From<nexus_ai::AIError> for WorkflowError {
    fn from(e: nexus_ai::AIError) -> Self {
        WorkflowError::Execution(e.to_string())
    }
}
