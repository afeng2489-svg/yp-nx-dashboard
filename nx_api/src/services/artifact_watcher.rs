//! 产物追踪的 StageWatcher 实现
//!
//! 把 core/workflow 的 StageWatcher trait 接入到我们的 ArtifactTracker + Repository。
//!
//! 工作方式：
//! - stage 开始前拍 working_dir 快照，存到内存 map
//! - stage 完成后再拍一次，diff 后写入 SQLite
//! - 内存 map 按 (execution_id, stage_name) 索引，用 parking_lot::Mutex 保护

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::services::artifact_repository::SqliteArtifactRepository;
use crate::services::artifact_tracker::{
    diff_snapshots, snapshot_with_options, SnapshotOptions, WorkdirSnapshot,
};

/// 产物追踪 watcher
pub struct ArtifactStageWatcher {
    repo: Arc<SqliteArtifactRepository>,
    /// 当前 workspace path（动态共享）
    workspace_path: Arc<parking_lot::RwLock<Option<String>>>,
    /// (execution_id, stage_name) → 该 stage 开始前的 snapshot
    pending: Mutex<HashMap<(String, String), WorkdirSnapshot>>,
    /// snapshot 配置
    opts: SnapshotOptions,
}

impl ArtifactStageWatcher {
    pub fn new(
        repo: Arc<SqliteArtifactRepository>,
        workspace_path: Arc<parking_lot::RwLock<Option<String>>>,
    ) -> Self {
        Self {
            repo,
            workspace_path,
            pending: Mutex::new(HashMap::new()),
            opts: SnapshotOptions::default(),
        }
    }

    fn current_workdir(&self) -> Option<PathBuf> {
        self.workspace_path.read().clone().map(PathBuf::from)
    }
}

impl nexus_workflow::watcher::StageWatcher for ArtifactStageWatcher {
    fn before_stage(&self, execution_id: &str, stage_name: &str) {
        let Some(workdir) = self.current_workdir() else {
            return; // 没有 workspace，跳过
        };

        let snap = snapshot_with_options(&workdir, &self.opts);
        let key = (execution_id.to_string(), stage_name.to_string());
        self.pending.lock().insert(key, snap);
    }

    fn after_stage(&self, execution_id: &str, stage_name: &str) {
        let Some(workdir) = self.current_workdir() else {
            return;
        };

        let key = (execution_id.to_string(), stage_name.to_string());
        let before = self.pending.lock().remove(&key);
        let Some(before) = before else {
            tracing::debug!(
                "[ArtifactWatcher] 没有 stage 开始前的 snapshot，跳过: {} / {}",
                execution_id,
                stage_name
            );
            return;
        };

        let after = snapshot_with_options(&workdir, &self.opts);
        let diff = diff_snapshots(&before, &after);

        let n_changes = diff.added.len() + diff.modified.len() + diff.deleted.len();
        if n_changes == 0 {
            return;
        }

        match self.repo.record_diff(execution_id, Some(stage_name), &diff) {
            Ok(n) => tracing::info!(
                "[ArtifactWatcher] {} / {} 写入 {} 条产物记录",
                execution_id,
                stage_name,
                n
            ),
            Err(e) => tracing::warn!(
                "[ArtifactWatcher] {} / {} 写入失败: {}",
                execution_id,
                stage_name,
                e
            ),
        }
    }
}
