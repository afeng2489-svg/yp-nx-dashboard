//! 文件型技能仓储
//!
//! 直接从 `.claude/agents/*.md` 文件读写技能，支持创建、更新、删除操作。

use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

use crate::models::skill::{CreateSkillRequest, SkillRecord, UpdateSkillRequest};

/// 技能文件仓储错误
#[derive(Debug, thiserror::Error)]
pub enum FileSkillRepositoryError {
    #[error("技能不存在: {0}")]
    NotFound(String),

    #[error("技能已存在: {0}")]
    AlreadyExists(String),

    #[error("文件操作失败: {0}")]
    IoError(#[from] std::io::Error),

    #[error("解析失败: {0}")]
    ParseError(String),

    #[error("无效的操作: {0}")]
    InvalidOperation(String),
}

/// 技能文件元信息
#[derive(Debug, Clone)]
pub struct SkillFileInfo {
    pub path: PathBuf,
    pub id: String,
    pub name: String,
}

/// 文件型技能仓储
#[derive(Clone)]
pub struct FileSkillRepository {
    agents_dir: PathBuf,
    cache: Arc<RwLock<Vec<SkillRecord>>>,
}

impl FileSkillRepository {
    /// 创建文件型技能仓储
    pub fn new(agents_dir: PathBuf) -> Result<Self, FileSkillRepositoryError> {
        if !agents_dir.exists() {
            std::fs::create_dir_all(&agents_dir)?;
        }

        let repo = Self {
            agents_dir,
            cache: Arc::new(RwLock::new(Vec::new())),
        };
        repo.reload_cache()?;
        Ok(repo)
    }

    /// 获取 agents 目录路径
    pub fn agents_dir(&self) -> &PathBuf {
        &self.agents_dir
    }

    /// 重新加载缓存
    pub fn reload_cache(&self) -> Result<(), FileSkillRepositoryError> {
        let mut skills = Vec::new();

        let entries = std::fs::read_dir(&self.agents_dir)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            if let Some(record) = self.parse_file(&path) {
                skills.push(record);
            }
        }

        *self.cache.write() = skills;
        Ok(())
    }

    /// 从文件解析技能
    fn parse_file(&self, path: &PathBuf) -> Option<SkillRecord> {
        let content = std::fs::read_to_string(path).ok()?;
        let file_stem = path.file_stem()?.to_str()?;
        self.parse_content(&content, file_stem).ok()
    }

    /// 解析 md 文件内容
    pub fn parse_content(
        &self,
        content: &str,
        fallback_id: &str,
    ) -> Result<SkillRecord, FileSkillRepositoryError> {
        // 解析 frontmatter
        let mut name = fallback_id.to_string();
        let mut description = String::new();
        let mut category = "general".to_string();
        let mut tags = vec!["agent".to_string()];
        let mut instruction = String::new();

        let mut in_frontmatter = false;
        let mut after_frontmatter = false;
        let mut frontmatter_content = String::new();
        let mut found_closing_delimiter = false;

        for line in content.lines() {
            if !after_frontmatter {
                if line.trim() == "---" {
                    if !in_frontmatter {
                        in_frontmatter = true;
                        continue;
                    } else {
                        // Found closing ---
                        in_frontmatter = false;
                        after_frontmatter = true;
                        found_closing_delimiter = true;
                        frontmatter_content = frontmatter_content.trim().to_string();
                        // 解析 frontmatter（包括 multiline instruction）
                        let (n, d, c, t, inst) =
                            Self::parse_frontmatter_with_instruction(&frontmatter_content);
                        name = n;
                        description = d;
                        category = c;
                        tags = t;
                        if !inst.is_empty() {
                            instruction = inst;
                        }
                        continue;
                    }
                }
                if in_frontmatter {
                    frontmatter_content.push_str(line);
                    frontmatter_content.push('\n');
                }
            } else {
                instruction.push_str(line);
                instruction.push('\n');
            }
        }

        // 处理没有 proper frontmatter 的情况（只有一个 --- 或没有 ---）
        if !found_closing_delimiter {
            // 检查 frontmatter_content 是否包含 name 字段
            let mut has_name = false;
            for line in frontmatter_content.lines() {
                if line.trim().starts_with("name:") {
                    has_name = true;
                    break;
                }
            }

            if has_name {
                // 把它当作 frontmatter 来解析（即使没有 closing ---）
                let (n, d, c, t, inst) =
                    Self::parse_frontmatter_with_instruction(&frontmatter_content);
                name = n;
                description = d;
                category = c;
                tags = t;
                if !inst.is_empty() {
                    instruction = inst;
                }
            } else {
                // 没有 frontmatter，整个内容作为 instruction
                instruction = content.trim().to_string();
            }
        }

        if name.is_empty() {
            return Err(FileSkillRepositoryError::ParseError(
                "技能名称不能为空".to_string(),
            ));
        }

        // 将 name 转换为 id（filename 作为 id）
        let id = std::path::Path::new(fallback_id)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(fallback_id)
            .to_string();

        let now = chrono::Utc::now();

        Ok(SkillRecord {
            id,
            name,
            description,
            category,
            version: "1.0.0".to_string(),
            author: Some("Claude Agent".to_string()),
            tags,
            parameters: vec![],
            code: Some(instruction.trim().to_string()),
            is_preset: true,
            enabled: true,
            created_at: now,
            updated_at: now,
        })
    }

    /// 解析 frontmatter 内容，提取 name, description, category, tags
    /// 同时返回 instruction 的 multiline 内容（如果存在）
    fn parse_frontmatter_with_instruction(
        content: &str,
    ) -> (String, String, String, Vec<String>, String) {
        let mut name = String::new();
        let mut description = String::new();
        let mut category = "general".to_string();
        let mut tags = vec!["agent".to_string()];
        let mut instruction = String::new();

        let mut in_multiline = false;
        let mut multiline_indent: Option<usize> = None;
        let mut current_multiline_key = "";
        let mut multiline_content = String::new();

        for line in content.lines() {
            if in_multiline {
                // 检查是否是无内容的行（仅空白）
                let is_blank = line.trim().is_empty();

                if is_blank {
                    // 空行：如果是 multiline 第一行之后的空行，直接添加（带换行）
                    if multiline_indent.is_some() {
                        multiline_content.push('\n');
                    }
                    // 继续等待内容
                    continue;
                }

                // 非空行，检查缩进
                let first_non_ws = line.find(|c: char| !c.is_whitespace());

                if multiline_indent.is_none() {
                    // 第一行内容，确定缩进级别
                    if let Some(pos) = first_non_ws {
                        multiline_indent = Some(pos);
                        multiline_content.push_str(line[pos..].trim_end());
                    }
                    continue;
                }

                // 检查缩进是否足够
                if let Some(pos) = first_non_ws {
                    if pos >= multiline_indent.unwrap_or(0) {
                        // 继续在 multiline 值中
                        let base_indent = multiline_indent.unwrap_or(0);
                        let stripped = line[base_indent..].trim_end();
                        multiline_content.push('\n');
                        multiline_content.push_str(stripped);
                        continue;
                    }
                }

                // 缩进不够，退出 multiline 模式
                if current_multiline_key == "instruction" {
                    instruction = multiline_content.trim().to_string();
                }
                in_multiline = false;
                multiline_indent = None;
                multiline_content.clear();
                // 不要 continue，让当前行被重新处理
            }

            // 检查是否进入 multiline 模式
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                if value == "|" || value == ">|" || value == "|+" {
                    // 进入 multiline 模式
                    in_multiline = true;
                    current_multiline_key = key;
                    multiline_indent = None;
                    multiline_content.clear();
                    continue;
                }

                match key {
                    "name" => name = value.to_string(),
                    "description" => description = value.to_string(),
                    "category" => category = value.to_string(),
                    "tags" => {
                        if value.starts_with('[') {
                            if let Ok(parsed) = serde_json::from_str::<Vec<String>>(value) {
                                tags = parsed;
                            }
                        } else {
                            tags = value.split(',').map(|s| s.trim().to_string()).collect();
                        }
                    }
                    _ => {}
                }
            }
        }

        // 处理最后可能残留的 multiline（文件结尾）
        if in_multiline && current_multiline_key == "instruction" {
            instruction = multiline_content.trim().to_string();
        }

        (name, description, category, tags, instruction)
    }

    /// 解析 frontmatter 内容（兼容版本）
    fn parse_frontmatter(
        content: &str,
        name: &mut String,
        description: &mut String,
        category: &mut String,
        tags: &mut Vec<String>,
    ) {
        let (n, d, c, t, _) = Self::parse_frontmatter_with_instruction(content);
        *name = n;
        *description = d;
        *category = c;
        *tags = t;
    }

    /// 获取文件路径
    fn get_file_path(&self, id: &str) -> PathBuf {
        self.agents_dir.join(format!("{}.md", id))
    }

    /// 生成 frontmatter
    fn generate_frontmatter(
        name: &str,
        description: &str,
        category: &str,
        tags: &[String],
    ) -> String {
        let tags_json = serde_json::to_string(tags).unwrap_or_else(|_| "[\"agent\"]".to_string());
        format!(
            "---\nname: {}\ndescription: {}\ncategory: {}\ntags: {}\ninstruction: |\n",
            name, description, category, tags_json
        )
    }

    /// 创建技能（写文件）
    pub fn create(&self, req: CreateSkillRequest) -> Result<SkillRecord, FileSkillRepositoryError> {
        let file_path = self.get_file_path(&req.id);

        if file_path.exists() {
            return Err(FileSkillRepositoryError::AlreadyExists(req.id.clone()));
        }

        let frontmatter = Self::generate_frontmatter(
            &req.name,
            &req.description,
            &req.category,
            req.tags.as_deref().unwrap_or(&["agent".to_string()]),
        );

        let code = req.code.as_deref().unwrap_or("");
        let content = format!("{}{}", frontmatter, code);

        std::fs::write(&file_path, content)?;

        let record = self.parse_file(&file_path).ok_or_else(|| {
            FileSkillRepositoryError::ParseError("Failed to parse created file".to_string())
        })?;

        self.reload_cache()?;
        Ok(record)
    }

    /// 更新技能（写文件）
    pub fn update(
        &self,
        id: &str,
        req: UpdateSkillRequest,
    ) -> Result<SkillRecord, FileSkillRepositoryError> {
        let file_path = self.get_file_path(id);

        if !file_path.exists() {
            return Err(FileSkillRepositoryError::NotFound(id.to_string()));
        }

        // 读取现有内容
        let existing = self.parse_file(&file_path).ok_or_else(|| {
            FileSkillRepositoryError::ParseError("Failed to parse existing file".to_string())
        })?;

        let name = req.name.as_ref().unwrap_or(&existing.name);
        let description = req.description.as_ref().unwrap_or(&existing.description);
        let category = req.category.as_ref().unwrap_or(&existing.category);
        let tags = req.tags.as_ref().unwrap_or(&existing.tags);
        let code = req.code.as_ref().or(existing.code.as_ref());

        let frontmatter = Self::generate_frontmatter(name, description, category, tags);
        let code_str = code.as_ref().map(|s| s.as_str()).unwrap_or("");
        let content = format!("{}{}", frontmatter, code_str);

        std::fs::write(&file_path, content)?;

        let record = self.parse_file(&file_path).ok_or_else(|| {
            FileSkillRepositoryError::ParseError("Failed to parse updated file".to_string())
        })?;

        self.reload_cache()?;
        Ok(record)
    }

    /// 删除技能（删文件）
    pub fn delete(&self, id: &str) -> Result<(), FileSkillRepositoryError> {
        let file_path = self.get_file_path(id);

        if !file_path.exists() {
            return Err(FileSkillRepositoryError::NotFound(id.to_string()));
        }

        // 不允许删除预设技能（但这里我们允许，因为文件就是来源）
        // 如果需要保护，可以检查 is_preset 标记

        std::fs::remove_file(&file_path)?;
        self.reload_cache()?;
        Ok(())
    }

    /// 获取单个技能
    pub fn get(&self, id: &str) -> Result<Option<SkillRecord>, FileSkillRepositoryError> {
        let file_path = self.get_file_path(id);

        if !file_path.exists() {
            return Ok(None);
        }

        Ok(self.parse_file(&file_path))
    }

    /// 列出所有技能
    pub fn list(&self) -> Result<Vec<SkillRecord>, FileSkillRepositoryError> {
        Ok(self.cache.read().clone())
    }

    /// 按类别获取
    pub fn list_by_category(
        &self,
        category: &str,
    ) -> Result<Vec<SkillRecord>, FileSkillRepositoryError> {
        Ok(self
            .cache
            .read()
            .iter()
            .filter(|r| r.category == category)
            .cloned()
            .collect())
    }

    /// 按标签获取
    pub fn list_by_tag(&self, tag: &str) -> Result<Vec<SkillRecord>, FileSkillRepositoryError> {
        Ok(self
            .cache
            .read()
            .iter()
            .filter(|r| r.tags.contains(&tag.to_string()))
            .cloned()
            .collect())
    }

    /// 搜索技能
    pub fn search(&self, query: &str) -> Result<Vec<SkillRecord>, FileSkillRepositoryError> {
        let query_lower = query.to_lowercase();
        Ok(self
            .cache
            .read()
            .iter()
            .filter(|r| {
                r.name.to_lowercase().contains(&query_lower)
                    || r.description.to_lowercase().contains(&query_lower)
                    || r.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect())
    }

    /// 从原始 .md 内容导入技能
    pub fn import_from_content(
        &self,
        content: &str,
        fallback_id: &str,
    ) -> Result<SkillRecord, FileSkillRepositoryError> {
        let record = self.parse_content(content, fallback_id)?;

        let file_path = self.get_file_path(&record.id);
        if file_path.exists() {
            return Err(FileSkillRepositoryError::AlreadyExists(record.id.clone()));
        }

        // 直接写入原始内容（保留原始格式）
        std::fs::write(&file_path, content)?;

        self.reload_cache()?;

        // 从缓存中获取最终结果
        self.cache
            .read()
            .iter()
            .find(|r| r.id == record.id)
            .cloned()
            .ok_or_else(|| {
                FileSkillRepositoryError::ParseError("导入后缓存中未找到技能".to_string())
            })
    }

    /// 检查技能是否存在
    pub fn exists(&self, id: &str) -> Result<bool, FileSkillRepositoryError> {
        Ok(self.get_file_path(id).exists())
    }

    /// 列出所有技能文件信息
    pub fn list_files(&self) -> Result<Vec<SkillFileInfo>, FileSkillRepositoryError> {
        let mut files = Vec::new();

        let entries = std::fs::read_dir(&self.agents_dir)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            let file_stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            // 读取文件获取 name
            let name = if let Ok(content) = std::fs::read_to_string(&path) {
                // 简单解析 name
                content
                    .lines()
                    .find(|l| l.starts_with("name:"))
                    .and_then(|l| l.split(':').nth(1))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| file_stem.clone())
            } else {
                file_stem.clone()
            };

            files.push(SkillFileInfo {
                path,
                id: file_stem,
                name,
            });
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_content ─────────────────────────────────────────────────────────

    #[test]
    fn test_parse_content_with_frontmatter() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = FileSkillRepository::new(tmp.path().to_path_buf()).unwrap();

        let md = r#"---
name: My Skill
description: Does something cool
category: development
tags: ["rust", "testing"]
---

This is the instruction body.
"#;
        let record = repo.parse_content(md, "my-skill").unwrap();
        assert_eq!(record.id, "my-skill");
        assert_eq!(record.name, "My Skill");
        assert_eq!(record.description, "Does something cool");
        assert_eq!(record.category, "development");
        assert!(record.tags.contains(&"rust".to_string()));
        assert!(record.tags.contains(&"testing".to_string()));
        assert!(record
            .code
            .as_deref()
            .unwrap_or("")
            .contains("instruction body"));
    }

    #[test]
    fn test_parse_content_no_frontmatter_uses_body_as_code() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = FileSkillRepository::new(tmp.path().to_path_buf()).unwrap();

        let md = "Just raw instructions with no frontmatter.\n";
        let record = repo.parse_content(md, "raw-skill").unwrap();
        assert_eq!(record.id, "raw-skill");
        assert!(record
            .code
            .as_deref()
            .unwrap_or("")
            .contains("raw instructions"));
    }

    #[test]
    fn test_parse_content_defaults_category_to_general() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = FileSkillRepository::new(tmp.path().to_path_buf()).unwrap();

        let md = "---\nname: Min Skill\n---\nBody.\n";
        let record = repo.parse_content(md, "min-skill").unwrap();
        assert_eq!(record.category, "general");
    }

    // ── file CRUD ─────────────────────────────────────────────────────────────

    #[test]
    fn test_create_and_list_skill() {
        use crate::models::skill::CreateSkillRequest;
        let tmp = tempfile::tempdir().unwrap();
        let repo = FileSkillRepository::new(tmp.path().to_path_buf()).unwrap();

        let req = CreateSkillRequest {
            id: "test-skill".to_string(),
            name: "Test Skill".to_string(),
            description: "A test skill".to_string(),
            category: "testing".to_string(),
            version: None,
            author: None,
            tags: Some(vec!["test".to_string()]),
            parameters: None,
            code: Some("Do the thing.".to_string()),
        };

        repo.create(req).unwrap();

        let all = repo.list().unwrap();
        assert!(
            all.iter().any(|s| s.id == "test-skill"),
            "skill should be in list"
        );
    }

    #[test]
    fn test_create_duplicate_returns_error() {
        use crate::models::skill::CreateSkillRequest;
        let tmp = tempfile::tempdir().unwrap();
        let repo = FileSkillRepository::new(tmp.path().to_path_buf()).unwrap();

        let req = CreateSkillRequest {
            id: "dup-skill".to_string(),
            name: "Dup Skill".to_string(),
            description: "desc".to_string(),
            category: "general".to_string(),
            version: None,
            author: None,
            tags: None,
            parameters: None,
            code: None,
        };

        repo.create(req.clone()).unwrap();
        let result = repo.create(req);
        assert!(matches!(
            result,
            Err(FileSkillRepositoryError::AlreadyExists(_))
        ));
    }

    #[test]
    fn test_get_nonexistent_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = FileSkillRepository::new(tmp.path().to_path_buf()).unwrap();

        let result = repo.get("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_skill_removes_from_list() {
        use crate::models::skill::CreateSkillRequest;
        let tmp = tempfile::tempdir().unwrap();
        let repo = FileSkillRepository::new(tmp.path().to_path_buf()).unwrap();

        let req = CreateSkillRequest {
            id: "to-delete".to_string(),
            name: "To Delete".to_string(),
            description: "".to_string(),
            category: "general".to_string(),
            version: None,
            author: None,
            tags: None,
            parameters: None,
            code: None,
        };
        repo.create(req).unwrap();
        repo.delete("to-delete").unwrap();

        let found = repo.get("to-delete").unwrap();
        assert!(found.is_none(), "deleted skill should not be findable");
    }
}
