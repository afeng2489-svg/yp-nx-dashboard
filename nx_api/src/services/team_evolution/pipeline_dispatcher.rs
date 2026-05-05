//! Pipeline Dispatcher — 调度执行层

use std::sync::Arc;
use tokio::time::{interval, Duration};

use super::pipeline_service::PipelineService;
use super::quality_gate::run_quality_gate;
use crate::models::pipeline::PipelinePhase;
use crate::services::claude_cli::call_claude_cli_with_timeout;
use crate::services::session_service::SessionService;
use crate::services::team_service::TeamService;

pub struct PipelineDispatcher {
    pipeline_service: Arc<PipelineService>,
    session_service: Arc<SessionService>,
    team_service: Arc<TeamService>,
}

impl PipelineDispatcher {
    pub fn new(
        pipeline_service: Arc<PipelineService>,
        session_service: Arc<SessionService>,
        team_service: Arc<TeamService>,
    ) -> Self {
        Self { pipeline_service, session_service, team_service }
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
        let pipelines = self.pipeline_service
            .find_running_pipelines()
            .map_err(|e| e.to_string())?;

        for pipeline in pipelines {
            let steps = self.pipeline_service
                .get_dispatchable_steps(&pipeline.id)
                .map_err(|e| e.to_string())?;

            for step in steps {
                let ps = Arc::clone(&self.pipeline_service);
                let ss = Arc::clone(&self.session_service);
                let pipeline_id = pipeline.id.clone();
                let step_id = step.id.clone();
                let phase = format!("{:?}", step.phase);
                let step_phase = step.phase.clone();

                // 查找角色 system_prompt，拼入 instruction 前
                let prompt = self.build_prompt(&step.role_id, &step.instruction);

                let _ = ps.mark_step_running(&step_id);

                tokio::spawn(async move {
                    let session = ss.create_session(format!("pipeline-step:{}", step_id)).await;

                    tracing::info!("[Dispatcher] 执行 step {} ({})", step_id, phase);

                    let result = call_claude_cli_with_timeout(&prompt, 300, None).await;

                    match result {
                        Ok(output) => {
                            if let Ok(session) = &session {
                                let _ = ss.delete_session(&session.id).await;
                            }
                            // 质量门：step 完成后检测项目测试
                            let working_dir = session.as_ref().ok()
                                .and_then(|_| None::<String>); // dispatcher 暂无 working_dir，预留接口
                            let final_output = match run_quality_gate(working_dir.as_deref()) {
                                Some(gate) if !gate.passed => {
                                    let failures: Vec<String> = gate.checks.iter()
                                        .filter(|c| !c.passed)
                                        .map(|c| format!("{}: {}", c.cmd, c.stderr.chars().take(200).collect::<String>()))
                                        .collect();
                                    format!("{}\n\n--- 质量门失败 ---\n{}", output, failures.join("\n"))
                                }
                                Some(gate) => format!("{}\n\n--- {} ---", output, gate),
                                None => output,
                            };
                            let _ = ps.on_step_completed(&pipeline_id, &step_id, &final_output);
                            tracing::info!("[Dispatcher] step {} 完成", step_id);
                            // 架构设计完成后请求人工审批
                            if step_phase == PipelinePhase::ArchitectureDesign {
                                let _ = ps.request_approval(&pipeline_id);
                                tracing::info!("[Dispatcher] pipeline {} 等待人工审批", pipeline_id);
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

    /// 用 role_id 查 system_prompt，拼到 instruction 前
    fn build_prompt(&self, role_id: &str, instruction: &str) -> String {
        // 先尝试按 role_id 直接查
        if let Ok(role) = self.team_service.get_role(role_id) {
            if !role.system_prompt.is_empty() {
                return format!("{}\n\n---\n\n{}", role.system_prompt, instruction);
            }
        }
        // 找不到角色时直接用 instruction
        instruction.to_string()
    }
}
