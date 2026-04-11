//! 技能数据模型
//!
//! 定义技能的持久化数据结构。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 技能类别
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillCategory {
    WorkflowPlanning,
    Collaboration,
    Development,
    Testing,
    Review,
    Documentation,
    Research,
    General,
}

impl std::fmt::Display for SkillCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillCategory::WorkflowPlanning => write!(f, "workflow_planning"),
            SkillCategory::Collaboration => write!(f, "collaboration"),
            SkillCategory::Development => write!(f, "development"),
            SkillCategory::Testing => write!(f, "testing"),
            SkillCategory::Review => write!(f, "review"),
            SkillCategory::Documentation => write!(f, "documentation"),
            SkillCategory::Research => write!(f, "research"),
            SkillCategory::General => write!(f, "general"),
        }
    }
}

impl std::str::FromStr for SkillCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "workflow_planning" | "workflowplanning" => Ok(SkillCategory::WorkflowPlanning),
            "collaboration" => Ok(SkillCategory::Collaboration),
            "development" => Ok(SkillCategory::Development),
            "testing" => Ok(SkillCategory::Testing),
            "review" => Ok(SkillCategory::Review),
            "documentation" | "docs" => Ok(SkillCategory::Documentation),
            "research" => Ok(SkillCategory::Research),
            "general" => Ok(SkillCategory::General),
            _ => Err(format!("Unknown category: {}", s)),
        }
    }
}

/// 技能参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillParameter {
    pub name: String,
    pub description: String,
    pub param_type: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

/// 技能元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: SkillCategory,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub parameters: Vec<SkillParameter>,
}

/// 数据库中的技能记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRecord {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub parameters: Vec<SkillParameter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub is_preset: bool,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SkillRecord {
    /// 从数据库行转换为 SkillRecord
    pub fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let tags_json: String = row.get("tags")?;
        let parameters_json: String = row.get("parameters")?;
        let created_at_str: String = row.get("created_at")?;
        let updated_at_str: String = row.get("updated_at")?;

        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let parameters: Vec<SkillParameter> = serde_json::from_str(&parameters_json).unwrap_or_default();

        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            category: row.get("category")?,
            version: row.get("version")?,
            author: row.get("author")?,
            tags,
            parameters,
            code: row.get("code")?,
            is_preset: row.get::<_, i32>("is_preset")? != 0,
            enabled: row.get::<_, i32>("enabled")? != 0,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }
}

/// 创建技能请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateSkillRequest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub tags: Option<Vec<String>>,
    pub parameters: Option<Vec<SkillParameter>>,
    pub code: Option<String>,
}

/// 更新技能请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSkillRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub tags: Option<Vec<String>>,
    pub parameters: Option<Vec<SkillParameter>>,
    pub code: Option<String>,
    pub enabled: Option<bool>,
}

/// 技能摘要 (用于列表展示)
#[derive(Debug, Clone, Serialize)]
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

impl From<&SkillRecord> for SkillSummary {
    fn from(record: &SkillRecord) -> Self {
        Self {
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

/// 技能详情
#[derive(Debug, Clone, Serialize)]
pub struct SkillDetail {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub parameters: Vec<SkillParameter>,
    pub code: Option<String>,
    pub is_preset: bool,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&SkillRecord> for SkillDetail {
    fn from(record: &SkillRecord) -> Self {
        Self {
            id: record.id.clone(),
            name: record.name.clone(),
            description: record.description.clone(),
            category: record.category.clone(),
            version: record.version.clone(),
            author: record.author.clone(),
            tags: record.tags.clone(),
            parameters: record.parameters.clone(),
            code: record.code.clone(),
            is_preset: record.is_preset,
            enabled: record.enabled,
            created_at: record.created_at.to_rfc3339(),
            updated_at: record.updated_at.to_rfc3339(),
        }
    }
}
