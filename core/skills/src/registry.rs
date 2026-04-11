//! 技能注册表
//!
//! 管理系统中所有可用的技能，支持技能的注册、查找和调用。

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use thiserror::Error;

use super::{Skill, SkillCategory, SkillId};

/// 技能注册表错误
#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("技能不存在: {0}")]
    NotFound(SkillId),

    #[error("技能已存在: {0}")]
    AlreadyExists(SkillId),

    #[error("类别不存在: {0}")]
    CategoryNotFound(String),
}

/// 技能注册表
///
/// 管理所有可用技能的注册、查找和分类。
#[derive(Debug)]
pub struct SkillRegistry {
    /// 技能存储
    skills: Arc<RwLock<HashMap<SkillId, Skill>>>,
    /// 按类别索引
    by_category: Arc<RwLock<HashMap<SkillCategory, Vec<SkillId>>>>,
    /// 按标签索引
    by_tag: Arc<RwLock<HashMap<String, Vec<SkillId>>>>,
}

impl SkillRegistry {
    /// 创建新的技能注册表
    pub fn new() -> Self {
        Self {
            skills: Arc::new(RwLock::new(HashMap::new())),
            by_category: Arc::new(RwLock::new(HashMap::new())),
            by_tag: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册技能
    pub fn register(&self, skill: Skill) -> Result<(), RegistryError> {
        let skill_id = skill.metadata.id.clone();
        let category = skill.metadata.category;
        let tags = skill.metadata.tags.clone();

        // 检查是否已存在
        {
            let skills = self.skills.read();
            if skills.contains_key(&skill_id) {
                return Err(RegistryError::AlreadyExists(skill_id));
            }
        }

        // 注册技能
        {
            let mut skills = self.skills.write();
            skills.insert(skill_id.clone(), skill);
        }

        // 更新类别索引
        {
            let mut by_category = self.by_category.write();
            by_category
                .entry(category)
                .or_default()
                .push(skill_id.clone());
        }

        // 更新标签索引
        {
            let mut by_tag = self.by_tag.write();
            for tag in tags {
                by_tag.entry(tag).or_default().push(skill_id.clone());
            }
        }

        Ok(())
    }

    /// 批量注册技能
    pub fn register_many(&self, skills: impl IntoIterator<Item = Skill>) -> Result<usize, RegistryError> {
        let mut registered = 0;
        for skill in skills {
            self.register(skill)?;
            registered += 1;
        }
        Ok(registered)
    }

    /// 获取技能
    pub fn get(&self, id: &SkillId) -> Option<Skill> {
        let skills = self.skills.read();
        skills.get(id).cloned()
    }

    /// 获取技能是否存在
    pub fn contains(&self, id: &SkillId) -> bool {
        let skills = self.skills.read();
        skills.contains_key(id)
    }

    /// 获取所有技能
    pub fn all(&self) -> Vec<Skill> {
        let skills = self.skills.read();
        skills.values().cloned().collect()
    }

    /// 按类别获取技能
    pub fn by_category(&self, category: SkillCategory) -> Vec<Skill> {
        let skill_ids = {
            let by_category = self.by_category.read();
            by_category.get(&category).cloned().unwrap_or_default()
        };

        let skills = self.skills.read();
        skill_ids
            .iter()
            .filter_map(|id| skills.get(id).cloned())
            .collect()
    }

    /// 按标签获取技能
    pub fn by_tag(&self, tag: &str) -> Vec<Skill> {
        let skill_ids = {
            let by_tag = self.by_tag.read();
            by_tag.get(tag).cloned().unwrap_or_default()
        };

        let skills = self.skills.read();
        skill_ids
            .iter()
            .filter_map(|id| skills.get(id).cloned())
            .collect()
    }

    /// 搜索技能
    pub fn search(&self, query: &str) -> Vec<Skill> {
        let query_lower = query.to_lowercase();
        let skills = self.skills.read();

        skills
            .values()
            .filter(|skill| {
                skill.metadata.name.to_lowercase().contains(&query_lower)
                    || skill.metadata.description.to_lowercase().contains(&query_lower)
                    || skill.metadata.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }

    /// 获取技能数量
    pub fn len(&self) -> usize {
        let skills = self.skills.read();
        skills.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        let skills = self.skills.read();
        skills.is_empty()
    }

    /// 获取所有类别
    pub fn categories(&self) -> Vec<SkillCategory> {
        let by_category = self.by_category.read();
        by_category.keys().cloned().collect()
    }

    /// 获取所有标签
    pub fn tags(&self) -> Vec<String> {
        let by_tag = self.by_tag.read();
        by_tag.keys().cloned().collect()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SkillMetadata;

    fn create_test_skill(id: &str, category: SkillCategory) -> Skill {
        let metadata = SkillMetadata::new(
            SkillId::new(id),
            format!("测试技能 {}", id),
            "这是一个测试技能"
        )
        .with_category(category)
        .with_tag("test");

        Skill::new(metadata, "test")
    }

    #[test]
    fn test_register_and_get() {
        let registry = SkillRegistry::new();
        let skill = create_test_skill("test-1", SkillCategory::Development);

        registry.register(skill.clone()).unwrap();
        let found = registry.get(&skill.metadata.id);

        assert!(found.is_some());
        assert_eq!(found.unwrap().metadata.name, "测试技能 test-1");
    }

    #[test]
    fn test_duplicate_registration() {
        let registry = SkillRegistry::new();
        let skill = create_test_skill("test-1", SkillCategory::Development);

        registry.register(skill.clone()).unwrap();
        let result = registry.register(skill);

        assert!(result.is_err());
    }

    #[test]
    fn test_by_category() {
        let registry = SkillRegistry::new();

        registry.register(create_test_skill("dev-1", SkillCategory::Development)).unwrap();
        registry.register(create_test_skill("test-1", SkillCategory::Testing)).unwrap();
        registry.register(create_test_skill("dev-2", SkillCategory::Development)).unwrap();

        let dev_skills = registry.by_category(SkillCategory::Development);
        assert_eq!(dev_skills.len(), 2);

        let test_skills = registry.by_category(SkillCategory::Testing);
        assert_eq!(test_skills.len(), 1);
    }

    #[test]
    fn test_search() {
        let registry = SkillRegistry::new();

        registry.register(create_test_skill("search-test", SkillCategory::Development)).unwrap();
        registry.register(create_test_skill("other", SkillCategory::Testing)).unwrap();

        let results = registry.search("search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.id.as_str(), "search-test");
    }
}
