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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::team::AgentId;

    // ── ExecutionStatus ─────────────────────────────────────────────

    #[test]
    fn test_execution_status_variants() {
        let variants: Vec<ExecutionStatus> = vec![
            ExecutionStatus::Pending,
            ExecutionStatus::Running,
            ExecutionStatus::Completed,
            ExecutionStatus::Failed,
            ExecutionStatus::Cancelled,
        ];
        assert_eq!(variants.len(), 5);
    }

    #[test]
    fn test_execution_status_equality() {
        assert_eq!(ExecutionStatus::Pending, ExecutionStatus::Pending);
        assert_eq!(ExecutionStatus::Running, ExecutionStatus::Running);
        assert_eq!(ExecutionStatus::Completed, ExecutionStatus::Completed);
        assert_eq!(ExecutionStatus::Failed, ExecutionStatus::Failed);
        assert_eq!(ExecutionStatus::Cancelled, ExecutionStatus::Cancelled);
        assert_ne!(ExecutionStatus::Running, ExecutionStatus::Completed);
    }

    #[test]
    fn test_execution_status_serde_roundtrip() {
        let variants = [
            ExecutionStatus::Pending,
            ExecutionStatus::Running,
            ExecutionStatus::Completed,
            ExecutionStatus::Failed,
            ExecutionStatus::Cancelled,
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let back: ExecutionStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*v, back);
        }
    }

    // ── AgentOutput ─────────────────────────────────────────────────

    #[test]
    fn test_agent_output_serde_roundtrip() {
        let output = AgentOutput {
            agent_id: AgentId::new(),
            agent_name: "architect".into(),
            text: "design complete".into(),
            duration_ms: 5000,
        };

        let json = serde_json::to_string(&output).unwrap();
        let back: AgentOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(back.agent_name, "architect");
        assert_eq!(back.text, "design complete");
        assert_eq!(back.duration_ms, 5000);
    }

    #[test]
    fn test_agent_output_zero_duration() {
        let output = AgentOutput {
            agent_id: AgentId::new(),
            agent_name: "fast".into(),
            text: "done".into(),
            duration_ms: 0,
        };
        assert_eq!(output.duration_ms, 0);
    }

    #[test]
    fn test_agent_output_empty_text() {
        let output = AgentOutput {
            agent_id: AgentId::new(),
            agent_name: "silent".into(),
            text: "".into(),
            duration_ms: 100,
        };
        assert!(output.text.is_empty());
    }

    // ── StageResult ─────────────────────────────────────────────────

    #[test]
    fn test_stage_result_empty() {
        let sr = StageResult {
            stage_name: "test".into(),
            agent_results: vec![],
            failures: vec![],
        };
        assert_eq!(sr.stage_name, "test");
        assert!(sr.agent_results.is_empty());
        assert!(sr.failures.is_empty());
    }

    #[test]
    fn test_stage_result_with_failures() {
        let sr = StageResult {
            stage_name: "build".into(),
            agent_results: vec![],
            failures: vec!["compilation error".into()],
        };
        assert_eq!(sr.failures.len(), 1);
        assert!(sr.agent_results.is_empty());
    }

    #[test]
    fn test_stage_result_serde_roundtrip() {
        let sr = StageResult {
            stage_name: "analyze".into(),
            agent_results: vec![AgentOutput {
                agent_id: AgentId::new(),
                agent_name: "researcher".into(),
                text: "analysis".into(),
                duration_ms: 1000,
            }],
            failures: vec![],
        };

        let json = serde_json::to_string(&sr).unwrap();
        let back: StageResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.stage_name, "analyze");
        assert_eq!(back.agent_results.len(), 1);
    }

    // ── ExecutionResult ─────────────────────────────────────────────

    #[test]
    fn test_execution_result_serde_roundtrip() {
        let result = ExecutionResult {
            execution_id: Uuid::new_v4(),
            workflow_name: "test-workflow".into(),
            status: ExecutionStatus::Completed,
            stage_results: vec![],
            started_at: Utc::now(),
            finished_at: Some(Utc::now()),
            duration_ms: Some(1000),
        };

        let json = serde_json::to_string(&result).unwrap();
        let back: ExecutionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.workflow_name, "test-workflow");
        assert_eq!(back.status, ExecutionStatus::Completed);
        assert!(back.duration_ms.is_some());
    }

    #[test]
    fn test_execution_result_not_finished() {
        let result = ExecutionResult {
            execution_id: Uuid::new_v4(),
            workflow_name: "running".into(),
            status: ExecutionStatus::Running,
            stage_results: vec![],
            started_at: Utc::now(),
            finished_at: None,
            duration_ms: None,
        };

        assert!(result.finished_at.is_none());
        assert!(result.duration_ms.is_none());
    }

    #[test]
    fn test_execution_result_with_stages() {
        let stages = vec![
            StageResult {
                stage_name: "init".into(),
                agent_results: vec![],
                failures: vec![],
            },
            StageResult {
                stage_name: "build".into(),
                agent_results: vec![AgentOutput {
                    agent_id: AgentId::new(),
                    agent_name: "builder".into(),
                    text: "built".into(),
                    duration_ms: 2000,
                }],
                failures: vec![],
            },
        ];

        let result = ExecutionResult {
            execution_id: Uuid::new_v4(),
            workflow_name: "multi-stage".into(),
            status: ExecutionStatus::Completed,
            stage_results: stages,
            started_at: Utc::now(),
            finished_at: Some(Utc::now()),
            duration_ms: Some(2000),
        };

        assert_eq!(result.stage_results.len(), 2);
        assert_eq!(result.stage_results[0].stage_name, "init");
        assert_eq!(result.stage_results[1].stage_name, "build");
    }

    // ── WorkflowDefinition & StageDefinition ────────────────────────

    #[test]
    fn test_workflow_definition_serde_roundtrip() {
        let wf = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "test".into(),
            description: "a test workflow".into(),
            stages: vec![StageDefinition {
                name: "stage1".into(),
                agents: vec!["agent-a".into()],
                parallel: false,
                continue_on_error: true,
                prompt_template: "do the thing".into(),
            }],
        };

        let json = serde_json::to_string(&wf).unwrap();
        let back: WorkflowDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "test");
        assert_eq!(back.stages.len(), 1);
        assert_eq!(back.stages[0].name, "stage1");
    }

    #[test]
    fn test_workflow_definition_no_stages() {
        let wf = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "empty".into(),
            description: "".into(),
            stages: vec![],
        };
        assert!(wf.stages.is_empty());
    }

    #[test]
    fn test_stage_definition_parallel_flag() {
        let stage = StageDefinition {
            name: "parallel-stage".into(),
            agents: vec!["a".into(), "b".into()],
            parallel: true,
            continue_on_error: false,
            prompt_template: "".into(),
        };
        assert!(stage.parallel);
        assert!(!stage.continue_on_error);
    }

    #[test]
    fn test_stage_definition_multiple_agents() {
        let stage = StageDefinition {
            name: "multi-agent".into(),
            agents: vec!["architect".into(), "developer".into(), "tester".into()],
            parallel: true,
            continue_on_error: false,
            prompt_template: "collaborate".into(),
        };
        assert_eq!(stage.agents.len(), 3);
    }

    // ── WorkflowExecutor (type-only tests) ──────────────────────────

    #[test]
    fn test_cancel_execution_nonexistent() {
        let bus = Arc::new(MessageBus::new());
        let executor = WorkflowExecutor::new(
            Arc::new(crate::cli::CliManager::new()),
            Arc::new(crate::team::TeamManager::new(bus.clone())),
            bus,
        );
        assert!(!executor.cancel_execution(Uuid::new_v4()));
    }

    #[test]
    fn test_get_execution_nonexistent() {
        let bus = Arc::new(MessageBus::new());
        let executor = WorkflowExecutor::new(
            Arc::new(crate::cli::CliManager::new()),
            Arc::new(crate::team::TeamManager::new(bus.clone())),
            bus,
        );
        assert!(executor.get_execution(Uuid::new_v4()).is_none());
    }

    // ── ExecutionResult serialization with all statuses ─────────────

    #[test]
    fn test_execution_result_all_statuses_serde() {
        for status in &[
            ExecutionStatus::Pending,
            ExecutionStatus::Running,
            ExecutionStatus::Completed,
            ExecutionStatus::Failed,
            ExecutionStatus::Cancelled,
        ] {
            let result = ExecutionResult {
                execution_id: Uuid::new_v4(),
                workflow_name: "test".into(),
                status: *status,
                stage_results: vec![],
                started_at: Utc::now(),
                finished_at: None,
                duration_ms: None,
            };
            let json = serde_json::to_string(&result).unwrap();
            let back: ExecutionResult = serde_json::from_str(&json).unwrap();
            assert_eq!(back.status, *status);
        }
    }
}
