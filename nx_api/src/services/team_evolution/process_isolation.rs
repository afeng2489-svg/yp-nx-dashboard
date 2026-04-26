//! 进程隔离 — 双进程环境隔离
//!
//! 前后端各自独立 PTY session + 独立 workspace_path。
//! 复用现有 ClaudeTerminalSession，按 role_id 隔离。

use serde::{Deserialize, Serialize};

/// 进程类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProcessType {
    Backend,
    Frontend,
    General,
}

/// 隔离进程描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolatedProcess {
    pub role_id: String,
    pub role_type: ProcessType,
    pub workspace_path: String,
    pub pty_session_id: Option<String>,
    pub env_overrides: Vec<(String, String)>,
}

/// 进程注册表 — 管理所有活跃的隔离进程
pub struct ProcessRegistry {
    processes: parking_lot::RwLock<Vec<IsolatedProcess>>,
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self {
            processes: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// 注册一个隔离进程
    pub fn register(&self, process: IsolatedProcess) -> Result<(), String> {
        let mut processes = self.processes.write();

        // 检查同 role_id 是否已注册
        if processes.iter().any(|p| p.role_id == process.role_id) {
            return Err(format!("Role {} already has a registered process", process.role_id));
        }

        // 校验无同目录争抢
        if let Some(conflict) = Self::find_directory_conflict(&processes, &process) {
            return Err(format!(
                "Directory conflict: role {} and role {} share workspace '{}'",
                conflict.role_id, process.role_id, process.workspace_path
            ));
        }

        processes.push(process);
        Ok(())
    }

    /// 注销进程
    pub fn unregister(&self, role_id: &str) -> Option<IsolatedProcess> {
        let mut processes = self.processes.write();
        let idx = processes.iter().position(|p| p.role_id == role_id)?;
        Some(processes.swap_remove(idx))
    }

    /// 查找进程
    pub fn find(&self, role_id: &str) -> Option<IsolatedProcess> {
        let processes = self.processes.read();
        processes.iter().find(|p| p.role_id == role_id).cloned()
    }

    /// 列出所有进程
    pub fn list_all(&self) -> Vec<IsolatedProcess> {
        self.processes.read().clone()
    }

    /// 按类型筛选
    pub fn list_by_type(&self, process_type: &ProcessType) -> Vec<IsolatedProcess> {
        self.processes.read()
            .iter()
            .filter(|p| &p.role_type == process_type)
            .cloned()
            .collect()
    }

    /// 更新 PTY session ID
    pub fn set_pty_session(&self, role_id: &str, session_id: &str) -> bool {
        let mut processes = self.processes.write();
        if let Some(p) = processes.iter_mut().find(|p| p.role_id == role_id) {
            p.pty_session_id = Some(session_id.to_string());
            true
        } else {
            false
        }
    }

    /// 清除项目相关的所有进程
    pub fn clear_all(&self) {
        self.processes.write().clear();
    }

    /// 当前活跃进程数
    pub fn active_count(&self) -> usize {
        self.processes.read().len()
    }

    /// 查找目录冲突
    fn find_directory_conflict<'a>(
        existing: &'a [IsolatedProcess],
        new_process: &IsolatedProcess,
    ) -> Option<&'a IsolatedProcess> {
        existing.iter().find(|p| {
            p.workspace_path == new_process.workspace_path
                && p.role_id != new_process.role_id
        })
    }
}

/// 根据 role_name 猜测进程类型
pub fn infer_process_type(role_name: &str) -> ProcessType {
    let lower = role_name.to_lowercase();
    if lower.contains("前端") || lower.contains("frontend") || lower.contains("front-end") || lower.contains("react") || lower.contains("vue") {
        ProcessType::Frontend
    } else if lower.contains("后端") || lower.contains("backend") || lower.contains("back-end") || lower.contains("rust") || lower.contains("api") {
        ProcessType::Backend
    } else {
        ProcessType::General
    }
}
