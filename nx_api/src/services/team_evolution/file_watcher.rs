//! 文件监控 — workspace_path 级别文件变更通知
//!
//! 使用 notify crate 监控工作区文件变更，黑白名单过滤，
//! debounce 后广播变更事件，快照服务订阅更新 files_touched。

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use super::error::TeamEvolutionError;
use super::feature_flag_service::FeatureFlagService;
use crate::models::feature_flag::keys;
use notify::Watcher;

/// 文件变更事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangeEvent {
    pub path: String,
    pub kind: FileChangeKind,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileChangeKind {
    Created,
    Modified,
    Deleted,
}

/// 文件监控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWatchConfig {
    /// 白名单 glob 模式
    pub whitelist: Vec<String>,
    /// 黑名单 glob 模式
    pub blacklist: Vec<String>,
    /// debounce 时间（毫秒）
    pub debounce_ms: u64,
}

impl Default for FileWatchConfig {
    fn default() -> Self {
        Self {
            whitelist: vec![
                "src/**/*.rs".to_string(),
                "src/**/*.ts".to_string(),
                "src/**/*.tsx".to_string(),
                "src/**/*.js".to_string(),
                "src/**/*.jsx".to_string(),
                "src/**/*.py".to_string(),
                "src/**/*.go".to_string(),
                "src/**/*.java".to_string(),
                "Cargo.toml".to_string(),
                "package.json".to_string(),
                "*.md".to_string(),
            ],
            blacklist: vec![
                "node_modules/**".to_string(),
                "target/**".to_string(),
                ".git/**".to_string(),
                "dist/**".to_string(),
                "build/**".to_string(),
                "__pycache__/**".to_string(),
                ".next/**".to_string(),
            ],
            debounce_ms: 300,
        }
    }
}

/// 文件监控管理器
pub struct FileWatcher {
    config: FileWatchConfig,
    feature_flags: Arc<FeatureFlagService>,
    /// 最近变更记录 (project_id -> changes)
    recent_changes: Arc<RwLock<std::collections::HashMap<String, Vec<FileChangeEvent>>>>,
    /// Active watchers: project_id -> watcher handle
    watchers: Arc<RwLock<std::collections::HashMap<String, notify::RecommendedWatcher>>>,
}

impl FileWatcher {
    pub fn new(config: FileWatchConfig, feature_flags: Arc<FeatureFlagService>) -> Self {
        Self {
            config,
            feature_flags,
            recent_changes: Arc::new(RwLock::new(std::collections::HashMap::new())),
            watchers: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.feature_flags
            .is_enabled(keys::FILE_WATCH)
            .unwrap_or(false)
    }

    /// 检查文件路径是否应该被监控（白名单+黑名单过滤）
    pub fn should_watch(&self, path: &str) -> bool {
        // Check blacklist first
        for pattern in &self.config.blacklist {
            if glob_match(pattern, path) {
                return false;
            }
        }

        // If whitelist is empty, watch everything not blacklisted
        if self.config.whitelist.is_empty() {
            return true;
        }

        // Check whitelist
        for pattern in &self.config.whitelist {
            if glob_match(pattern, path) {
                return true;
            }
        }

        false
    }

    /// Start watching a project workspace directory
    pub fn start_watching(
        &self,
        project_id: &str,
        workspace_path: &str,
        change_callback: Box<dyn Fn(FileChangeEvent) + Send + Sync>,
    ) -> Result<(), TeamEvolutionError> {
        if !self.is_enabled() {
            return Ok(());
        }

        let path = PathBuf::from(workspace_path);
        if !path.exists() {
            return Err(TeamEvolutionError::FileWatchError(format!(
                "Workspace path does not exist: {workspace_path}"
            )));
        }

        // Stop existing watcher for this project
        self.stop_watching(project_id);

        let config = self.config.clone();
        let project_id_owned = project_id.to_string();
        let recent = self.recent_changes.clone();

        let result = notify::RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        for path_buf in &event.paths {
                            let path_str = path_buf.to_string_lossy().to_string();

                            // Skip blacklisted paths
                            if glob_match_any(&config.blacklist, &path_str) {
                                continue;
                            }

                            // Skip non-whitelisted paths (if whitelist is non-empty)
                            if !config.whitelist.is_empty()
                                && !glob_match_any(&config.whitelist, &path_str)
                            {
                                continue;
                            }

                            let kind = match event.kind {
                                notify::EventKind::Create(_) => FileChangeKind::Created,
                                notify::EventKind::Modify(_) => FileChangeKind::Modified,
                                notify::EventKind::Remove(_) => FileChangeKind::Deleted,
                                _ => continue,
                            };

                            let change = FileChangeEvent {
                                path: path_str,
                                kind,
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            };

                            // Record change
                            {
                                let mut changes = recent.write();
                                let entry = changes.entry(project_id_owned.clone()).or_default();
                                entry.push(change.clone());
                                if entry.len() > 100 {
                                    let drain_count = entry.len() - 100;
                                    entry.drain(0..drain_count);
                                }
                            }

                            // Notify callback
                            change_callback(change);
                        }
                    }
                    Err(e) => {
                        tracing::debug!("[FileWatcher] Watch error: {e}");
                    }
                }
            },
            notify::Config::default().with_poll_interval(Duration::from_millis(config.debounce_ms)),
        );

        match result {
            Ok(mut watcher) => {
                if let Err(e) = watcher.watch(&path, notify::RecursiveMode::Recursive) {
                    tracing::warn!("[FileWatcher] Failed to start watching {workspace_path}: {e}");
                    return Err(TeamEvolutionError::FileWatchError(format!(
                        "Failed to watch: {e}"
                    )));
                }

                self.watchers
                    .write()
                    .insert(project_id.to_string(), watcher);
                tracing::info!(
                    "[FileWatcher] Started watching {workspace_path} for project {project_id}"
                );
                Ok(())
            }
            Err(e) => {
                tracing::warn!("[FileWatcher] Failed to create watcher: {e}");
                Err(TeamEvolutionError::FileWatchError(format!(
                    "Failed to create watcher: {e}"
                )))
            }
        }
    }

    /// Stop watching a project
    pub fn stop_watching(&self, project_id: &str) {
        if self.watchers.write().remove(project_id).is_some() {
            tracing::info!("[FileWatcher] Stopped watching for project {project_id}");
        }
    }

    /// 记录文件变更（由外部调用，如集成 notify crate 的回调）
    pub fn record_change(&self, project_id: &str, event: FileChangeEvent) {
        let mut changes = self.recent_changes.write();
        let entry = changes.entry(project_id.to_string()).or_default();
        entry.push(event);

        // Keep only last 100 changes per project
        if entry.len() > 100 {
            let drain_count = entry.len() - 100;
            entry.drain(0..drain_count);
        }
    }

    /// 获取项目的最近变更
    pub fn get_recent_changes(&self, project_id: &str) -> Vec<FileChangeEvent> {
        self.recent_changes
            .read()
            .get(project_id)
            .cloned()
            .unwrap_or_default()
    }

    /// 获取配置
    pub fn config(&self) -> &FileWatchConfig {
        &self.config
    }
}

/// Check if path matches any pattern in the list
fn glob_match_any(patterns: &[String], path: &str) -> bool {
    for pattern in patterns {
        if glob_match(pattern, path) {
            return true;
        }
    }
    false
}

/// Simple glob matching (supports * and ** patterns)
fn glob_match(pattern: &str, path: &str) -> bool {
    let pattern_lower = pattern.to_lowercase();
    let path_lower = path.to_lowercase();

    if pattern.contains("**") {
        // ** matches any number of directories
        let parts: Vec<&str> = pattern_lower.split("**").collect();
        if parts.is_empty() {
            return true;
        }
        let first = parts[0];
        if !first.is_empty() && !path_lower.starts_with(first) {
            return false;
        }
        // Check all parts appear in order
        let mut search_from = 0;
        for part in &parts {
            if part.is_empty() {
                continue;
            }
            if let Some(idx) = path_lower[search_from..].find(part) {
                search_from += idx + part.len();
            } else {
                return false;
            }
        }
        return true;
    }

    // Simple * wildcard — basic string matching without regex crate
    if pattern_lower.contains('*') {
        // Split on * and check all parts appear in order
        let parts: Vec<&str> = pattern_lower.split('*').filter(|p| !p.is_empty()).collect();
        if parts.is_empty() {
            return true;
        }
        let mut search_from = 0;
        for part in &parts {
            if let Some(idx) = path_lower[search_from..].find(part) {
                search_from += idx + part.len();
            } else {
                return false;
            }
        }
        return true;
    }

    path_lower.contains(&pattern_lower)
}
