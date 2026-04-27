//! 快照服务 — 事件驱动的角色快照保存 + 项目进度聚合
//!
//! 订阅 AgentExecutionEvent（旁路监听），自动更新角色快照和项目总进度。

use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use super::error::TeamEvolutionError;
use super::feature_flag_service::FeatureFlagService;
use super::snapshot_repository::{
    ProjectProgress, RoleSnapshot, RoleSnapshotHistory, SqliteSnapshotRepository,
};
use crate::models::feature_flag::keys;

pub struct SnapshotService {
    repo: Arc<SqliteSnapshotRepository>,
    feature_flags: Arc<FeatureFlagService>,
}

impl SnapshotService {
    pub fn new(
        repo: Arc<SqliteSnapshotRepository>,
        feature_flags: Arc<FeatureFlagService>,
    ) -> Self {
        Self {
            repo,
            feature_flags,
        }
    }

    // ── 角色快照操作 ──────────────────────────────────────────

    /// 更新或创建角色快照（Pipeline step 开始/完成时调用）
    pub fn update_role_snapshot(
        &self,
        project_id: &str,
        team_id: &str,
        role_id: &str,
        role_name: &str,
        phase: &str,
        progress_pct: u32,
        current_task: &str,
        summary: &str,
        cli_output: &str,
        files: &[String],
    ) -> Result<RoleSnapshot, TeamEvolutionError> {
        if !self
            .feature_flags
            .is_enabled(keys::SNAPSHOT)
            .unwrap_or(false)
        {
            return Err(TeamEvolutionError::FeatureDisabled(
                keys::SNAPSHOT.to_string(),
            ));
        }

        let now = Utc::now().to_rfc3339();

        // 查找已有快照或创建新的
        let existing = self.repo.find_snapshot(project_id, role_id)?;
        let snap = match existing {
            Some(mut s) => {
                s.phase = phase.to_string();
                s.progress_pct = progress_pct;
                s.current_task = current_task.to_string();
                s.summary = summary.to_string();
                s.last_cli_output = cli_output.to_string();
                s.files_touched = files.to_vec();
                s.execution_count += 1;
                s.updated_at = now;
                s
            }
            None => RoleSnapshot {
                id: Uuid::new_v4().to_string(),
                project_id: project_id.to_string(),
                team_id: team_id.to_string(),
                role_id: role_id.to_string(),
                role_name: role_name.to_string(),
                phase: phase.to_string(),
                progress_pct,
                current_task: current_task.to_string(),
                summary: summary.to_string(),
                last_cli_output: cli_output.to_string(),
                files_touched: files.to_vec(),
                execution_count: 1,
                checksum: String::new(),
                created_at: now.clone(),
                updated_at: now,
            },
        };

        self.repo.upsert_snapshot(&snap)?;

        // 写入历史
        self.repo.push_history(&snap)?;

        // 重算项目总进度
        self.recalculate_project_progress(project_id, team_id)?;

        Ok(snap)
    }

    /// 标记角色为特定状态（休眠、失败等）
    pub fn set_role_phase(
        &self,
        project_id: &str,
        role_id: &str,
        phase: &str,
    ) -> Result<(), TeamEvolutionError> {
        if let Some(mut snap) = self.repo.find_snapshot(project_id, role_id)? {
            snap.phase = phase.to_string();
            snap.updated_at = Utc::now().to_rfc3339();
            self.repo.upsert_snapshot(&snap)?;

            // 重算进度
            if let Ok(snaps) = self.repo.find_snapshots_by_project(project_id) {
                if let Some(first) = snaps.first() {
                    let _ = self.recalculate_project_progress(project_id, &first.team_id);
                }
            }
        }
        Ok(())
    }

    /// 全量保存：所有活跃角色当前状态落盘（项目关闭时调用）
    pub fn snapshot_all_active(&self, project_id: &str) -> Result<u32, TeamEvolutionError> {
        let snapshots = self.repo.find_snapshots_by_project(project_id)?;
        let mut saved = 0u32;
        for snap in &snapshots {
            if snap.phase != "idle" && snap.phase != "done" {
                self.repo.push_history(snap)?;
                saved += 1;
            }
        }
        Ok(saved)
    }

    /// 构建恢复上下文（崩溃恢复时使用）
    pub fn build_resume_context(&self, project_id: &str) -> Result<String, TeamEvolutionError> {
        let snapshots = self.repo.find_snapshots_by_project(project_id)?;
        let progress = self.repo.find_progress(project_id)?;

        let mut ctx = String::new();
        ctx.push_str("## 项目恢复上下文\n\n");

        if let Some(p) = progress {
            ctx.push_str(&format!(
                "**总进度**: {}% | 阶段: {} | 活跃角色: {}/{}\n\n",
                p.overall_pct, p.overall_phase, p.active_roles, p.total_roles
            ));
        }

        for snap in &snapshots {
            ctx.push_str(&format!(
                "### {} ({})\n- 阶段: {} | 进度: {}%\n- 当前任务: {}\n- 摘要: {}\n- 修改文件: {:?}\n\n",
                snap.role_name, snap.role_id,
                snap.phase, snap.progress_pct,
                snap.current_task, snap.summary, snap.files_touched,
            ));
        }

        Ok(ctx)
    }

    // ── 查询 ──────────────────────────────────────────────────

    pub fn get_project_progress(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectProgress>, TeamEvolutionError> {
        self.feature_flags.require_readable(keys::SNAPSHOT)?;
        self.repo.find_progress(project_id)
    }

    pub fn get_role_snapshots(
        &self,
        project_id: &str,
    ) -> Result<Vec<RoleSnapshot>, TeamEvolutionError> {
        self.feature_flags.require_readable(keys::SNAPSHOT)?;
        self.repo.find_snapshots_by_project(project_id)
    }

    pub fn get_role_snapshot(
        &self,
        project_id: &str,
        role_id: &str,
    ) -> Result<Option<RoleSnapshot>, TeamEvolutionError> {
        self.feature_flags.require_readable(keys::SNAPSHOT)?;
        self.repo.find_snapshot(project_id, role_id)
    }

    pub fn get_role_history(
        &self,
        project_id: &str,
        role_id: &str,
    ) -> Result<Vec<RoleSnapshotHistory>, TeamEvolutionError> {
        self.feature_flags.require_readable(keys::SNAPSHOT)?;
        self.repo.find_history(project_id, role_id)
    }

    // ── 内部 ──────────────────────────────────────────────────

    /// 从角色快照聚合计算项目总进度
    fn recalculate_project_progress(
        &self,
        project_id: &str,
        team_id: &str,
    ) -> Result<(), TeamEvolutionError> {
        let snapshots = self.repo.find_snapshots_by_project(project_id)?;

        let total = snapshots.len() as u32;
        let active = snapshots
            .iter()
            .filter(|s| {
                s.phase != "idle"
                    && s.phase != "done"
                    && s.phase != "failed"
                    && s.phase != "hibernated"
            })
            .count() as u32;
        let completed = snapshots.iter().filter(|s| s.phase == "done").count() as u32;
        let failed = snapshots.iter().filter(|s| s.phase == "failed").count() as u32;

        // overall_pct = average of all role progress_pct
        let overall_pct = snapshots
            .iter()
            .map(|s| s.progress_pct)
            .sum::<u32>()
            .checked_div(total)
            .unwrap_or(0);

        // Find the most active phase
        let overall_phase = if completed == total && total > 0 {
            "completed".to_string()
        } else if failed > 0 && active == 0 {
            "failed".to_string()
        } else {
            snapshots
                .iter()
                .filter(|s| s.phase != "idle" && s.phase != "done")
                .map(|s| s.phase.as_str())
                .next()
                .unwrap_or("idle")
                .to_string()
        };

        // Last activity
        let last = snapshots.iter().max_by_key(|s| &s.updated_at);
        let (last_activity, last_activity_at) = match last {
            Some(s) => (
                format!("{}: {}", s.role_name, s.current_task),
                Some(s.updated_at.clone()),
            ),
            None => (String::new(), None),
        };

        let now = Utc::now().to_rfc3339();
        let progress = ProjectProgress {
            project_id: project_id.to_string(),
            team_id: team_id.to_string(),
            pipeline_id: None,
            overall_phase,
            overall_pct,
            total_roles: total,
            active_roles: active,
            completed_roles: completed,
            failed_roles: failed,
            last_activity,
            last_activity_at,
            updated_at: now,
        };

        self.repo.upsert_progress(&progress)?;
        Ok(())
    }
}
