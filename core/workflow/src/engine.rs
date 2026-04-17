//! 工作流引擎
//!
//! 工作流的核心执行引擎。

use std::sync::Arc;
use std::process::Stdio;
use parking_lot::RwLock;
use tokio::process::Command;

use crate::{WorkflowDefinition, WorkflowState, WorkflowStatus, StageOutput, AgentState, AgentStatus};
use crate::events::{EventEmitter, WorkflowEvent};
use crate::parser::WorkflowError as ParserWorkflowError;
use nexus_ai::ChatMessage;

/// 共享工作流状态
type SharedState = Arc<RwLock<WorkflowState>>;

/// 工作流执行引擎
pub struct WorkflowEngine {
    /// 事件发射器
    event_emitter: Arc<dyn EventEmitter>,
    /// 工作目录（用于 Claude CLI --project 参数）
    working_directory: Option<String>,
}

impl WorkflowEngine {
    /// 创建新的工作流引擎
    pub fn new(event_emitter: Arc<dyn EventEmitter>) -> Self {
        Self {
            event_emitter,
            working_directory: None,
        }
    }

    /// 创建带工作目录的工作流引擎
    pub fn with_working_directory(event_emitter: Arc<dyn EventEmitter>, working_directory: Option<String>) -> Self {
        Self {
            event_emitter,
            working_directory,
        }
    }

    /// 执行工作流
    pub async fn execute(&self, workflow: &WorkflowDefinition) -> Result<WorkflowResult, WorkflowError> {
        let state: SharedState = Arc::new(RwLock::new(WorkflowState::new(&workflow.name)));

        {
            let s = state.read();
            self.event_emitter.emit(WorkflowEvent::WorkflowStarted {
                execution_id: s.execution_id,
                workflow_id: workflow.name.clone(),
            });
        }

        state.write().start();

        // 从工作流定义初始化变量
        for (key, value) in &workflow.variables {
            state.write().set_var(key, value.clone());
        }

        // 顺序执行阶段
        for (stage_idx, stage) in workflow.stages.iter().enumerate() {
            if state.read().should_stop() {
                break;
            }

            {
                let s = state.read();
                self.event_emitter.emit(WorkflowEvent::StageStarted {
                    execution_id: s.execution_id,
                    stage_name: stage.name.clone(),
                    stage_index: stage_idx,
                });
            }

            // 执行阶段（带 on_error 重试逻辑）
            let outputs = match self.execute_stage(&state, stage, &workflow.agents).await {
                Ok(outputs) => outputs,
                Err(e) => {
                    if let Some(ref error_handler) = workflow.on_error {
                        if error_handler.retry {
                            let mut last_err = e;
                            let mut retry_result = None;
                            for attempt in 1..=error_handler.max_retries {
                                tracing::warn!(
                                    "Stage '{}' 失败，重试 {}/{}",
                                    stage.name, attempt, error_handler.max_retries
                                );
                                match self.execute_stage(&state, stage, &workflow.agents).await {
                                    Ok(outputs) => {
                                        retry_result = Some(outputs);
                                        break;
                                    }
                                    Err(e) => {
                                        last_err = e;
                                    }
                                }
                            }
                            match retry_result {
                                Some(outputs) => outputs,
                                None => return Err(last_err),
                            }
                        } else {
                            return Err(e);
                        }
                    } else {
                        return Err(e);
                    }
                }
            };

            {
                let mut s = state.write();
                s.record_stage(&stage.name, outputs);
            }

            {
                let s = state.read();
                self.event_emitter.emit(WorkflowEvent::StageCompleted {
                    execution_id: s.execution_id,
                    stage_name: stage.name.clone(),
                    outputs: s.stage_results.last().unwrap().outputs.clone(),
                });
            }
        }

        let mut s = state.write();
        if s.status == WorkflowStatus::Running {
            s.complete();
            self.event_emitter.emit(WorkflowEvent::WorkflowCompleted {
                execution_id: s.execution_id,
                final_state: serde_json::to_string(&s.variables).unwrap_or_default(),
            });
        }

        Ok(WorkflowResult {
            execution_id: s.execution_id,
            status: s.status,
            variables: s.variables.clone(),
            stage_results: s.stage_results.clone(),
        })
    }

    /// 执行单个阶段
    async fn execute_stage(
        &self,
        state: &SharedState,
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

                let state_clone = Arc::clone(state);
                let agent_clone = agent.clone();
                let engine = self.clone();

                handles.push(tokio::spawn(async move {
                    engine.execute_agent(&state_clone, &agent_clone).await
                }));
            }

            let mut outputs = Vec::new();
            let mut errors = Vec::new();
            for handle in handles {
                match handle.await {
                    Ok(Ok(agent_outputs)) => outputs.extend(agent_outputs),
                    Ok(Err(e)) => errors.push(e),
                    Err(e) => errors.push(WorkflowError::Execution(format!("任务 panic: {}", e))),
                }
            }

            if !errors.is_empty() && !stage.continue_on_error {
                return Err(errors.into_iter().next().unwrap());
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

                match self.execute_agent(state, agent).await {
                    Ok(agent_outputs) => outputs.extend(agent_outputs),
                    Err(e) => {
                        if stage.continue_on_error {
                            tracing::warn!("智能体 {} 失败但继续执行: {}", agent_id, e);
                        } else {
                            return Err(e);
                        }
                    }
                }
            }

            Ok(outputs)
        }
    }

    /// 检查所有依赖是否满足
    fn check_dependencies(&self, agent: &crate::parser::AgentDefinition, state: &SharedState) -> Result<bool, WorkflowError> {
        if agent.depends_on.is_empty() {
            return Ok(true);
        }
        let state_read = state.read();
        for dep_id in &agent.depends_on {
            if let Some(dep_state) = state_read.agent_states.get(dep_id) {
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
        state: &SharedState,
        agent: &crate::parser::AgentDefinition,
    ) -> Result<Vec<StageOutput>, WorkflowError> {
        let execution_id = state.read().execution_id;

        let mut agent_state = AgentState {
            agent_id: agent.id.clone(),
            role: agent.role.clone(),
            status: AgentStatus::Running,
            last_message: None,
            updated_at: chrono::Utc::now(),
        };

        // 写入 Running 状态
        state.write().update_agent(&agent.id, agent_state.clone());

        self.event_emitter.emit(WorkflowEvent::AgentStarted {
            execution_id,
            agent_id: agent.id.clone(),
            role: agent.role.clone(),
        });

        // 使用解析后的变量构建提示词
        let resolved_prompt = state.read().resolve_template(&agent.prompt);

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
                agent_state.updated_at = chrono::Utc::now();

                // 写回完成状态
                state.write().update_agent(&agent.id, agent_state);

                self.event_emitter.emit(WorkflowEvent::AgentCompleted {
                    execution_id,
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
                agent_state.updated_at = chrono::Utc::now();

                // 写回失败状态
                state.write().update_agent(&agent.id, agent_state);

                self.event_emitter.emit(WorkflowEvent::AgentFailed {
                    execution_id,
                    agent_id: agent.id.clone(),
                    error: e.to_string(),
                });

                Err(WorkflowError::Execution(format!("智能体 {} 失败: {}", agent.id, e)))
            }
        }
    }

    /// 调用 Claude CLI
    async fn call_claude_cli(&self, prompt: &str) -> Result<String, WorkflowError> {
        let mut cmd = Command::new("claude");
        cmd.args(["-p", "--dangerously-skip-permissions", prompt]);

        // 如果设置了 working_directory，设置当前工作目录
        if let Some(ref dir) = self.working_directory {
            cmd.current_dir(dir);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
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
            working_directory: self.working_directory.clone(),
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
