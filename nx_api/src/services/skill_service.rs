//! 技能服务
//!
//! 直接从 `.claude/agents/*.md` 文件读写技能，数据库仅用于备份。

use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock as TokioRwLock;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::services::file_skill_repository::{FileSkillRepository, FileSkillRepositoryError};
use crate::models::skill::{CreateSkillRequest, SkillRecord, UpdateSkillRequest};

/// 技能服务错误
#[derive(Debug, thiserror::Error)]
pub enum SkillServiceError {
    #[error("技能不存在: {0}")]
    SkillNotFound(String),

    #[error("技能执行失败: {0}")]
    ExecutionFailed(String),

    #[error("技能注册失败: {0}")]
    RegistrationFailed(String),

    #[error("技能验证失败: {0}")]
    ValidationFailed(String),

    #[error("文件错误: {0}")]
    FileError(String),

    #[error("技能已存在: {0}")]
    AlreadyExists(String),
}

impl From<FileSkillRepositoryError> for SkillServiceError {
    fn from(e: FileSkillRepositoryError) -> Self {
        match e {
            FileSkillRepositoryError::NotFound(id) => SkillServiceError::SkillNotFound(id),
            FileSkillRepositoryError::AlreadyExists(id) => SkillServiceError::AlreadyExists(id),
            FileSkillRepositoryError::ParseError(msg) => SkillServiceError::ValidationFailed(msg),
            _ => SkillServiceError::FileError(e.to_string()),
        }
    }
}

/// 技能摘要信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: String,
    pub tags: Vec<String>,
    pub parameter_count: usize,
    pub is_preset: bool,
}

/// 技能详情
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillDetail {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub parameters: Vec<SkillParameterInfo>,
    pub code: Option<String>,
    pub is_preset: bool,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// 技能参数信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillParameterInfo {
    pub name: String,
    pub description: String,
    pub param_type: String,
    pub required: bool,
    pub default: Option<serde_json::Value>,
}

/// 技能执行请求
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecuteSkillRequest {
    pub skill_id: String,
    pub phase: Option<String>,
    pub params: serde_json::Value,
    pub working_dir: Option<String>,
}

/// 技能执行响应
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecuteSkillResponse {
    pub success: bool,
    pub skill_id: String,
    pub phase: Option<String>,
    pub output: serde_json::Value,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// 技能搜索请求
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchSkillsRequest {
    pub query: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// 类别计数
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CategoryCount {
    pub category: String,
    pub count: usize,
}

/// 标签计数
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TagCount {
    pub tag: String,
    pub count: usize,
}

/// 技能统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillStats {
    pub total_skills: usize,
    pub by_category: Vec<CategoryCount>,
    pub by_tag: Vec<TagCount>,
}

/// 技能服务（文件型）
#[derive(Clone)]
pub struct SkillService {
    file_repo: Arc<FileSkillRepository>,
    /// 最后同步时间
    last_sync: Arc<TokioRwLock<Option<chrono::DateTime<chrono::Utc>>>>,
    /// 同步间隔（秒）
    sync_interval_secs: u64,
    /// 同步进行中标志
    syncing: Arc<AtomicBool>,
}

impl SkillService {
    /// 使用文件目录创建技能服务
    pub fn with_agents_dir(agents_dir: PathBuf) -> Result<Self, SkillServiceError> {
        let file_repo = FileSkillRepository::new(agents_dir)
            .map_err(|e| SkillServiceError::FileError(e.to_string()))?;

        Ok(Self {
            file_repo: Arc::new(file_repo),
            last_sync: Arc::new(TokioRwLock::new(None)),
            sync_interval_secs: 300, // 5分钟同步一次
            syncing: Arc::new(AtomicBool::new(false)),
        })
    }

    /// 获取 agents 目录
    pub fn agents_dir(&self) -> PathBuf {
        self.file_repo.agents_dir().clone()
    }

    /// 创建技能（写文件）
    pub fn create_skill(&self, req: CreateSkillRequest) -> Result<SkillDetail, SkillServiceError> {
        let record = self.file_repo.create(req)?;
        Ok(self.record_to_detail(&record))
    }

    /// 更新技能（写文件）
    pub fn update_skill(&self, id: &str, req: UpdateSkillRequest) -> Result<SkillDetail, SkillServiceError> {
        let record = self.file_repo.update(id, req)?;
        Ok(self.record_to_detail(&record))
    }

    /// 删除技能（删文件）
    pub fn delete_skill(&self, id: &str) -> Result<(), SkillServiceError> {
        self.file_repo.delete(id)?;
        Ok(())
    }

    /// 获取所有技能摘要
    pub fn list_skills(&self) -> Vec<SkillSummary> {
        self.file_repo.list()
            .map(|records| records.iter().map(|r| self.record_to_summary(r)).collect())
            .unwrap_or_default()
    }

    /// 按类别获取技能
    pub fn list_by_category(&self, category: &str) -> Result<Vec<SkillSummary>, SkillServiceError> {
        let records = self.file_repo.list_by_category(category)?;
        Ok(records.iter().map(|r| self.record_to_summary(r)).collect())
    }

    /// 按标签获取技能
    pub fn list_by_tag(&self, tag: &str) -> Vec<SkillSummary> {
        self.file_repo.list_by_tag(tag)
            .map(|records| records.iter().map(|r| self.record_to_summary(r)).collect())
            .unwrap_or_default()
    }

    /// 获取技能详情
    pub fn get_skill(&self, id: &str) -> Result<SkillDetail, SkillServiceError> {
        let record = self.file_repo.get(id)?
            .ok_or_else(|| SkillServiceError::SkillNotFound(id.to_string()))?;
        Ok(self.record_to_detail(&record))
    }

    /// 搜索技能
    pub fn search_skills(&self, query: &str) -> Vec<SkillSummary> {
        self.file_repo.search(query)
            .map(|records| records.iter().map(|r| self.record_to_summary(r)).collect())
            .unwrap_or_default()
    }

    /// 获取所有类别
    pub fn list_categories(&self) -> Vec<String> {
        let records = self.file_repo.list().unwrap_or_default();
        let mut categories: std::collections::HashSet<String> = std::collections::HashSet::new();
        for record in records {
            categories.insert(record.category);
        }
        categories.into_iter().collect()
    }

    /// 获取所有标签
    pub fn list_tags(&self) -> Vec<String> {
        let records = self.file_repo.list().unwrap_or_default();
        let mut tags: std::collections::HashSet<String> = std::collections::HashSet::new();
        for record in records {
            for tag in record.tags {
                tags.insert(tag);
            }
        }
        tags.into_iter().collect()
    }

    /// 获取技能统计
    pub fn get_stats(&self) -> SkillStats {
        let records = self.file_repo.list().unwrap_or_default();

        let mut category_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for record in &records {
            *category_counts.entry(record.category.clone()).or_insert(0) += 1;
        }

        let mut tag_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for record in &records {
            for tag in &record.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        SkillStats {
            total_skills: records.len(),
            by_category: category_counts.into_iter()
                .map(|(category, count)| CategoryCount { category, count })
                .collect(),
            by_tag: tag_counts.into_iter()
                .map(|(tag, count)| TagCount { tag, count })
                .collect(),
        }
    }

    /// 执行技能
    pub async fn execute_skill(
        &self,
        skill_id: &str,
        phase: Option<String>,
        params: serde_json::Value,
        working_dir: Option<String>,
    ) -> Result<ExecuteSkillResponse, SkillServiceError> {
        let record = self.file_repo.get(skill_id)?
            .ok_or_else(|| SkillServiceError::SkillNotFound(skill_id.to_string()))?;

        let start = std::time::Instant::now();

        // 构建执行 prompt
        let phase_str = phase.clone().unwrap_or_else(|| "default".to_string());
        let prompt = format!(
            "Execute the following skill:\n\n\
             Skill: {}\n\
             Description: {}\n\
             Phase: {}\n\
             Parameters: {}\n\n\
             Code:\n```\n{}\n```\n\n\
             Execute this skill and return the result.",
            record.name,
            record.description,
            phase_str,
            serde_json::to_string_pretty(&params).unwrap_or_default(),
            record.code.as_deref().unwrap_or("(no code)"),
        );

        let dir_ref = working_dir.as_deref();
        match crate::services::claude_cli::call_claude_cli_with_timeout(&prompt, 120, dir_ref).await {
            Ok(cli_output) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                Ok(ExecuteSkillResponse {
                    success: true,
                    skill_id: skill_id.to_string(),
                    phase,
                    output: serde_json::json!({ "result": cli_output }),
                    error: None,
                    duration_ms,
                })
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                Ok(ExecuteSkillResponse {
                    success: false,
                    skill_id: skill_id.to_string(),
                    phase,
                    output: serde_json::json!({}),
                    error: Some(e),
                    duration_ms,
                })
            }
        }
    }

    /// 从 agents 目录重新加载技能（扫描文件变化）
    pub fn reload_skills(&self) -> Result<usize, SkillServiceError> {
        self.file_repo.reload_cache()
            .map_err(|e| SkillServiceError::FileError(e.to_string()))?;
        let count = self.file_repo.list().map(|v| v.len()).unwrap_or(0);
        Ok(count)
    }

    /// 获取所有技能文件信息
    pub fn list_skill_files(&self) -> Result<Vec<crate::services::file_skill_repository::SkillFileInfo>, SkillServiceError> {
        self.file_repo.list_files()
            .map_err(|e| SkillServiceError::FileError(e.to_string()))
    }

    /// 将记录转换为详情
    fn record_to_detail(&self, record: &SkillRecord) -> SkillDetail {
        SkillDetail {
            id: record.id.clone(),
            name: record.name.clone(),
            description: record.description.clone(),
            category: record.category.clone(),
            version: record.version.clone(),
            author: record.author.clone(),
            tags: record.tags.clone(),
            parameters: record.parameters.iter().map(|p| SkillParameterInfo {
                name: p.name.clone(),
                description: p.description.clone(),
                param_type: format!("{:?}", p.param_type).to_lowercase(),
                required: p.required,
                default: p.default.clone(),
            }).collect(),
            code: record.code.clone(),
            is_preset: record.is_preset,
            enabled: record.enabled,
            created_at: record.created_at.to_rfc3339(),
            updated_at: record.updated_at.to_rfc3339(),
        }
    }

    /// 将记录转换为摘要
    fn record_to_summary(&self, record: &SkillRecord) -> SkillSummary {
        SkillSummary {
            id: record.id.clone(),
            name: record.name.clone(),
            description: record.description.clone(),
            category: record.category.clone(),
            version: record.version.clone(),
            tags: record.tags.clone(),
            parameter_count: record.parameters.len(),
            is_preset: record.is_preset,
        }
    }
}

impl Default for SkillService {
    fn default() -> Self {
        // 默认使用当前目录下的 .claude/agents
        let agents_dir = std::env::current_dir()
            .unwrap_or_default()
            .join(".claude/agents");
        Self::with_agents_dir(agents_dir).unwrap_or_else(|_| {
            // 创建一个临时服务
            Self {
                file_repo: Arc::new(FileSkillRepository::new(PathBuf::from("/tmp/agents")).unwrap()),
                last_sync: Arc::new(TokioRwLock::new(None)),
                sync_interval_secs: 300,
                syncing: Arc::new(AtomicBool::new(false)),
            }
        })
    }
}
