//! Pipeline Service — 调度核心
//!
//! 负责 Pipeline 生命周期管理和步骤调度。
//! 具体的 dispatch 逻辑（调用 CLI 执行）将在 P1.3 任务中完善。

use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use super::error::TeamEvolutionError;
use super::feature_flag_service::FeatureFlagService;
use super::pipeline_repository::SqlitePipelineRepository;
use crate::models::feature_flag::keys;
use crate::models::pipeline::{
    PhaseGatePolicy, Pipeline, PipelinePhase, PipelineStatus, PipelineStep, StepStatus,
};

pub struct PipelineService {
    repo: Arc<SqlitePipelineRepository>,
    feature_flags: Arc<FeatureFlagService>,
}

impl PipelineService {
    pub fn new(
        repo: Arc<SqlitePipelineRepository>,
        feature_flags: Arc<FeatureFlagService>,
    ) -> Self {
        Self {
            repo,
            feature_flags,
        }
    }

    /// Create a new pipeline for a project
    pub fn create_pipeline(
        &self,
        project_id: &str,
        team_id: &str,
    ) -> Result<Pipeline, TeamEvolutionError> {
        self.feature_flags.require_enabled(keys::PIPELINE)?;

        let now = Utc::now();
        let pipeline = Pipeline {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.to_string(),
            team_id: team_id.to_string(),
            current_phase: PipelinePhase::RequirementsAnalysis,
            status: PipelineStatus::Idle,
            phase_gate_policy: PhaseGatePolicy::default(),
            created_at: now,
            updated_at: now,
        };

        self.repo.create_pipeline(&pipeline)?;
        Ok(pipeline)
    }

    /// Add steps to a pipeline
    pub fn add_steps(&self, steps: &[PipelineStep]) -> Result<(), TeamEvolutionError> {
        self.repo.create_steps_batch(steps)
    }

    /// Start a pipeline
    pub fn start(&self, pipeline_id: &str) -> Result<Pipeline, TeamEvolutionError> {
        self.feature_flags.require_enabled(keys::PIPELINE)?;

        let pipeline = self
            .repo
            .find_pipeline_by_id(pipeline_id)?
            .ok_or_else(|| TeamEvolutionError::PipelineNotFound(pipeline_id.to_string()))?;

        if pipeline.status == PipelineStatus::Running {
            return Err(TeamEvolutionError::PipelineAlreadyRunning(
                pipeline_id.to_string(),
            ));
        }

        self.repo
            .update_pipeline_status(pipeline_id, &PipelineStatus::Running)?;

        self.repo
            .find_pipeline_by_id(pipeline_id)?
            .ok_or_else(|| TeamEvolutionError::PipelineNotFound(pipeline_id.to_string()))
    }

    /// Pause a running pipeline
    pub fn pause(&self, pipeline_id: &str) -> Result<Pipeline, TeamEvolutionError> {
        let pipeline = self
            .repo
            .find_pipeline_by_id(pipeline_id)?
            .ok_or_else(|| TeamEvolutionError::PipelineNotFound(pipeline_id.to_string()))?;

        if pipeline.status != PipelineStatus::Running {
            return Err(TeamEvolutionError::PipelinePaused(pipeline_id.to_string()));
        }

        self.repo
            .update_pipeline_status(pipeline_id, &PipelineStatus::Paused)?;

        self.repo
            .find_pipeline_by_id(pipeline_id)?
            .ok_or_else(|| TeamEvolutionError::PipelineNotFound(pipeline_id.to_string()))
    }

    /// Resume a paused pipeline
    pub fn resume(&self, pipeline_id: &str) -> Result<Pipeline, TeamEvolutionError> {
        let pipeline = self
            .repo
            .find_pipeline_by_id(pipeline_id)?
            .ok_or_else(|| TeamEvolutionError::PipelineNotFound(pipeline_id.to_string()))?;

        if pipeline.status != PipelineStatus::Paused {
            return Err(TeamEvolutionError::Internal(format!(
                "Pipeline {pipeline_id} is not paused, current status: {:?}",
                pipeline.status
            )));
        }

        self.repo
            .update_pipeline_status(pipeline_id, &PipelineStatus::Running)?;

        self.repo
            .find_pipeline_by_id(pipeline_id)?
            .ok_or_else(|| TeamEvolutionError::PipelineNotFound(pipeline_id.to_string()))
    }

    /// 请求人工审批（将 pipeline 置为 WaitingForApproval）
    pub fn request_approval(&self, pipeline_id: &str) -> Result<Pipeline, TeamEvolutionError> {
        self.repo
            .update_pipeline_status(pipeline_id, &PipelineStatus::WaitingForApproval)?;
        self.repo
            .find_pipeline_by_id(pipeline_id)?
            .ok_or_else(|| TeamEvolutionError::PipelineNotFound(pipeline_id.to_string()))
    }

    /// 审批通过 → 继续执行
    pub fn approve(&self, pipeline_id: &str) -> Result<Pipeline, TeamEvolutionError> {
        let pipeline = self
            .repo
            .find_pipeline_by_id(pipeline_id)?
            .ok_or_else(|| TeamEvolutionError::PipelineNotFound(pipeline_id.to_string()))?;
        if pipeline.status != PipelineStatus::WaitingForApproval {
            return Err(TeamEvolutionError::Internal(format!(
                "Pipeline {pipeline_id} is not waiting for approval"
            )));
        }
        self.repo
            .update_pipeline_status(pipeline_id, &PipelineStatus::Running)?;
        self.repo
            .find_pipeline_by_id(pipeline_id)?
            .ok_or_else(|| TeamEvolutionError::PipelineNotFound(pipeline_id.to_string()))
    }

    /// 审批拒绝 → 标记失败
    pub fn reject(&self, pipeline_id: &str, reason: &str) -> Result<Pipeline, TeamEvolutionError> {
        let pipeline = self
            .repo
            .find_pipeline_by_id(pipeline_id)?
            .ok_or_else(|| TeamEvolutionError::PipelineNotFound(pipeline_id.to_string()))?;
        if pipeline.status != PipelineStatus::WaitingForApproval {
            return Err(TeamEvolutionError::Internal(format!(
                "Pipeline {pipeline_id} is not waiting for approval"
            )));
        }
        tracing::info!("[Pipeline] {} rejected: {}", pipeline_id, reason);
        self.repo
            .update_pipeline_status(pipeline_id, &PipelineStatus::Failed)?;
        self.repo
            .find_pipeline_by_id(pipeline_id)?
            .ok_or_else(|| TeamEvolutionError::PipelineNotFound(pipeline_id.to_string()))
    }

    /// Get the next batch of steps ready to dispatch
    pub fn get_dispatchable_steps(
        &self,
        pipeline_id: &str,
    ) -> Result<Vec<PipelineStep>, TeamEvolutionError> {
        self.feature_flags.require_enabled(keys::PIPELINE)?;

        let pipeline = self
            .repo
            .find_pipeline_by_id(pipeline_id)?
            .ok_or_else(|| TeamEvolutionError::PipelineNotFound(pipeline_id.to_string()))?;

        if pipeline.status != PipelineStatus::Running {
            return Ok(vec![]);
        }

        let ready_steps = self.repo.get_ready_steps(pipeline_id)?;

        // Filter by current phase execution mode
        let current_group = pipeline.current_phase.phase_group();
        let phase_steps: Vec<PipelineStep> = ready_steps
            .into_iter()
            .filter(|s| s.phase.phase_group() == current_group)
            .collect();

        // Serial mode: only take the first ready step
        // Parallel mode: take all ready steps (grouped by role_id internally)
        if pipeline.current_phase.execution_mode() == crate::models::pipeline::ExecutionMode::Serial
        {
            Ok(phase_steps.into_iter().take(1).collect())
        } else {
            Ok(phase_steps)
        }
    }

    /// Called when a step completes successfully
    pub fn on_step_completed(
        &self,
        pipeline_id: &str,
        step_id: &str,
        output: &str,
    ) -> Result<(), TeamEvolutionError> {
        self.repo
            .update_step_status(step_id, &StepStatus::Completed, Some(output))?;
        self.try_advance_phase_pub(pipeline_id)?;
        Ok(())
    }

    /// Called when a step fails
    pub fn on_step_failed(
        &self,
        pipeline_id: &str,
        step_id: &str,
        error: &str,
    ) -> Result<bool, TeamEvolutionError> {
        let pipeline = self
            .repo
            .find_pipeline_by_id(pipeline_id)?
            .ok_or_else(|| TeamEvolutionError::PipelineNotFound(pipeline_id.to_string()))?;

        // Try auto-retry if policy allows
        if pipeline.phase_gate_policy.auto_retry {
            let retry_count = self.repo.increment_step_retry(step_id)?;
            if retry_count <= pipeline.phase_gate_policy.max_step_retries {
                return Ok(true); // Will be retried
            }
        }

        // Mark as failed
        self.repo
            .update_step_status(step_id, &StepStatus::Failed, Some(error))?;

        // Block dependent steps
        let all_steps = self.repo.find_steps_by_pipeline(pipeline_id)?;
        for step in &all_steps {
            if step.depends_on.contains(&step_id.to_string()) && step.status == StepStatus::Pending
            {
                self.repo
                    .update_step_status(&step.id, &StepStatus::Blocked, None)?;
            }
        }

        Ok(false) // Not retrying
    }

    /// Retry a failed step manually
    pub fn retry_step(
        &self,
        pipeline_id: &str,
        step_id: &str,
    ) -> Result<PipelineStep, TeamEvolutionError> {
        self.feature_flags.require_enabled(keys::PIPELINE)?;

        let steps = self.repo.find_steps_by_pipeline(pipeline_id)?;
        let step = steps.iter().find(|s| s.id == step_id).ok_or_else(|| {
            TeamEvolutionError::StepNotFound {
                pipeline_id: pipeline_id.to_string(),
                step_id: step_id.to_string(),
            }
        })?;

        if step.status != StepStatus::Failed && step.status != StepStatus::Blocked {
            return Err(TeamEvolutionError::StepNotRetriable {
                step_id: step_id.to_string(),
                status: format!("{:?}", step.status),
            });
        }

        self.repo
            .update_step_status(step_id, &StepStatus::Pending, None)?;

        // Unblock any steps that were blocked by this step
        for s in &steps {
            if s.status == StepStatus::Blocked && s.depends_on.contains(&step_id.to_string()) {
                self.repo
                    .update_step_status(&s.id, &StepStatus::Pending, None)?;
            }
        }

        // Return updated step
        let updated_steps = self.repo.find_steps_by_pipeline(pipeline_id)?;
        updated_steps
            .into_iter()
            .find(|s| s.id == step_id)
            .ok_or_else(|| TeamEvolutionError::StepNotFound {
                pipeline_id: pipeline_id.to_string(),
                step_id: step_id.to_string(),
            })
    }

    /// Get pipeline status with all steps
    pub fn get_status(
        &self,
        pipeline_id: &str,
    ) -> Result<(Pipeline, Vec<PipelineStep>), TeamEvolutionError> {
        self.feature_flags.require_readable(keys::PIPELINE)?;

        let pipeline = self
            .repo
            .find_pipeline_by_id(pipeline_id)?
            .ok_or_else(|| TeamEvolutionError::PipelineNotFound(pipeline_id.to_string()))?;
        let steps = self.repo.find_steps_by_pipeline(pipeline_id)?;
        Ok((pipeline, steps))
    }

    /// Get pipeline by project ID
    pub fn get_by_project(
        &self,
        project_id: &str,
    ) -> Result<Option<(Pipeline, Vec<PipelineStep>)>, TeamEvolutionError> {
        self.feature_flags.require_readable(keys::PIPELINE)?;

        match self.repo.find_pipeline_by_project(project_id)? {
            Some(pipeline) => {
                let steps = self.repo.find_steps_by_pipeline(&pipeline.id)?;
                Ok(Some((pipeline, steps)))
            }
            None => Ok(None),
        }
    }

    /// Find all running pipelines (for periodic auto-dispatch)
    pub fn find_running_pipelines(&self) -> Result<Vec<Pipeline>, TeamEvolutionError> {
        self.repo.find_running_pipelines()
    }

    /// Update step status (used by dispatch layer to mark steps as Running)
    pub fn update_step_status(
        &self,
        step_id: &str,
        status: &StepStatus,
        output: Option<&str>,
    ) -> Result<(), TeamEvolutionError> {
        self.repo.update_step_status(step_id, status, output)
    }

    /// Mark step as running (used by dispatcher)
    pub fn mark_step_running(&self, step_id: &str) -> Result<(), TeamEvolutionError> {
        self.repo
            .update_step_status(step_id, &StepStatus::Running, None)
    }

    /// Terminate session when step completes (used by dispatcher)
    pub async fn terminate_session(&self, session_id: &str) -> Result<(), TeamEvolutionError> {
        Ok(())
    }

    /// Find pipeline by ID (used by dispatch layer to get project_id/team_id)
    pub fn find_pipeline(&self, pipeline_id: &str) -> Result<Option<Pipeline>, TeamEvolutionError> {
        self.repo.find_pipeline_by_id(pipeline_id)
    }

    /// Check phase gate and advance to next phase if all steps in current phase are complete
    pub fn try_advance_phase_pub(&self, pipeline_id: &str) -> Result<(), TeamEvolutionError> {
        let pipeline = self
            .repo
            .find_pipeline_by_id(pipeline_id)?
            .ok_or_else(|| TeamEvolutionError::PipelineNotFound(pipeline_id.to_string()))?;

        let steps = self.repo.find_steps_by_pipeline(pipeline_id)?;
        let current_group = pipeline.current_phase.phase_group();

        // Check if all steps in current phase group are complete
        let phase_steps: Vec<&PipelineStep> = steps
            .iter()
            .filter(|s| s.phase.phase_group() == current_group)
            .collect();

        let all_complete = phase_steps
            .iter()
            .all(|s| s.status == StepStatus::Completed);
        let any_failed = phase_steps.iter().any(|s| s.status == StepStatus::Failed);

        if any_failed && !pipeline.phase_gate_policy.auto_retry {
            self.repo
                .update_pipeline_status(pipeline_id, &PipelineStatus::Failed)?;
            return Ok(());
        }

        if !all_complete {
            return Ok(());
        }

        // Advance to next phase group
        let next_phase = match current_group {
            1 => Some(PipelinePhase::BackendDev),     // Phase 1 → Phase 2
            2 => Some(PipelinePhase::ApiIntegration), // Phase 2 → Phase 3
            3 => {
                // All phases complete
                self.repo
                    .update_pipeline_status(pipeline_id, &PipelineStatus::Completed)?;
                None
            }
            _ => None,
        };

        if let Some(phase) = next_phase {
            self.repo.update_pipeline_phase(pipeline_id, &phase)?;
        }

        Ok(())
    }
}
