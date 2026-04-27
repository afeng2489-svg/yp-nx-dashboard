//! 工作流 YAML 解析器
//!
//! 解析 YAML 格式的工作流定义文件。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 从 YAML 解析的工作流定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// 工作流名称
    pub name: String,
    /// 工作流版本
    #[serde(default = "default_version")]
    pub version: String,
    /// 工作流描述
    #[serde(default)]
    pub description: Option<String>,
    /// 可以启动此工作流的触发器
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    /// 工作流级变量
    #[serde(default)]
    pub variables: HashMap<String, serde_json::Value>,
    /// 智能体定义
    #[serde(default)]
    pub agents: Vec<AgentDefinition>,
    /// 阶段定义
    #[serde(default)]
    pub stages: Vec<StageDefinition>,
    /// 错误处理
    #[serde(default)]
    pub on_error: Option<ErrorHandler>,
}

fn default_version() -> String {
    "1.0".to_string()
}

/// 触发器定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    /// 触发器类型
    #[serde(rename = "type")]
    pub trigger_type: TriggerType,
    /// 描述
    #[serde(default)]
    pub description: Option<String>,
    /// 对于手动触发器，输入模式
    #[serde(default)]
    pub inputs: Option<HashMap<String, InputDefinition>>,
}

/// 触发器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    /// 手动触发
    Manual,
    /// Webhook 触发
    Webhook,
    /// 定时触发
    Schedule,
    /// 事件触发
    Event,
}

/// 工作流输入定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputDefinition {
    /// 输入类型
    #[serde(rename = "type")]
    pub input_type: String,
    /// 是否必填
    #[serde(default)]
    pub required: bool,
    /// 默认值
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    /// 描述
    #[serde(default)]
    pub description: Option<String>,
}

/// 变量提取规则
/// agent 执行完后，用正则从输出中提取变量写入 WorkflowState
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarExtraction {
    /// 写入 state 的变量名
    pub name: String,
    /// 正则表达式，第一个捕获组为变量值
    /// 例：pattern: "EXTRACT:confidence=([0-9.]+)"
    pub pattern: String,
}

/// 智能体定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// 唯一智能体 ID
    pub id: String,
    /// 智能体角色
    pub role: String,
    /// 使用的模型
    pub model: String,
    /// 系统提示词
    pub prompt: String,
    /// 依赖 (其他智能体 ID)
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// 附加配置
    #[serde(default)]
    pub config: AgentConfig,
    /// 从输出中提取变量（为空则不提取，完全向后兼容）
    #[serde(default)]
    pub extract_vars: Vec<VarExtraction>,
}

/// 智能体配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// 采样温度
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// 最大 token 数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    /// 此智能体可使用的工具
    #[serde(default)]
    pub tools: Vec<String>,
    /// 是否流式输出
    #[serde(default = "default_false")]
    pub stream: bool,
}

fn default_temperature() -> f32 {
    0.7
}
fn default_max_tokens() -> usize {
    4096
}
fn default_false() -> bool {
    false
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: 4096,
            tools: Vec::new(),
            stream: false,
        }
    }
}

/// Stage 类型
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StageType {
    /// 原有类型：运行 agents（默认，向后兼容）
    #[default]
    Agent,
    /// 新增：暂停等待用户在前端做选择
    UserInput,
    /// 新增：循环执行 body_stages 直到 break_condition 为 true
    Loop,
}

/// 阶段跳转规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageTransition {
    /// 跳转条件表达式，引用 state 变量
    /// 格式：  "变量名 == '字符串'"  或  "变量名 >= 数字"
    /// 为空时作为兜底 fallback，无条件跳转
    #[serde(default)]
    pub condition: Option<String>,
    /// 跳转目标 stage 的 name 字段值
    pub goto: String,
}

/// 用户输入选项（配合 stage_type: user_input 使用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInputOption {
    /// 展示给用户的文字
    pub label: String,
    /// 写入 output_var 的值
    pub value: String,
    /// 选项说明（可选）
    #[serde(default)]
    pub description: Option<String>,
}

/// 阶段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageDefinition {
    /// 阶段名称（在 next.goto 中通过此名称引用）
    pub name: String,
    /// Stage 类型（默认 agent，向后兼容）
    #[serde(default)]
    pub stage_type: StageType,
    /// 此阶段的智能体（stage_type=agent 时使用）
    #[serde(default)]
    pub agents: Vec<String>,
    /// 是否并行运行智能体
    #[serde(default = "default_false")]
    pub parallel: bool,
    /// 此阶段的预期输出
    #[serde(default)]
    pub output: Vec<OutputDefinition>,
    /// 即使智能体失败也继续
    #[serde(default)]
    pub continue_on_error: bool,
    /// 条件跳转规则（为空时按 stages 数组顺序执行，向后兼容）
    #[serde(default)]
    pub next: Vec<StageTransition>,

    // ---- user_input 专用字段 ----
    /// 展示给用户的问题文本
    #[serde(default)]
    pub question: Option<String>,
    /// 选项列表
    #[serde(default)]
    pub options: Vec<UserInputOption>,
    /// 用户选择结果写入的变量名
    #[serde(default)]
    pub output_var: Option<String>,

    // ---- loop 专用字段 ----
    /// 循环退出条件（引用 state 变量，格式同 StageTransition.condition）
    #[serde(default)]
    pub break_condition: Option<String>,
    /// 每次循环执行的 stage 名称列表
    #[serde(default)]
    pub body_stages: Vec<String>,
    /// 最大循环次数（超出后工作流 failed）
    #[serde(default = "default_max_loop")]
    pub max_iterations: usize,
}

fn default_max_loop() -> usize {
    10
}

/// 输出定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputDefinition {
    /// 输出路径 (可以是 glob 模式)
    pub path: String,
    /// 内容类型
    #[serde(default)]
    pub content_type: Option<String>,
}

/// 错误处理定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorHandler {
    /// 错误时运行的阶段
    pub stage: String,
    /// 是否先重试
    #[serde(default)]
    pub retry: bool,
    /// 最大重试次数
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
}

fn default_max_retries() -> usize {
    3
}

/// 工作流解析器
pub struct WorkflowParser;

impl WorkflowParser {
    /// 从 YAML 字符串解析工作流
    pub fn parse(yaml: &str) -> Result<WorkflowDefinition, WorkflowError> {
        serde_yaml::from_str(yaml).map_err(|e| WorkflowError::Parse(e.to_string()))
    }

    /// 从文件解析工作流
    pub fn parse_file(path: &std::path::Path) -> Result<WorkflowDefinition, WorkflowError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| WorkflowError::Io(e.to_string()))?;
        Self::parse(&content)
    }

    /// 验证工作流定义
    pub fn validate(workflow: &WorkflowDefinition) -> Result<(), WorkflowError> {
        // 检查重复的智能体 ID
        let mut agent_ids = std::collections::HashSet::new();
        for agent in &workflow.agents {
            if !agent_ids.insert(&agent.id) {
                return Err(WorkflowError::Validation(format!(
                    "重复的智能体 ID: {}",
                    agent.id
                )));
            }
        }

        // 检查智能体依赖是否存在
        for agent in &workflow.agents {
            for dep in &agent.depends_on {
                if !agent_ids.contains(dep) {
                    return Err(WorkflowError::Validation(format!(
                        "智能体 '{}' 依赖不存在的智能体 '{}'",
                        agent.id, dep
                    )));
                }
            }
        }

        // 检查阶段引用的智能体是否有效
        for stage in &workflow.stages {
            for agent_id in &stage.agents {
                if !agent_ids.contains(agent_id) {
                    return Err(WorkflowError::Validation(format!(
                        "阶段 '{}' 引用了不存在的智能体 '{}'",
                        stage.name, agent_id
                    )));
                }
            }
        }

        // 检查重复的阶段名称
        let mut stage_names = std::collections::HashSet::new();
        for stage in &workflow.stages {
            if !stage_names.insert(&stage.name) {
                return Err(WorkflowError::Validation(format!(
                    "重复的阶段名称: {}",
                    stage.name
                )));
            }
        }

        Ok(())
    }
}

/// 工作流解析错误
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("解析错误: {0}")]
    Parse(String),

    #[error("IO 错误: {0}")]
    Io(String),

    #[error("验证错误: {0}")]
    Validation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_WORKFLOW: &str = r#"
name: "Test Workflow"
version: "1.0"

agents:
  - id: "planner"
    role: "architect"
    model: "claude-opus-4-5"
    prompt: "You are an architect"

  - id: "coder"
    role: "developer"
    model: "claude-sonnet-4-5"
    prompt: "You are a developer"
    depends_on: ["planner"]

stages:
  - name: "Planning"
    agents: ["planner"]
    output:
      - path: "./docs/design.md"

  - name: "Implementation"
    agents: ["coder"]
    parallel: false
"#;

    #[test]
    fn test_parse_valid_workflow() {
        let workflow = WorkflowParser::parse(VALID_WORKFLOW).unwrap();
        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.agents.len(), 2);
        assert_eq!(workflow.stages.len(), 2);
    }

    #[test]
    fn test_validate_workflow() {
        let workflow = WorkflowParser::parse(VALID_WORKFLOW).unwrap();
        assert!(WorkflowParser::validate(&workflow).is_ok());
    }

    #[test]
    fn test_detect_circular_dependency() {
        let yaml = r#"
name: "Circular Workflow"
agents:
  - id: "a"
    role: "r1"
    model: "m1"
    prompt: "p1"
    depends_on: ["b"]
  - id: "b"
    role: "r2"
    model: "m2"
    prompt: "p2"
    depends_on: ["a"]
stages:
  - name: "S1"
    agents: ["a"]
"#;
        let workflow = WorkflowParser::parse(yaml).unwrap();
        // 循环依赖在解析层面允许，会在运行时捕获
        assert!(WorkflowParser::validate(&workflow).is_ok());
    }
}
