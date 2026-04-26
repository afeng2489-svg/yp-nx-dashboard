//! 进程生命周期管理 — 闲置休眠 + 自动回收 + 资源限制 + 项目关闭清理

use std::sync::Arc;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::Instant;

use super::error::TeamEvolutionError;
use super::feature_flag_service::FeatureFlagService;
use crate::models::feature_flag::keys;

/// 进程活动记录
#[derive(Debug, Clone)]
struct ProcessActivity {
    last_active: Instant,
    pid: Option<u32>,
    project_id: String,
    role_id: String,
    memory_bytes: u64,
}

/// 进程生命周期配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleConfig {
    /// 闲置休眠超时（秒），默认 300 = 5 分钟
    pub hibernate_timeout_secs: u64,
    /// 闲置回收超时（秒），默认 900 = 15 分钟
    pub reclaim_timeout_secs: u64,
    /// 最大并发进程数，默认 4
    pub max_concurrent: usize,
    /// 最大总内存（字节），默认 2GB
    pub max_total_memory_bytes: u64,
    /// 检查间隔（秒），默认 30
    pub check_interval_secs: u64,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            hibernate_timeout_secs: 300,
            reclaim_timeout_secs: 900,
            max_concurrent: 4,
            max_total_memory_bytes: 2 * 1024 * 1024 * 1024, // 2GB
            check_interval_secs: 30,
        }
    }
}

/// 进程状态变更事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessLifecycleEvent {
    /// 进程因闲置进入休眠
    Hibernated {
        execution_id: String,
        role_id: String,
        idle_secs: u64,
    },
    /// 进程被回收
    Reclaimed {
        role_id: String,
        idle_secs: u64,
    },
    /// 资源限制达到
    ResourceLimitReached {
        current_processes: usize,
        max_processes: usize,
        current_memory_mb: u64,
        max_memory_mb: u64,
    },
    /// 项目进程已清理
    ProjectCleanedUp {
        project_id: String,
        terminated_count: usize,
    },
}

/// 进程生命周期管理器
pub struct ProcessLifecycleManager {
    activities: Arc<RwLock<HashMap<String, ProcessActivity>>>,
    config: LifecycleConfig,
    feature_flags: Arc<FeatureFlagService>,
}

impl ProcessLifecycleManager {
    pub fn new(config: LifecycleConfig, feature_flags: Arc<FeatureFlagService>) -> Self {
        Self {
            activities: Arc::new(RwLock::new(HashMap::new())),
            config,
            feature_flags,
        }
    }

    /// 注册一个活跃进程
    pub fn register_process(
        &self,
        execution_id: &str,
        project_id: &str,
        role_id: &str,
        pid: Option<u32>,
    ) -> Result<(), TeamEvolutionError> {
        if !self.feature_flags.is_enabled(keys::PROCESS_LIFECYCLE).unwrap_or(false) {
            // Feature disabled, still register but skip limit checks
            let mut activities = self.activities.write();
            activities.insert(execution_id.to_string(), ProcessActivity {
                last_active: Instant::now(),
                pid,
                project_id: project_id.to_string(),
                role_id: role_id.to_string(),
                memory_bytes: 0,
            });
            return Ok(());
        }

        // 检查并发限制
        let current_count = self.activities.read().len();
        if current_count >= self.config.max_concurrent {
            return Err(TeamEvolutionError::ResourceLimitReached {
                current: current_count,
                max: self.config.max_concurrent,
            });
        }

        let mut activities = self.activities.write();
        activities.insert(execution_id.to_string(), ProcessActivity {
            last_active: Instant::now(),
            pid,
            project_id: project_id.to_string(),
            role_id: role_id.to_string(),
            memory_bytes: 0,
        });

        Ok(())
    }

    /// 更新进程活跃时间（收到事件时调用）
    pub fn touch(&self, execution_id: &str) {
        if let Some(activity) = self.activities.write().get_mut(execution_id) {
            activity.last_active = Instant::now();
        }
    }

    /// 注销进程（正常完成时调用）
    pub fn unregister_process(&self, execution_id: &str) {
        self.activities.write().remove(execution_id);
    }

    /// 检查是否可以启动新进程
    pub fn can_start_process(&self) -> Result<bool, TeamEvolutionError> {
        if !self.feature_flags.is_enabled(keys::PROCESS_LIFECYCLE).unwrap_or(false) {
            return Ok(true);
        }

        let activities = self.activities.read();
        let current = activities.len();

        if current >= self.config.max_concurrent {
            return Ok(false);
        }

        // 检查总内存
        let total_memory: u64 = activities.values().map(|a| a.memory_bytes).sum();
        if total_memory >= self.config.max_total_memory_bytes {
            return Ok(false);
        }

        Ok(true)
    }

    /// 获取当前资源使用统计
    pub fn get_stats(&self) -> ProcessStats {
        let activities = self.activities.read();
        let total = activities.len();
        let total_memory: u64 = activities.values().map(|a| a.memory_bytes).sum();

        let mut idle_candidates = Vec::new();
        let mut hibernated_candidates = Vec::new();
        let now = Instant::now();

        for (exec_id, activity) in activities.iter() {
            let idle_secs = now.duration_since(activity.last_active).as_secs();
            if idle_secs > self.config.reclaim_timeout_secs {
                hibernated_candidates.push(exec_id.clone());
            } else if idle_secs > self.config.hibernate_timeout_secs {
                idle_candidates.push(exec_id.clone());
            }
        }

        ProcessStats {
            active_processes: total,
            max_processes: self.config.max_concurrent,
            total_memory_bytes: total_memory,
            max_memory_bytes: self.config.max_total_memory_bytes,
            idle_candidates,
            hibernated_candidates,
        }
    }

    /// 扫描并返回需要休眠/回收的进程事件
    /// 由外部定时器调用
    pub fn scan_lifecycle_events(&self) -> Vec<ProcessLifecycleEvent> {
        if !self.feature_flags.is_enabled(keys::PROCESS_LIFECYCLE).unwrap_or(false) {
            return vec![];
        }

        let activities = self.activities.read();
        let now = Instant::now();
        let mut events = Vec::new();

        for (exec_id, activity) in activities.iter() {
            let idle_secs = now.duration_since(activity.last_active).as_secs();

            if idle_secs > self.config.reclaim_timeout_secs {
                events.push(ProcessLifecycleEvent::Reclaimed {
                    role_id: activity.role_id.clone(),
                    idle_secs,
                });
            } else if idle_secs > self.config.hibernate_timeout_secs {
                events.push(ProcessLifecycleEvent::Hibernated {
                    execution_id: exec_id.clone(),
                    role_id: activity.role_id.clone(),
                    idle_secs,
                });
            }
        }

        events
    }

    /// 清理项目相关的所有进程
    pub fn cleanup_project_processes(&self, project_id: &str) -> Vec<String> {
        let mut activities = self.activities.write();
        let to_remove: Vec<String> = activities.iter()
            .filter(|(_, a)| a.project_id == project_id)
            .map(|(id, _)| id.clone())
            .collect();

        for id in &to_remove {
            activities.remove(id);
        }

        to_remove
    }

    /// 获取配置
    pub fn config(&self) -> &LifecycleConfig {
        &self.config
    }
}

/// 资源统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStats {
    pub active_processes: usize,
    pub max_processes: usize,
    pub total_memory_bytes: u64,
    pub max_memory_bytes: u64,
    /// 应该休眠的 execution_id
    pub idle_candidates: Vec<String>,
    /// 应该回收的 execution_id
    pub hibernated_candidates: Vec<String>,
}
