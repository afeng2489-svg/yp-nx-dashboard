//! Pipeline Dispatcher — 调度执行层

use std::sync::Arc;
use tokio::time::{interval, Duration};

use super::pipeline_service::PipelineService;
use super::quality_gate::run_quality_gate;
use crate::models::pipeline::PipelinePhase;
use crate::services::claude_cli::call_claude_cli_with_timeout;
use crate::services::session_service::SessionService;
use crate::services::team_service::TeamService;
use crate::services::workspace_service::WorkspaceService;

pub struct PipelineDispatcher {
    pipeline_service: Arc<PipelineService>,
    session_service: Arc<SessionService>,
    team_service: Arc<TeamService>,
    workspace_service: Arc<WorkspaceService>,
}

impl PipelineDispatcher {
    pub fn new(
        pipeline_service: Arc<PipelineService>,
        session_service: Arc<SessionService>,
        team_service: Arc<TeamService>,
        workspace_service: Arc<WorkspaceService>,
    ) -> Self {
        Self {
            pipeline_service,
            session_service,
            team_service,
            workspace_service,
        }
    }

    fn resolve_working_dir(&self, project_id: &str) -> Option<String> {
        if let Ok(Some(ws)) = self.workspace_service.get_workspace(project_id) {
            return ws.root_path.filter(|p| !p.is_empty());
        }
        if let Ok(projects) = self.workspace_service.list_workspaces() {
            for ws in projects {
                if ws.id == project_id {
                    return ws.root_path.filter(|p| !p.is_empty());
                }
            }
        }
        None
    }

    pub fn start(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(10));
            loop {
                ticker.tick().await;
                if let Err(e) = self.tick().await {
                    tracing::warn!("[Dispatcher] tick error: {}", e);
                }
            }
        });
    }

    async fn tick(&self) -> Result<(), String> {
        let pipelines = self
            .pipeline_service
            .find_running_pipelines()
            .map_err(|e| e.to_string())?;

        for pipeline in pipelines {
            let working_dir = self.resolve_working_dir(&pipeline.project_id);
            let steps = self
                .pipeline_service
                .get_dispatchable_steps(&pipeline.id)
                .map_err(|e| e.to_string())?;

            for step in steps {
                let ps = Arc::clone(&self.pipeline_service);
                let ss = Arc::clone(&self.session_service);
                let pipeline_id = pipeline.id.clone();
                let step_id = step.id.clone();
                let phase = format!("{:?}", step.phase);
                let step_phase = step.phase.clone();
                let wd = working_dir.clone();

                let prompt = self.build_prompt(&step.role_id, &step.instruction);

                let _ = ps.mark_step_running(&step_id);

                tokio::spawn(async move {
                    let session = ss
                        .create_session(format!("pipeline-step:{}", step_id))
                        .await;

                    tracing::info!(
                        "[Dispatcher] 执行 step {} ({}) cwd={:?}",
                        step_id,
                        phase,
                        wd
                    );

                    let result = call_claude_cli_with_timeout(&prompt, 300, wd.as_deref()).await;

                    match result {
                        Ok(output) => {
                            if let Ok(session) = &session {
                                let _ = ss.delete_session(&session.id).await;
                            }
                            let final_output = match run_quality_gate(wd.as_deref()) {
                                Some(gate) if !gate.passed => {
                                    let failures: Vec<String> = gate
                                        .checks
                                        .iter()
                                        .filter(|c| !c.passed)
                                        .map(|c| {
                                            format!(
                                                "{}: {}",
                                                c.cmd,
                                                c.stderr.chars().take(200).collect::<String>()
                                            )
                                        })
                                        .collect();
                                    format!(
                                        "{}\n\n--- 质量门失败 ---\n{}",
                                        output,
                                        failures.join("\n")
                                    )
                                }
                                Some(gate) => format!("{}\n\n--- {} ---", output, gate),
                                None => output,
                            };
                            let _ = ps.on_step_completed(&pipeline_id, &step_id, &final_output);
                            tracing::info!("[Dispatcher] step {} 完成", step_id);
                            if step_phase == PipelinePhase::ArchitectureDesign {
                                let _ = ps.request_approval(&pipeline_id);
                                tracing::info!(
                                    "[Dispatcher] pipeline {} 等待人工审批",
                                    pipeline_id
                                );
                            }
                        }
                        Err(e) => {
                            let _ = ps.on_step_failed(&pipeline_id, &step_id, &e);
                            tracing::warn!("[Dispatcher] step {} 失败: {}", step_id, e);
                        }
                    }
                });
            }
        }
        Ok(())
    }

    fn build_prompt(&self, role_id: &str, instruction: &str) -> String {
        if let Ok(role) = self.team_service.get_role(role_id) {
            if !role.system_prompt.is_empty() {
                return format!("{}\n\n---\n\n{}", role.system_prompt, instruction);
            }
        }
        instruction.to_string()
    }
}
