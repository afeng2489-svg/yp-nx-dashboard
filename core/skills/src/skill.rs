//! 技能定义
//!
//! 技能是 NexusFlow 工作流系统的核心执行单元。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// Skill ID and Category
// ============================================================================

/// 技能 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillId(pub String);

impl SkillId {
    /// 从字符串创建技能 ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// 获取字符串表示
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SkillId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for SkillId {
    fn default() -> Self {
        Self::new(Uuid::new_v4().to_string())
    }
}

/// 技能类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillCategory {
    /// 工作流规划
    WorkflowPlanning,
    /// 协作
    Collaboration,
    /// 开发
    Development,
    /// 测试
    Testing,
    /// 评审
    Review,
    /// 文档
    Documentation,
    /// 研究
    Research,
    /// 通用
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

// ============================================================================
// Skill Executor Trait (for dynamic skill execution)
// ============================================================================

/// 技能执行错误
#[derive(Debug, Error)]
pub enum SkillError {
    #[error("技能执行失败: {0}")]
    ExecutionFailed(String),

    #[error("无效的阶段: {0}")]
    InvalidPhase(String),

    #[error("阶段执行失败: {0}")]
    PhaseFailed(String),

    #[error("技能验证失败: {0}")]
    ValidationFailed(String),

    #[error("技能不存在: {0}")]
    NotFound(String),

    #[error("上下文错误: {0}")]
    ContextError(String),
}

/// 技能阶段
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillPhase {
    /// 阶段名称
    pub name: String,
    /// 阶段描述
    pub description: String,
    /// 是否必需
    pub required: bool,
}

impl SkillPhase {
    /// 创建新的技能阶段
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            required: true,
        }
    }

    /// 设置为可选阶段
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }
}

/// 技能执行上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContext {
    /// 输入参数
    pub params: serde_json::Value,
    /// 工作目录
    pub working_dir: Option<String>,
    /// 环境变量
    pub env_vars: std::collections::HashMap<String, String>,
    /// 元数据
    pub metadata: std::collections::HashMap<String, String>,
}

impl SkillContext {
    /// 创建新的技能上下文
    pub fn new(params: serde_json::Value) -> Self {
        Self {
            params,
            working_dir: None,
            env_vars: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// 设置工作目录
    pub fn with_working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// 添加环境变量
    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// 获取参数值
    pub fn get_param(&self, key: &str) -> Option<&serde_json::Value> {
        self.params.get(key)
    }
}

/// 技能执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecutionResult {
    /// 是否成功
    pub success: bool,
    /// 输出结果
    pub output: serde_json::Value,
    /// 执行的阶段
    pub phase: Option<String>,
    /// 错误信息
    pub error: Option<String>,
    /// 执行时间（毫秒）
    pub duration_ms: u64,
}

impl SkillExecutionResult {
    /// 创建成功结果
    pub fn success(phase: impl Into<String>, output: serde_json::Value, duration_ms: u64) -> Self {
        Self {
            success: true,
            output,
            phase: Some(phase.into()),
            error: None,
            duration_ms,
        }
    }

    /// 创建失败结果
    pub fn failure(phase: impl Into<String>, error: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            success: false,
            output: serde_json::Value::Null,
            phase: Some(phase.into()),
            error: Some(error.into()),
            duration_ms,
        }
    }
}

/// 技能执行器 trait
///
/// 所有可执行的技能都必须实现此 trait。
#[async_trait::async_trait]
pub trait SkillExecutor: Send + Sync {
    /// 获取技能名称
    fn name(&self) -> &str;

    /// 获取技能描述
    fn description(&self) -> &str;

    /// 获取技能版本
    fn version(&self) -> &str {
        "1.0.0"
    }

    /// 获取技能类别
    fn category(&self) -> SkillCategory;

    /// 获取技能阶段列表
    fn phases(&self) -> Vec<SkillPhase>;

    /// 验证技能配置
    fn validate(&self, params: &serde_json::Value) -> Result<(), SkillError>;

    /// 执行指定阶段
    ///
    /// # Arguments
    /// * `phase` - 要执行的阶段名称
    /// * `context` - 执行上下文
    ///
    /// # Returns
    /// 执行结果
    async fn execute(
        &self,
        phase: &str,
        context: &SkillContext,
    ) -> Result<SkillExecutionResult, SkillError>;

    /// 获取技能标签
    fn tags(&self) -> Vec<String> {
        Vec::new()
    }
}

/// 技能参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillParameter {
    /// 参数名称
    pub name: String,
    /// 参数描述
    pub description: String,
    /// 参数类型
    pub param_type: ParameterType,
    /// 是否必需
    pub required: bool,
    /// 默认值（可选）
    pub default: Option<serde_json::Value>,
}

/// 参数类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    String,
    Integer,
    Boolean,
    Array,
    Object,
}

/// 技能元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// 技能 ID
    pub id: SkillId,
    /// 技能名称
    pub name: String,
    /// 技能描述
    pub description: String,
    /// 类别
    pub category: SkillCategory,
    /// 版本
    pub version: String,
    /// 作者
    pub author: Option<String>,
    /// 标签
    pub tags: Vec<String>,
    /// 参数定义
    pub parameters: Vec<SkillParameter>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

impl SkillMetadata {
    /// 创建新的技能元数据
    pub fn new(id: SkillId, name: impl Into<String>, description: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            name: name.into(),
            description: description.into(),
            category: SkillCategory::General,
            version: "1.0.0".to_string(),
            author: None,
            tags: Vec::new(),
            parameters: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// 设置类别
    pub fn with_category(mut self, category: SkillCategory) -> Self {
        self.category = category;
        self
    }

    /// 设置版本
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// 添加标签
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// 添加参数
    pub fn with_parameter(mut self, param: SkillParameter) -> Self {
        self.parameters.push(param);
        self
    }
}

/// 技能定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// 元数据
    pub metadata: SkillMetadata,
    /// 技能执行器类型
    pub executor_type: String,
    /// 技能配置
    pub config: serde_json::Value,
}

impl Skill {
    /// 创建新技能
    pub fn new(metadata: SkillMetadata, executor_type: impl Into<String>) -> Self {
        Self {
            metadata,
            executor_type: executor_type.into(),
            config: serde_json::json!({}),
        }
    }

    /// 创建带配置的技能
    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = config;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_id() {
        let id = SkillId::new("test-skill");
        assert_eq!(id.as_str(), "test-skill");
        assert_eq!(id.to_string(), "test-skill");
    }

    #[test]
    fn test_skill_metadata() {
        let metadata = SkillMetadata::new(
            SkillId::new("test"),
            "测试技能",
            "这是一个测试技能"
        )
        .with_category(SkillCategory::Development)
        .with_tag("test")
        .with_tag("example");

        assert_eq!(metadata.name, "测试技能");
        assert_eq!(metadata.category, SkillCategory::Development);
        assert_eq!(metadata.tags.len(), 2);
    }

    #[test]
    fn test_skill_creation() {
        let metadata = SkillMetadata::new(
            SkillId::new("my-skill"),
            "我的技能",
            "技能描述"
        );

        let skill = Skill::new(metadata, "builtin");

        assert_eq!(skill.executor_type, "builtin");
    }
}
