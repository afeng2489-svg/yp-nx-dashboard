//! Project module service
//!
//! Business logic for module-level project state tracking.

use std::path::Path;
use std::sync::Arc;

use crate::models::project_module::{ModuleStatus, ProjectModule, UpsertModuleRequest};
use crate::services::project_module_repository::{ModuleRepoError, SqliteModuleRepository};

#[derive(Debug)]
pub struct ProjectModuleService {
    repo: Arc<SqliteModuleRepository>,
}

impl ProjectModuleService {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, ModuleRepoError> {
        Ok(Self {
            repo: Arc::new(SqliteModuleRepository::new(db_path)?),
        })
    }

    pub fn get_modules(&self, project_id: &str) -> Result<Vec<ProjectModule>, ModuleRepoError> {
        self.repo.find_by_project(project_id)
    }

    pub fn upsert_module(
        &self,
        project_id: &str,
        req: UpsertModuleRequest,
    ) -> Result<ProjectModule, ModuleRepoError> {
        if let Some(mut existing) = self
            .repo
            .find_by_project_and_name(project_id, &req.module_name)?
        {
            if let Some(status) = req.status {
                existing.status = status;
            }
            if let Some(summary) = req.summary {
                existing.summary = summary;
            }
            if let Some(files) = req.files_changed {
                existing.files_changed = files;
            }
            if let Some(exec_id) = req.last_execution_id {
                existing.last_execution_id = Some(exec_id);
            }
            existing.updated_at = chrono::Utc::now();
            self.repo.update(&existing)?;
            Ok(existing)
        } else {
            let mut module = ProjectModule::new(project_id.to_string(), req.module_name.clone());
            if let Some(status) = req.status {
                module.status = status;
            }
            if let Some(summary) = req.summary {
                module.summary = summary;
            }
            if let Some(files) = req.files_changed {
                module.files_changed = files;
            }
            if let Some(exec_id) = req.last_execution_id {
                module.last_execution_id = Some(exec_id);
            }
            self.repo.create(&module)?;
            Ok(module)
        }
    }

    pub fn delete_module(&self, module_id: &str) -> Result<bool, ModuleRepoError> {
        self.repo.delete(module_id)
    }

    /// Build a text summary of project state for AI prompt injection.
    pub fn build_state_summary(&self, project_id: &str) -> String {
        let modules = match self.repo.find_by_project(project_id) {
            Ok(m) => m,
            Err(_) => return String::new(),
        };
        if modules.is_empty() {
            return String::new();
        }
        let mut lines = vec!["## 当前项目状态".to_string()];
        for m in &modules {
            let status_label = match m.status {
                ModuleStatus::Pending => "未开始",
                ModuleStatus::InProgress => "进行中",
                ModuleStatus::Completed => "已完成",
                ModuleStatus::Failed => "失败",
            };
            let detail = if m.summary.is_empty() {
                String::new()
            } else {
                format!("（{}）", m.summary)
            };
            lines.push(format!(
                "- {} 模块：{}{}",
                m.module_name, status_label, detail
            ));
        }
        lines.join("\n")
    }
}
