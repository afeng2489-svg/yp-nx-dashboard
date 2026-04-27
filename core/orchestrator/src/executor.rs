//! Workflow Executor - Orchestrates multi-agent workflow execution

use crate::cli::{CliManager, CliRequest};
use crate::error::OrchestratorError;
use crate::message_bus::{Channel, MessageBus, MessagePayload};
use crate::team::{AgentId, TeamId, TeamManager};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub stages: Vec<StageDefinition>,
}

/// Stage in a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageDefinition {
    pub name: String,
    pub agents: Vec<String>,
    pub parallel: bool,
    pub continue_on_error: bool,
    pub prompt_template: String,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub execution_id: Uuid,
    pub workflow_name: String,
    pub status: ExecutionStatus,
    pub stage_results: Vec<StageResult>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Result of a single stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub stage_name: String,
    pub agent_results: Vec<AgentOutput>,
    pub failures: Vec<String>,
}

/// Output from an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub agent_id: AgentId,
    pub agent_name: String,
    pub text: String,
    pub duration_ms: u64,
}

/// Active execution tracking
struct ActiveExecution {
    execution_id: Uuid,
    workflow_name: String,
    status: ExecutionStatus,
    stage_results: Vec<StageResult>,
    started_at: DateTime<Utc>,
}

/// Workflow executor
pub struct WorkflowExecutor {
    cli_manager: Arc<CliManager>,
    team_manager: Arc<TeamManager>,
    message_bus: Arc<MessageBus>,
    active_executions: RwLock<HashMap<Uuid, ActiveExecution>>,
}

impl WorkflowExecutor {
    pub fn new(
        cli_manager: Arc<CliManager>,
        team_manager: Arc<TeamManager>,
        message_bus: Arc<MessageBus>,
    ) -> Self {
        Self {
            cli_manager,
            team_manager,
            message_bus,
            active_executions: RwLock::new(HashMap::new()),
        }
    }

    /// Execute a workflow
    pub async fn execute(
        &self,
        workflow: WorkflowDefinition,
        team_id: TeamId,
    ) -> Result<ExecutionResult, OrchestratorError> {
        let execution_id = Uuid::new_v4();
        let started_at = Utc::now();

        // Track active execution
        {
            let mut executions = self.active_executions.write();
            executions.insert(
                execution_id,
                ActiveExecution {
                    execution_id,
                    workflow_name: workflow.name.clone(),
                    status: ExecutionStatus::Running,
                    stage_results: Vec::new(),
                    started_at,
                },
            );
        }

        // Publish execution started
        let _ = self.message_bus.publish(
            Channel::AgentEvents,
            MessagePayload::AgentStarted {
                agent_id: AgentId::default(),
            },
        );

        let mut stage_results = Vec::new();

        // Execute each stage
        for stage in &workflow.stages {
            tracing::info!("Executing stage: {}", stage.name);

            let stage_result = if stage.parallel {
                self.execute_parallel_stage(stage, team_id).await
            } else {
                self.execute_sequential_stage(stage, team_id).await
            };

            stage_results.push(stage_result);

            // Check if any stage failed and we should stop
            let has_failures = stage_results
                .last()
                .map(|r| !r.failures.is_empty())
                .unwrap_or(false);

            if has_failures && !stage.continue_on_error {
                break;
            }
        }

        let finished_at = Utc::now();
        let duration_ms = (finished_at - started_at).num_milliseconds() as u64;

        let status = if stage_results.iter().all(|r| r.failures.is_empty()) {
            ExecutionStatus::Completed
        } else {
            ExecutionStatus::Failed
        };

        // Update active execution
        {
            let mut executions = self.active_executions.write();
            if let Some(exec) = executions.get_mut(&execution_id) {
                exec.status = status;
                exec.stage_results = stage_results.clone();
            }
        }

        Ok(ExecutionResult {
            execution_id,
            workflow_name: workflow.name,
            status,
            stage_results,
            started_at,
            finished_at: Some(finished_at),
            duration_ms: Some(duration_ms),
        })
    }

    /// Execute stage with agents in parallel
    async fn execute_parallel_stage(
        &self,
        stage: &StageDefinition,
        team_id: TeamId,
    ) -> StageResult {
        use tokio::task::JoinSet;

        let mut join_set = JoinSet::new();

        // Get team members for this stage
        let team = match self.team_manager.get_team(team_id) {
            Some(t) => t,
            None => {
                return StageResult {
                    stage_name: stage.name.clone(),
                    agent_results: Vec::new(),
                    failures: vec!["Team not found".to_string()],
                };
            }
        };

        for agent_name in &stage.agents {
            if let Some(member) = team.members.values().find(|m| m.name == *agent_name) {
                let member = member.clone();
                let prompt = stage.prompt_template.clone();
                let cli = self.cli_manager.clone();
                let exec = self.message_bus.clone();

                join_set.spawn(async move {
                    Self::execute_agent_task_internal(&member, &prompt, &cli, &exec).await
                });
            }
        }

        let mut agent_results = Vec::new();
        let mut failures = Vec::new();

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(output)) => agent_results.push(output),
                Ok(Err(e)) => failures.push(e),
                Err(e) => failures.push(format!("Task join error: {}", e)),
            }
        }

        StageResult {
            stage_name: stage.name.clone(),
            agent_results,
            failures,
        }
    }

    /// Execute stage with agents sequentially
    async fn execute_sequential_stage(
        &self,
        stage: &StageDefinition,
        team_id: TeamId,
    ) -> StageResult {
        let team = match self.team_manager.get_team(team_id) {
            Some(t) => t,
            None => {
                return StageResult {
                    stage_name: stage.name.clone(),
                    agent_results: Vec::new(),
                    failures: vec!["Team not found".to_string()],
                };
            }
        };

        let mut agent_results = Vec::new();
        let mut failures = Vec::new();

        for agent_name in &stage.agents {
            if let Some(member) = team.members.values().find(|m| m.name == *agent_name) {
                let result = Self::execute_agent_task_internal(
                    member,
                    &stage.prompt_template,
                    &self.cli_manager,
                    &self.message_bus,
                )
                .await;

                match result {
                    Ok(output) => agent_results.push(output),
                    Err(e) => {
                        failures.push(e.clone());
                        if !stage.continue_on_error {
                            break;
                        }
                    }
                }
            }
        }

        StageResult {
            stage_name: stage.name.clone(),
            agent_results,
            failures,
        }
    }

    /// Internal agent task execution
    async fn execute_agent_task_internal(
        member: &crate::team::TeamMember,
        prompt_template: &str,
        cli_manager: &Arc<CliManager>,
        _message_bus: &Arc<MessageBus>,
    ) -> Result<AgentOutput, String> {
        let start = std::time::Instant::now();

        let request = CliRequest {
            provider: member.provider,
            prompt: prompt_template.to_string(),
            system_prompt: Some(member.role.default_prompt().to_string()),
            working_dir: None,
            env_vars: HashMap::new(),
            timeout_secs: Some(member.timeout_secs),
            stream: false,
        };

        let response = cli_manager
            .execute(request)
            .await
            .map_err(|e| e.to_string())?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(AgentOutput {
            agent_id: member.id,
            agent_name: member.name.clone(),
            text: response.text,
            duration_ms,
        })
    }

    /// Get execution status
    pub fn get_execution(&self, execution_id: Uuid) -> Option<ExecutionResult> {
        let executions = self.active_executions.read();
        executions.get(&execution_id).map(|exec| ExecutionResult {
            execution_id: exec.execution_id,
            workflow_name: exec.workflow_name.clone(),
            status: exec.status,
            stage_results: exec.stage_results.clone(),
            started_at: exec.started_at,
            finished_at: None,
            duration_ms: None,
        })
    }

    /// Cancel an execution
    pub fn cancel_execution(&self, execution_id: Uuid) -> bool {
        let mut executions = self.active_executions.write();
        if let Some(exec) = executions.get_mut(&execution_id) {
            exec.status = ExecutionStatus::Cancelled;
            return true;
        }
        false
    }
}
