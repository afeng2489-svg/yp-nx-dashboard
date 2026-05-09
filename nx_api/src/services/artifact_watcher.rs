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
    workspace_path: Arc<parking_lot::RwLock<Option<String>>>,
    pending: Mutex<HashMap<(String, String), WorkdirSnapshot>>,
    opts: SnapshotOptions,
    /// 引擎内部 UUID → API exec_id 映射（每次执行注册一条）
    id_map: Mutex<HashMap<String, String>>,
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
            id_map: Mutex::new(HashMap::new()),
        }
    }

    /// 注册引擎内部 UUID → API exec_id 映射，并迁移已存在的 pending 条目
    pub fn register_id(&self, engine_id: &str, api_id: &str) {
        self.id_map
            .lock()
            .insert(engine_id.to_string(), api_id.to_string());
        // 若 before_stage 在映射注册前已触发，pending key 用的是 engine_id，需迁移
        let mut pending = self.pending.lock();
        let stale_keys: Vec<String> = pending
            .keys()
            .filter(|(id, _)| id == engine_id)
            .map(|(_, stage)| stage.clone())
            .collect();
        for stage in stale_keys {
            if let Some(snap) = pending.remove(&(engine_id.to_string(), stage.clone())) {
                pending.insert((api_id.to_string(), stage), snap);
            }
        }
    }

    fn resolve_id(&self, execution_id: &str) -> String {
        self.id_map
            .lock()
            .get(execution_id)
            .cloned()
            .unwrap_or_else(|| execution_id.to_string())
    }

    fn current_workdir(&self) -> Option<PathBuf> {
        self.workspace_path.read().clone().map(PathBuf::from)
    }
}

impl nexus_workflow::watcher::StageWatcher for ArtifactStageWatcher {
    fn before_stage(&self, execution_id: &str, stage_name: &str) {
        let Some(workdir) = self.current_workdir() else {
            tracing::warn!(
                "[ArtifactWatcher] before_stage: 无 workspace, 跳过 {}/{}",
                execution_id,
                stage_name
            );
            return;
        };
        let snap = snapshot_with_options(&workdir, &self.opts);
        let api_id = self.resolve_id(execution_id);
        tracing::info!(
            "[ArtifactWatcher] before_stage: engine_id={} -> api_id={} stage={} workdir={:?}",
            execution_id,
            api_id,
            stage_name,
            workdir
        );
        let key = (api_id, stage_name.to_string());
        self.pending.lock().insert(key, snap);
    }

    fn after_stage(&self, execution_id: &str, stage_name: &str) {
        let Some(workdir) = self.current_workdir() else {
            return;
        };
        let api_id = self.resolve_id(execution_id);
        let key = (api_id.clone(), stage_name.to_string());
        let before = self.pending.lock().remove(&key);
        let Some(before) = before else {
            tracing::debug!(
                "[ArtifactWatcher] 没有 stage 开始前的 snapshot，跳过: {} / {}",
                api_id,
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

        match self.repo.record_diff(&api_id, Some(stage_name), &diff) {
            Ok(n) => tracing::info!(
                "[ArtifactWatcher] {} / {} 写入 {} 条产物记录",
                api_id,
                stage_name,
                n
            ),
            Err(e) => tracing::warn!(
                "[ArtifactWatcher] {} / {} 写入失败: {}",
                api_id,
                stage_name,
                e
            ),
        }
    }
}
