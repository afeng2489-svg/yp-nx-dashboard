//! 技能服务
//!
//! 优先从数据库读写技能，文件系统降级为导入源。
//! 启动时自动 seed 预设技能库，开箱即用。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;

use crate::models::skill::{CreateSkillRequest, SkillRecord, UpdateSkillRequest};
use crate::services::file_skill_repository::{FileSkillRepository, FileSkillRepositoryError};
use crate::services::skill_repository::{
    SkillRepository, SkillRepositoryError, SqliteSkillRepository,
};

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

    #[error("数据库错误: {0}")]
    DatabaseError(String),

    #[error("技能已存在: {0}")]
    AlreadyExists(String),

    #[error("预设技能不可删除: {0}")]
    PresetCannotDelete(String),
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

impl From<SkillRepositoryError> for SkillServiceError {
    fn from(e: SkillRepositoryError) -> Self {
        match e {
            SkillRepositoryError::NotFound(id) => SkillServiceError::SkillNotFound(id),
            SkillRepositoryError::AlreadyExists(id) => SkillServiceError::AlreadyExists(id),
            _ => SkillServiceError::DatabaseError(e.to_string()),
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

/// 技能服务（DB 优先，文件系统为导入源）
#[derive(Clone)]
pub struct SkillService {
    db_repo: Arc<SqliteSkillRepository>,
    file_repo: Option<Arc<FileSkillRepository>>,
    /// 最后同步时间
    last_sync: Arc<TokioRwLock<Option<chrono::DateTime<chrono::Utc>>>>,
    /// 同步间隔（秒）
    sync_interval_secs: u64,
    /// 同步进行中标志
    syncing: Arc<AtomicBool>,
}

impl SkillService {
    /// 创建技能服务（DB 优先 + 可选文件导入源）
    pub fn new(
        db_repo: Arc<SqliteSkillRepository>,
        file_repo: Option<Arc<FileSkillRepository>>,
    ) -> Self {
        let svc = Self {
            db_repo,
            file_repo,
            last_sync: Arc::new(TokioRwLock::new(None)),
            sync_interval_secs: 300,
            syncing: Arc::new(AtomicBool::new(false)),
        };

        // Seed preset skills on startup
        if let Err(e) = svc.seed_presets() {
            tracing::warn!("[SkillService] Failed to seed presets: {}", e);
        }

        svc
    }

    /// 使用文件目录创建（向后兼容）
    pub fn with_agents_dir(agents_dir: PathBuf) -> Result<Self, SkillServiceError> {
        let file_repo = FileSkillRepository::new(agents_dir)
            .map_err(|e| SkillServiceError::FileError(e.to_string()))?;
        // Use in-memory DB as fallback for old code path
        let db_repo = SqliteSkillRepository::new_in_memory()
            .map_err(|e| SkillServiceError::DatabaseError(e.to_string()))?;
        Ok(Self::new(Arc::new(db_repo), Some(Arc::new(file_repo))))
    }

    /// 获取 agents 目录
    pub fn agents_dir(&self) -> PathBuf {
        self.file_repo
            .as_ref()
            .map(|fr| fr.agents_dir().clone())
            .unwrap_or_else(|| PathBuf::from(".claude/agents"))
    }

    /// 创建技能（写 DB）
    pub fn create_skill(&self, req: CreateSkillRequest) -> Result<SkillDetail, SkillServiceError> {
        let record = self.db_repo.create(req)?;
        Ok(self.record_to_detail(&record))
    }

    /// 更新技能（写 DB）
    pub fn update_skill(
        &self,
        id: &str,
        req: UpdateSkillRequest,
    ) -> Result<SkillDetail, SkillServiceError> {
        let record = self.db_repo.update(id, req)?;
        Ok(self.record_to_detail(&record))
    }

    /// 删除技能（预设技能改为禁用）
    pub fn delete_skill(&self, id: &str) -> Result<(), SkillServiceError> {
        let record = self
            .db_repo
            .get(id)?
            .ok_or_else(|| SkillServiceError::SkillNotFound(id.to_string()))?;

        if record.is_preset {
            // Preset skills: disable instead of delete
            self.db_repo.update(
                id,
                UpdateSkillRequest {
                    name: None,
                    description: None,
                    category: None,
                    version: None,
                    author: None,
                    tags: None,
                    parameters: None,
                    code: None,
                    enabled: Some(false),
                },
            )?;
            return Ok(());
        }

        self.db_repo.delete(id)?;
        Ok(())
    }

    /// 获取所有技能摘要
    pub fn list_skills(&self) -> Vec<SkillSummary> {
        self.db_repo
            .list()
            .map(|records| records.iter().map(|r| self.record_to_summary(r)).collect())
            .unwrap_or_default()
    }

    /// 按类别获取技能
    pub fn list_by_category(&self, category: &str) -> Result<Vec<SkillSummary>, SkillServiceError> {
        let records = self.db_repo.list_by_category(category)?;
        Ok(records.iter().map(|r| self.record_to_summary(r)).collect())
    }

    /// 按标签获取技能
    pub fn list_by_tag(&self, tag: &str) -> Vec<SkillSummary> {
        self.db_repo
            .list_by_tag(tag)
            .map(|records| records.iter().map(|r| self.record_to_summary(r)).collect())
            .unwrap_or_default()
    }

    /// 获取技能详情
    pub fn get_skill(&self, id: &str) -> Result<SkillDetail, SkillServiceError> {
        let record = self
            .db_repo
            .get(id)?
            .ok_or_else(|| SkillServiceError::SkillNotFound(id.to_string()))?;
        Ok(self.record_to_detail(&record))
    }

    /// 搜索技能
    pub fn search_skills(&self, query: &str) -> Vec<SkillSummary> {
        self.db_repo
            .search(query)
            .map(|records| records.iter().map(|r| self.record_to_summary(r)).collect())
            .unwrap_or_default()
    }

    /// 获取所有类别
    pub fn list_categories(&self) -> Vec<String> {
        let records = self.db_repo.list().unwrap_or_default();
        let mut categories: std::collections::HashSet<String> = std::collections::HashSet::new();
        for record in records {
            categories.insert(record.category);
        }
        categories.into_iter().collect()
    }

    /// 获取所有标签
    pub fn list_tags(&self) -> Vec<String> {
        let records = self.db_repo.list().unwrap_or_default();
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
        let records = self.db_repo.list().unwrap_or_default();

        let mut category_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for record in &records {
            *category_counts.entry(record.category.clone()).or_insert(0) += 1;
        }

        let mut tag_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for record in &records {
            for tag in &record.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        SkillStats {
            total_skills: records.len(),
            by_category: category_counts
                .into_iter()
                .map(|(category, count)| CategoryCount { category, count })
                .collect(),
            by_tag: tag_counts
                .into_iter()
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
        let record = self
            .db_repo
            .get(skill_id)?
            .ok_or_else(|| SkillServiceError::SkillNotFound(skill_id.to_string()))?;

        if !record.enabled {
            return Err(SkillServiceError::ExecutionFailed(format!(
                "技能 {} 已被禁用",
                skill_id
            )));
        }

        let start = std::time::Instant::now();

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
        match crate::services::claude_cli::call_claude_cli_with_timeout(&prompt, 120, dir_ref).await
        {
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

    /// 从 agents 目录重新加载技能到 DB
    pub fn reload_skills(&self) -> Result<usize, SkillServiceError> {
        self.import_from_agents()
    }

    /// 从文件系统导入所有技能到数据库
    pub fn import_from_agents(&self) -> Result<usize, SkillServiceError> {
        let file_repo = match &self.file_repo {
            Some(fr) => fr,
            None => return Ok(0),
        };

        file_repo
            .reload_cache()
            .map_err(|e| SkillServiceError::FileError(e.to_string()))?;

        let records = file_repo
            .list()
            .map_err(|e| SkillServiceError::FileError(e.to_string()))?;
        let mut imported = 0;
        for record in &records {
            if !self.db_repo.exists(&record.id).unwrap_or(false) {
                let req = CreateSkillRequest {
                    id: record.id.clone(),
                    name: record.name.clone(),
                    description: record.description.clone(),
                    category: record.category.clone(),
                    version: Some(record.version.clone()),
                    author: record.author.clone(),
                    tags: Some(record.tags.clone()),
                    parameters: Some(record.parameters.clone()),
                    code: record.code.clone(),
                };
                if let Err(e) = self.db_repo.create(req) {
                    tracing::warn!("[SkillService] Failed to import skill {}: {}", record.id, e);
                } else {
                    imported += 1;
                }
            }
        }

        tracing::info!(
            "[SkillService] Imported {} skills from agents dir",
            imported
        );
        Ok(imported)
    }

    /// 导入技能（从 URL、文件内容或粘贴文本）
    pub async fn import_skill(
        &self,
        source: &str,
        content: &str,
        filename: Option<&str>,
    ) -> Result<SkillDetail, SkillServiceError> {
        let file_repo = match &self.file_repo {
            Some(fr) => fr,
            None => return Err(SkillServiceError::FileError("文件导入源不可用".to_string())),
        };

        let (md_content, fallback_id) = match source {
            "url" => {
                let url = content.trim();
                if url.is_empty() {
                    return Err(SkillServiceError::ValidationFailed(
                        "URL 不能为空".to_string(),
                    ));
                }
                let response = reqwest::get(url)
                    .await
                    .map_err(|e| SkillServiceError::FileError(format!("下载失败: {}", e)))?;
                if !response.status().is_success() {
                    return Err(SkillServiceError::FileError(format!(
                        "下载失败: HTTP {}",
                        response.status()
                    )));
                }
                let body = response
                    .text()
                    .await
                    .map_err(|e| SkillServiceError::FileError(format!("读取内容失败: {}", e)))?;
                if body.len() > 1_048_576 {
                    return Err(SkillServiceError::ValidationFailed(
                        "文件大小超过 1MB 限制".to_string(),
                    ));
                }
                let url_filename = url
                    .rsplit('/')
                    .next()
                    .unwrap_or("imported-skill")
                    .trim_end_matches(".md");
                (body, url_filename.to_string())
            }
            "file" | "paste" => {
                if content.trim().is_empty() {
                    return Err(SkillServiceError::ValidationFailed(
                        "内容不能为空".to_string(),
                    ));
                }
                let id = filename
                    .map(|f| f.trim_end_matches(".md").to_string())
                    .unwrap_or_else(|| format!("imported-{}", chrono::Utc::now().timestamp()));
                (content.to_string(), id)
            }
            _ => {
                return Err(SkillServiceError::ValidationFailed(format!(
                    "不支持的来源类型: {}",
                    source
                )));
            }
        };

        let record = file_repo.import_from_content(&md_content, &fallback_id)?;

        // Also save to DB
        if !self.db_repo.exists(&record.id).unwrap_or(false) {
            let req = CreateSkillRequest {
                id: record.id.clone(),
                name: record.name.clone(),
                description: record.description.clone(),
                category: record.category.clone(),
                version: Some(record.version.clone()),
                author: record.author.clone(),
                tags: Some(record.tags.clone()),
                parameters: Some(record.parameters.clone()),
                code: record.code.clone(),
            };
            self.db_repo.create(req)?;
        }

        // Read from DB for consistent result
        let db_record = self
            .db_repo
            .get(&record.id)?
            .ok_or_else(|| SkillServiceError::SkillNotFound(record.id.clone()))?;
        Ok(self.record_to_detail(&db_record))
    }

    /// 获取所有技能文件信息
    pub fn list_skill_files(
        &self,
    ) -> Result<Vec<crate::services::file_skill_repository::SkillFileInfo>, SkillServiceError> {
        match &self.file_repo {
            Some(fr) => fr
                .list_files()
                .map_err(|e| SkillServiceError::FileError(e.to_string())),
            None => Ok(vec![]),
        }
    }

    /// Seed preset skills into the database
    fn seed_presets(&self) -> Result<(), SkillServiceError> {
        let presets = Self::preset_skills();
        for req in presets {
            if let Err(e) = self.db_repo.init_preset(req) {
                tracing::warn!("[SkillService] Failed to seed preset: {}", e);
            }
        }
        tracing::info!("[SkillService] Preset skills seeded");
        Ok(())
    }

    /// Built-in preset skill definitions
    fn preset_skills() -> Vec<CreateSkillRequest> {
        vec![
            CreateSkillRequest {
                id: "preset-code-review".to_string(),
                name: "代码审查".to_string(),
                description: "审查代码质量、安全性和最佳实践，给出改进建议".to_string(),
                category: "review".to_string(),
                version: Some("1.0.0".to_string()),
                author: Some("Nexus".to_string()),
                tags: Some(vec!["代码审查".to_string(), "质量".to_string(), "review".to_string()]),
                parameters: None,
                code: Some(
                    "You are a senior code reviewer. Analyze the provided code for:\n\
                     1. Bug risks and logic errors\n\
                     2. Security vulnerabilities\n\
                     3. Performance issues\n\
                     4. Code style and readability\n\
                     5. Missing error handling\n\
                     \n\
                     Provide specific, actionable feedback with line references.\
                     Prioritize findings by severity: Critical > High > Medium > Low."
                    .to_string(),
                ),
            },
            CreateSkillRequest {
                id: "preset-security-audit".to_string(),
                name: "安全审计".to_string(),
                description: "检测 OWASP Top 10 安全漏洞和常见安全风险".to_string(),
                category: "review".to_string(),
                version: Some("1.0.0".to_string()),
                author: Some("Nexus".to_string()),
                tags: Some(vec!["安全".to_string(), "审计".to_string(), "OWASP".to_string()]),
                parameters: None,
                code: Some(
                    "You are a security auditor. Check the code for:\n\
                     1. SQL injection, XSS, CSRF\n\
                     2. Authentication/authorization flaws\n\
                     3. Sensitive data exposure (hardcoded secrets, logs)\n\
                     4. Insecure dependencies\n\
                     5. Input validation gaps\n\
                     6. Race conditions\n\
                     \n\
                     Rate each finding by CVSS severity.\
                     Provide remediation steps for each vulnerability."
                    .to_string(),
                ),
            },
            CreateSkillRequest {
                id: "preset-test-writer".to_string(),
                name: "自动写测试".to_string(),
                description: "为指定模块自动生成单元测试和集成测试，目标覆盖率 80%+".to_string(),
                category: "testing".to_string(),
                version: Some("1.0.0".to_string()),
                author: Some("Nexus".to_string()),
                tags: Some(vec!["测试".to_string(), "TDD".to_string(), "覆盖率".to_string()]),
                parameters: None,
                code: Some(
                    "You are a test engineer. Generate comprehensive tests for the provided code:\n\
                     1. Unit tests for each public function/method\n\
                     2. Edge cases and boundary conditions\n\
                     3. Error handling paths\n\
                     4. Integration tests for API endpoints\n\
                     \n\
                     Follow TDD principles:\n\
                     - Write test first (RED)\n\
                     - Implement minimal code (GREEN)\n\
                     - Refactor (IMPROVE)\n\
                     \n\
                     Target: 80%+ coverage. Use the project's existing test framework."
                    .to_string(),
                ),
            },
            CreateSkillRequest {
                id: "preset-refactor".to_string(),
                name: "代码重构".to_string(),
                description: "重构代码提升可读性、降低复杂度、消除重复代码".to_string(),
                category: "development".to_string(),
                version: Some("1.0.0".to_string()),
                author: Some("Nexus".to_string()),
                tags: Some(vec!["重构".to_string(), "代码质量".to_string(), "DRY".to_string()]),
                parameters: None,
                code: Some(
                    "You are a refactoring specialist. Improve the code by:\n\
                     1. Extract functions for repeated logic (DRY)\n\
                     2. Reduce function complexity (< 50 lines, < 4 nesting levels)\n\
                     3. Improve naming clarity\n\
                     4. Apply appropriate design patterns\n\
                     5. Remove dead code\n\
                     \n\
                     Rules:\n\
                     - Do NOT change external behavior\n\
                     - Make small, incremental changes\n\
                     - Keep tests passing after each change\n\
                     - Explain each refactoring step"
                    .to_string(),
                ),
            },
            CreateSkillRequest {
                id: "preset-doc-writer".to_string(),
                name: "文档生成".to_string(),
                description: "为代码生成 API 文档、README、架构说明等".to_string(),
                category: "documentation".to_string(),
                version: Some("1.0.0".to_string()),
                author: Some("Nexus".to_string()),
                tags: Some(vec!["文档".to_string(), "API文档".to_string(), "README".to_string()]),
                parameters: None,
                code: Some(
                    "You are a technical documentation writer. Generate:\n\
                     1. API documentation with request/response examples\n\
                     2. README with setup instructions and usage\n\
                     3. Architecture decision records (ADR)\n\
                     4. Code comments for non-obvious logic (WHY, not WHAT)\n\
                     \n\
                     Guidelines:\n\
                     - Write in the user's preferred language\n\
                     - Include working code examples\n\
                     - Document error cases and edge cases\n\
                     - Keep docs up-to-date with code changes"
                    .to_string(),
                ),
            },
            CreateSkillRequest {
                id: "preset-bug-fixer".to_string(),
                name: "Bug 修复".to_string(),
                description: "分析 bug 根因并提供最小化修复方案".to_string(),
                category: "development".to_string(),
                version: Some("1.0.0".to_string()),
                author: Some("Nexus".to_string()),
                tags: Some(vec!["Bug修复".to_string(), "调试".to_string(), "根因分析".to_string()]),
                parameters: None,
                code: Some(
                    "You are a bug fix specialist. When given a bug report:\n\
                     1. Reproduce the issue mentally (trace the code path)\n\
                     2. Identify the root cause (not just the symptom)\n\
                     3. Propose a minimal fix that doesn't introduce new bugs\n\
                     4. Verify the fix handles edge cases\n\
                     5. Check for similar bugs elsewhere in the codebase\n\
                     \n\
                     Rules:\n\
                     - Fix the root cause, not the symptom\n\
                     - Minimal diff — don't refactor surrounding code\n\
                     - Add a test that would have caught this bug\n\
                     - Explain the fix in one sentence"
                    .to_string(),
                ),
            },
        ]
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
            parameters: record
                .parameters
                .iter()
                .map(|p| SkillParameterInfo {
                    name: p.name.clone(),
                    description: p.description.clone(),
                    param_type: format!("{:?}", p.param_type).to_lowercase(),
                    required: p.required,
                    default: p.default.clone(),
                })
                .collect(),
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
        let db_repo =
            SqliteSkillRepository::new_in_memory().expect("Failed to create in-memory skill repo");
        Self::new(Arc::new(db_repo), None)
    }
}
