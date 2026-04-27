//! Team Evolution 集成层 — 将 Pipeline/Snapshot/Resume/Lifecycle 连接到实际执行
//!
//! 核心桥接逻辑：
//! 1. Pipeline dispatch → 调用已有 try_pty_dispatch 或 execute_team_task
//! 2. AgentExecutionEvent 旁路订阅 → 自动更新快照 + checkpoint
//! 3. 定时任务 → lifecycle scan / temp clean / dispatch loop

use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

use crate::services::team_evolution::{
    error::TeamEvolutionError, pipeline_service::PipelineService,
    process_lifecycle::ProcessLifecycleManager, resume_service::ResumeService,
    snapshot_service::SnapshotService, temp_cleaner::TempCleaner,
};
use crate::ws::agent_execution::AgentExecutionEvent;

/// Pipeline 步骤 dispatch 请求 — 由 route 层消费后调用实际 CLI
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StepDispatchRequest {
    pub pipeline_id: String,
    pub step_id: String,
    pub role_id: String,
    pub instruction: String,
    pub team_id: String,
    pub project_id: String,
}

/// Team Evolution 事件处理器
/// 订阅 AgentExecutionEvent broadcast，自动触发快照更新、checkpoint 保存等
pub struct TeamEvolutionEventHandler {
    pipeline_service: Arc<PipelineService>,
    snapshot_service: Arc<SnapshotService>,
    resume_service: Arc<ResumeService>,
    process_lifecycle: Arc<ProcessLifecycleManager>,
    event_tx: broadcast::Sender<AgentExecutionEvent>,
    /// 当前 execution_id → (pipeline_id, step_id, project_id, team_id, role_id) 映射
    step_mapping: Arc<RwLock<std::collections::HashMap<String, StepMapping>>>,
}

#[derive(Debug, Clone)]
struct StepMapping {
    pipeline_id: String,
    step_id: String,
    project_id: String,
    team_id: String,
    role_id: String,
    instruction: String,
}

impl TeamEvolutionEventHandler {
    pub fn new(
        pipeline_service: Arc<PipelineService>,
        snapshot_service: Arc<SnapshotService>,
        resume_service: Arc<ResumeService>,
        process_lifecycle: Arc<ProcessLifecycleManager>,
        event_tx: broadcast::Sender<AgentExecutionEvent>,
    ) -> Self {
        Self {
            pipeline_service,
            snapshot_service,
            resume_service,
            process_lifecycle,
            event_tx,
            step_mapping: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// 注册 execution_id 到 pipeline step 的映射
    pub fn register_step_execution(
        &self,
        execution_id: &str,
        pipeline_id: &str,
        step_id: &str,
        project_id: &str,
        team_id: &str,
        role_id: &str,
    ) {
        self.step_mapping.write().insert(
            execution_id.to_string(),
            StepMapping {
                pipeline_id: pipeline_id.to_string(),
                step_id: step_id.to_string(),
                project_id: project_id.to_string(),
                team_id: team_id.to_string(),
                role_id: role_id.to_string(),
                instruction: String::new(), // filled later when dispatching
            },
        );
    }

    /// 启动事件监听循环
    pub fn spawn_event_listener(&self) {
        let mut rx = self.event_tx.subscribe();
        let pipeline = self.pipeline_service.clone();
        let snapshot = self.snapshot_service.clone();
        let resume = self.resume_service.clone();
        let lifecycle = self.process_lifecycle.clone();
        let mapping = self.step_mapping.clone();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        Self::handle_event(
                            &event, &pipeline, &snapshot, &resume, &lifecycle, &mapping,
                        );
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("[TeamEvolution] Event listener lagged {n} frames");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("[TeamEvolution] Event channel closed, stopping listener");
                        return;
                    }
                }
            }
        });
    }

    fn handle_event(
        event: &AgentExecutionEvent,
        pipeline: &Arc<PipelineService>,
        snapshot: &Arc<SnapshotService>,
        resume: &Arc<ResumeService>,
        lifecycle: &Arc<ProcessLifecycleManager>,
        mapping: &Arc<RwLock<std::collections::HashMap<String, StepMapping>>>,
    ) {
        let exec_id = event.execution_id().to_string();
        if exec_id.is_empty() {
            return;
        }

        // Touch process lifecycle
        lifecycle.touch(&exec_id);

        // Update checkpoint on output events
        match event {
            AgentExecutionEvent::Output { partial_output, .. } => {
                let _ = resume.update_checkpoint(&exec_id, partial_output);
            }
            AgentExecutionEvent::Progress { action, detail, .. } => {
                let _ = resume.update_checkpoint(&exec_id, &format!("{action}: {detail}"));

                // Update snapshot progress if we have a step mapping
                let mappings = mapping.read();
                if let Some(sm) = mappings.get(&exec_id) {
                    // Calculate progress from pipeline steps
                    let pct = pipeline
                        .get_status(&sm.pipeline_id)
                        .map(|(_, steps)| {
                            let total = steps.len().max(1);
                            let completed = steps
                                .iter()
                                .filter(|s| {
                                    s.status == crate::models::pipeline::StepStatus::Completed
                                })
                                .count();
                            (completed * 100 / total) as u32
                        })
                        .unwrap_or(0);

                    let _ = snapshot.update_role_snapshot(
                        &sm.project_id,
                        &sm.team_id,
                        &sm.role_id,
                        &sm.role_id, // role_name fallback
                        action,
                        pct,
                        &sm.step_id,
                        &format!("{action}: {detail}"),
                        "",
                        &[],
                    );
                }
            }
            AgentExecutionEvent::Completed { result, .. } => {
                // Mark checkpoint as completed
                let _ = resume.mark_completed(&exec_id);

                // Unregister from process lifecycle
                lifecycle.unregister_process(&exec_id);

                // Notify pipeline step completed
                let mappings = mapping.read();
                if let Some(sm) = mappings.get(&exec_id) {
                    let _ = pipeline.on_step_completed(&sm.pipeline_id, &sm.step_id, result);

                    // Update snapshot to done
                    let _ = snapshot.update_role_snapshot(
                        &sm.project_id,
                        &sm.team_id,
                        &sm.role_id,
                        &sm.role_id,
                        "done",
                        100,
                        &sm.step_id,
                        result,
                        "",
                        &[],
                    );
                }
                drop(mappings);

                // Clean up mapping
                mapping.write().remove(&exec_id);
            }
            AgentExecutionEvent::Failed { error, .. } => {
                lifecycle.unregister_process(&exec_id);

                let mappings = mapping.read();
                if let Some(sm) = mappings.get(&exec_id) {
                    let _ = pipeline.on_step_failed(&sm.pipeline_id, &sm.step_id, error);

                    let _ = snapshot.update_role_snapshot(
                        &sm.project_id,
                        &sm.team_id,
                        &sm.role_id,
                        &sm.role_id,
                        "failed",
                        0,
                        &sm.step_id,
                        error,
                        "",
                        &[],
                    );
                }
                drop(mappings);

                mapping.write().remove(&exec_id);
            }
            AgentExecutionEvent::Started { role_id, .. } => {
                // Create checkpoint for this execution
                let mappings = mapping.read();
                if let Some(sm) = mappings.get(&exec_id) {
                    let _ = resume.create_checkpoint(
                        &exec_id,
                        &sm.project_id,
                        Some(&sm.step_id),
                        role_id.as_deref().unwrap_or(&sm.role_id),
                        &sm.instruction,
                    );
                }
            }
            _ => {}
        }
    }

    /// 启动定时任务
    pub fn spawn_periodic_tasks(
        pipeline: Arc<PipelineService>,
        lifecycle: Arc<ProcessLifecycleManager>,
        temp_cleaner: Arc<TempCleaner>,
        event_tx: broadcast::Sender<AgentExecutionEvent>,
    ) {
        // Process lifecycle scan — every 30s
        let lc = lifecycle.clone();
        let tx = event_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                let events = lc.scan_lifecycle_events();
                for event in events {
                    match &event {
                        crate::services::team_evolution::process_lifecycle::ProcessLifecycleEvent::Hibernated {
                            execution_id, idle_secs, ..
                        } => {
                            let _ = tx.send(AgentExecutionEvent::Hibernated {
                                execution_id: execution_id.clone(),
                                idle_secs: *idle_secs,
                            });
                        }
                        crate::services::team_evolution::process_lifecycle::ProcessLifecycleEvent::ResourceLimitReached {
                            current_processes, max_processes, current_memory_mb, max_memory_mb, ..
                        } => {
                            let _ = tx.send(AgentExecutionEvent::ResourceLimitReached {
                                current_processes: *current_processes,
                                max_processes: *max_processes,
                                current_memory_mb: *current_memory_mb as u64,
                                max_memory_mb: *max_memory_mb as u64,
                            });
                        }
                        _ => {}
                    }
                }
            }
        });

        // Temp cleanup — every hour
        let cleaner = temp_cleaner.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600));
            loop {
                interval.tick().await;
                if let Err(e) = cleaner.run_all() {
                    tracing::warn!("[TeamEvolution] Temp cleanup failed: {e}");
                }
            }
        });

        // Pipeline auto-dispatch loop — every 5s, check for ready steps
        let ps = pipeline.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;

                // Scan all running pipelines for ready steps
                match ps.find_running_pipelines() {
                    Ok(pipelines) => {
                        for pipeline in &pipelines {
                            // Check for steps that need phase advance
                            if let Err(e) = ps.try_advance_phase_pub(&pipeline.id) {
                                tracing::debug!(
                                    "[TeamEvolution] Phase advance check failed for {}: {e}",
                                    pipeline.id
                                );
                            }

                            // Check for dispatchable steps and mark them ready
                            match ps.get_dispatchable_steps(&pipeline.id) {
                                Ok(steps) if !steps.is_empty() => {
                                    tracing::debug!(
                                        "[TeamEvolution] Pipeline {} has {} dispatchable steps",
                                        pipeline.id,
                                        steps.len()
                                    );
                                    // Steps are marked Running by the dispatch API endpoint.
                                    // The auto-dispatch loop identifies ready steps for monitoring.
                                }
                                Ok(_) => {}
                                Err(e) => {
                                    tracing::debug!(
                                        "[TeamEvolution] Dispatch check failed for {}: {e}",
                                        pipeline.id
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("[TeamEvolution] Failed to scan running pipelines: {e}");
                    }
                }
            }
        });
    }
}
