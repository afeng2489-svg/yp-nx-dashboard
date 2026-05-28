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
    /// 预算上限（美元），超 80% 告警，超 100% 自动暂停
    #[serde(default)]
    pub budget_limit_usd: Option<f64>,
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
    /// Schedule 触发的 cron 表达式（5字段: min hour dom month dow）
    #[serde(default)]
    pub cron: Option<String>,
    /// Event/链式触发的目标 workflow name
    #[serde(default)]
    pub workflow_ref: Option<String>,
    /// 链式触发是否传递上游输出作为 variables
    #[serde(default)]
    pub pass_output: Option<bool>,
    /// Webhook 触发的验证密钥
    #[serde(default)]
    pub secret: Option<String>,
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
    /// 自定义输出格式（None = 使用默认结构化输出指令）
    #[serde(default)]
    pub output_format: Option<String>,
    /// 执行器：claude_cli | api | auto（默认 claude_cli）
    #[serde(default)]
    pub executor: Option<crate::executor::ExecutorKind>,
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
    /// 人工审批：approve 继续 / reject 跳转重跑指定 stage
    Approval,
    /// 循环执行 body_stages 直到 break_condition 为 true
    Loop,
    /// 页面生成阶段（React 模板 + manifest）
    ///
    /// **Deprecated (AF-00b)**：工厂台 MVP / Golden Path 不使用此 stage。
    /// 请用 `agent` + `solo-dev` 工作流。保留解析与引擎路径仅为向后兼容。
    #[deprecated(
        since = "0.2.0",
        note = "Factory MVP uses Agent stages only; see docs/sprints/AF-00b-code-hygiene.yaml"
    )]
    #[serde(rename = "page_generate")]
    PageGenerate {
        manifest_template: String,
        output_dir: String,
    },
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

/// 质量门检查命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityCheck {
    /// 要执行的 shell 命令
    pub cmd: String,
    /// 超时时间（秒），默认 300
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_timeout() -> u64 {
    300
}

/// 质量门失败策略
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OnFail {
    /// 重试当前 stage
    #[default]
    Retry,
    /// 直接标记失败，不重试
    Fail,
}

/// 质量门定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGate {
    /// 检查命令列表
    pub checks: Vec<QualityCheck>,
    /// 失败策略
    #[serde(default)]
    pub on_fail: OnFail,
    /// 最大重试次数（默认 3）
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
    /// 引用内置模板名称（如 "rust_default"），与 checks 二选一
    #[serde(default)]
    pub template: Option<String>,
}

/// RAG 检索配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    pub knowledge_base_id: String,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    #[serde(default = "default_threshold")]
    pub threshold: f32,
}

fn default_top_k() -> usize {
    5
}
fn default_threshold() -> f32 {
    0.7
}

/// 阶段定义
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    /// 质量门：stage 完成后自动验证
    #[serde(default)]
    pub quality_gate: Option<QualityGate>,
    /// RAG 检索配置：stage 执行前自动检索并注入 prompt
    #[serde(default)]
    pub rag: Option<RagConfig>,
    /// 覆盖全局模型（None = 使用全局默认或自动路由）
    #[serde(default)]
    pub model: Option<String>,
    /// 失败自愈策略
    #[serde(default)]
    pub on_fail: Option<StageFailPolicy>,

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
    /// approval 专用：reject 时跳转重跑的 stage name
    #[serde(default)]
    pub on_reject_goto: Option<String>,

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
    /// 阶段级执行器覆盖
    #[serde(default)]
    pub executor: Option<crate::executor::ExecutorKind>,
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

/// Stage 级失败自愈策略
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageFailPolicy {
    /// 同模型重试次数
    #[serde(default = "default_stage_retries")]
    pub retry: usize,
    /// 同模型重试耗尽后升级到此模型
    #[serde(default)]
    pub escalate_model: Option<String>,
    /// 升级模型重试次数
    #[serde(default = "default_escalate_retries")]
    pub escalate_retries: usize,
    /// 全部失败后的动作：rollback | continue | fail（默认 fail）
    #[serde(default = "default_then_action")]
    pub then: String,
}

fn default_stage_retries() -> usize {
    2
}
fn default_escalate_retries() -> usize {
    1
}
fn default_then_action() -> String {
    "fail".to_string()
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

    // ── Basic parsing ──

    #[test]
    fn test_parse_valid_workflow() {
        let workflow = WorkflowParser::parse(VALID_WORKFLOW).unwrap();
        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.agents.len(), 2);
        assert_eq!(workflow.stages.len(), 2);
    }

    #[test]
    fn test_parse_minimal_workflow() {
        let yaml = r#"
name: "Minimal"
stages:
  - name: "S1"
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        assert_eq!(wf.name, "Minimal");
        assert_eq!(wf.version, "1.0");
        assert_eq!(wf.stages.len(), 1);
    }

    #[test]
    fn test_parse_empty_string_errors() {
        assert!(WorkflowParser::parse("").is_err());
    }

    #[test]
    fn test_parse_invalid_yaml_errors() {
        assert!(WorkflowParser::parse("not: [valid: yaml").is_err());
    }

    // ── Validation ──

    #[test]
    fn test_validate_workflow() {
        let workflow = WorkflowParser::parse(VALID_WORKFLOW).unwrap();
        assert!(WorkflowParser::validate(&workflow).is_ok());
    }

    #[test]
    fn test_validate_duplicate_agent_id() {
        let yaml = r#"
name: "Dup Agents"
agents:
  - id: "a"
    role: "r"
    model: "m"
    prompt: "p"
  - id: "a"
    role: "r"
    model: "m"
    prompt: "p"
stages:
  - name: "S1"
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        let err = WorkflowParser::validate(&wf).unwrap_err();
        assert!(err.to_string().contains("重复"));
    }

    #[test]
    fn test_validate_duplicate_stage_name() {
        let yaml = r#"
name: "Dup Stages"
agents:
  - id: "a"
    role: "r"
    model: "m"
    prompt: "p"
stages:
  - name: "S1"
  - name: "S1"
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        let err = WorkflowParser::validate(&wf).unwrap_err();
        assert!(err.to_string().contains("重复"));
    }

    #[test]
    fn test_validate_missing_agent_dependency() {
        let yaml = r#"
name: "Bad Dep"
agents:
  - id: "a"
    role: "r"
    model: "m"
    prompt: "p"
    depends_on: ["nonexistent"]
stages:
  - name: "S1"
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        let err = WorkflowParser::validate(&wf).unwrap_err();
        assert!(err.to_string().contains("依赖"));
    }

    #[test]
    fn test_validate_stage_refs_nonexistent_agent() {
        let yaml = r#"
name: "Bad Stage"
stages:
  - name: "S1"
    agents: ["ghost"]
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        let err = WorkflowParser::validate(&wf).unwrap_err();
        assert!(err.to_string().contains("不存在"));
    }

    #[test]
    fn test_validate_with_self_dependency() {
        let yaml = r#"
name: "Self Dep"
agents:
  - id: "a"
    role: "r"
    model: "m"
    prompt: "p"
    depends_on: ["a"]
stages:
  - name: "S1"
    agents: ["a"]
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        // Self-dependency is a valid reference (exists in agent_ids), runtime behavior TBD
        assert!(WorkflowParser::validate(&wf).is_ok());
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

    // ── Stage transitions ──

    #[test]
    fn test_parse_stage_transitions() {
        let yaml = r#"
name: "Conditional"
agents:
  - id: "a"
    role: "r"
    model: "m"
    prompt: "p"
stages:
  - name: "Check"
    agents: ["a"]
    next:
      - condition: "status == 'ok'"
        goto: "Deploy"
      - goto: "Rollback"
  - name: "Deploy"
    agents: ["a"]
  - name: "Rollback"
    agents: ["a"]
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        assert_eq!(wf.stages[0].next.len(), 2);
        assert_eq!(wf.stages[0].next[0].condition.as_deref(), Some("status == 'ok'"));
        assert_eq!(wf.stages[0].next[0].goto, "Deploy");
        assert_eq!(wf.stages[0].next[1].condition, None);
        assert_eq!(wf.stages[0].next[1].goto, "Rollback");
    }

    // ── Triggers ──

    #[test]
    fn test_parse_triggers() {
        let yaml = r#"
name: "Triggered"
triggers:
  - type: schedule
    cron: "0 9 * * 1-5"
  - type: webhook
    secret: "abc123"
stages:
  - name: "S1"
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        assert_eq!(wf.triggers.len(), 2);
        assert_eq!(wf.triggers[0].trigger_type, TriggerType::Schedule);
        assert_eq!(wf.triggers[0].cron.as_deref(), Some("0 9 * * 1-5"));
        assert_eq!(wf.triggers[1].trigger_type, TriggerType::Webhook);
        assert_eq!(wf.triggers[1].secret.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_parse_chain_trigger() {
        let yaml = r#"
name: "Chain"
triggers:
  - type: event
    workflow_ref: "upstream_wf"
    pass_output: true
stages:
  - name: "S1"
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        assert_eq!(wf.triggers[0].trigger_type, TriggerType::Event);
        assert_eq!(wf.triggers[0].workflow_ref.as_deref(), Some("upstream_wf"));
        assert_eq!(wf.triggers[0].pass_output, Some(true));
    }

    // ── Quality gate parsing ──

    #[test]
    fn test_parse_quality_gate() {
        let yaml = r#"
name: "Gated"
agents:
  - id: "a"
    role: "r"
    model: "m"
    prompt: "p"
stages:
  - name: "Build"
    agents: ["a"]
    quality_gate:
      checks:
        - cmd: "cargo build"
          timeout: 300
      on_fail: retry
      max_retries: 3
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        let gate = wf.stages[0].quality_gate.as_ref().unwrap();
        assert_eq!(gate.checks.len(), 1);
        assert_eq!(gate.checks[0].cmd, "cargo build");
        assert_eq!(gate.checks[0].timeout, 300);
        assert_eq!(gate.on_fail, OnFail::Retry);
        assert_eq!(gate.max_retries, 3);
    }

    #[test]
    fn test_parse_quality_gate_template() {
        let yaml = r#"
name: "Templated"
agents:
  - id: "a"
    role: "r"
    model: "m"
    prompt: "p"
stages:
  - name: "Build"
    agents: ["a"]
    quality_gate:
      checks: []
      template: "rust_default"
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        let gate = wf.stages[0].quality_gate.as_ref().unwrap();
        assert_eq!(gate.template.as_deref(), Some("rust_default"));
    }

    // ── UserInput stage ──

    #[test]
    fn test_parse_user_input_stage() {
        let yaml = r#"
name: "Interactive"
stages:
  - name: "Choose"
    stage_type: user_input
    question: "Pick one"
    options:
      - label: "Option A"
        value: "a"
      - label: "Option B"
        value: "b"
        description: "The second option"
    output_var: "choice"
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        let stage = &wf.stages[0];
        assert_eq!(stage.stage_type, StageType::UserInput);
        assert_eq!(stage.question.as_deref(), Some("Pick one"));
        assert_eq!(stage.options.len(), 2);
        assert_eq!(stage.options[0].label, "Option A");
        assert_eq!(stage.options[0].value, "a");
        assert_eq!(stage.options[1].description.as_deref(), Some("The second option"));
        assert_eq!(stage.output_var.as_deref(), Some("choice"));
    }

    #[test]
    fn test_parse_approval_stage() {
        let yaml = r#"
name: "ApprovalFlow"
stages:
  - name: "交付审批"
    stage_type: approval
    question: "是否批准合入？"
    output_var: approval_result
    on_reject_goto: 实现
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        let stage = &wf.stages[0];
        assert_eq!(stage.stage_type, StageType::Approval);
        assert_eq!(stage.on_reject_goto.as_deref(), Some("实现"));
    }

    // ── Loop stage ──

    #[test]
    fn test_parse_loop_stage() {
        let yaml = r#"
name: "Looper"
agents:
  - id: "a"
    role: "r"
    model: "m"
    prompt: "p"
stages:
  - name: "RepeatUntil"
    stage_type: loop
    break_condition: "count >= '5'"
    body_stages: ["Inner"]
    max_iterations: 10
  - name: "Inner"
    agents: ["a"]
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        let stage = &wf.stages[0];
        assert_eq!(stage.stage_type, StageType::Loop);
        assert_eq!(stage.break_condition.as_deref(), Some("count >= '5'"));
        assert_eq!(stage.body_stages, vec!["Inner"]);
        assert_eq!(stage.max_iterations, 10);
    }

    // ── Variables ──

    #[test]
    fn test_parse_workflow_variables() {
        let yaml = r#"
name: "WithVars"
variables:
  project_name: "nexus"
  debug: true
stages:
  - name: "S1"
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        assert_eq!(wf.variables.len(), 2);
        assert_eq!(wf.variables["project_name"].as_str().unwrap(), "nexus");
        assert_eq!(wf.variables["debug"].as_bool().unwrap(), true);
    }

    // ── Budget limit ──

    #[test]
    fn test_parse_budget_limit() {
        let yaml = r#"
name: "Budgeted"
budget_limit_usd: 10.50
stages:
  - name: "S1"
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        assert_eq!(wf.budget_limit_usd, Some(10.50));
    }

    // ── Error handler ──

    #[test]
    fn test_parse_error_handler() {
        let yaml = r#"
name: "ErrHandler"
stages:
  - name: "S1"
on_error:
  stage: "fallback"
  retry: true
  max_retries: 3
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        let eh = wf.on_error.as_ref().unwrap();
        assert_eq!(eh.stage, "fallback");
        assert!(eh.retry);
        assert_eq!(eh.max_retries, 3);
    }

    // ── Stage fail policy ──

    #[test]
    fn test_parse_stage_fail_policy() {
        let yaml = r#"
name: "FailPolicy"
agents:
  - id: "a"
    role: "r"
    model: "m"
    prompt: "p"
stages:
  - name: "Risky"
    agents: ["a"]
    on_fail:
      retry: 3
      escalate_model: "claude-opus-4-5"
      escalate_retries: 2
      then: "continue"
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        let policy = wf.stages[0].on_fail.as_ref().unwrap();
        assert_eq!(policy.retry, 3);
        assert_eq!(policy.escalate_model.as_deref(), Some("claude-opus-4-5"));
        assert_eq!(policy.escalate_retries, 2);
        assert_eq!(policy.then, "continue");
    }

    // ── Default values ──

    #[test]
    fn test_default_max_loop() {
        assert_eq!(default_max_loop(), 10);
    }

    #[test]
    fn test_default_max_retries() {
        assert_eq!(default_max_retries(), 3);
    }

    #[test]
    fn test_stage_default_type_is_agent() {
        let yaml = r#"
name: "DefaultType"
agents:
  - id: "a"
    role: "r"
    model: "m"
    prompt: "p"
stages:
  - name: "S1"
    agents: ["a"]
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        assert_eq!(wf.stages[0].stage_type, StageType::Agent);
    }

    // ── RAG config ──

    #[test]
    fn test_parse_rag_config() {
        let yaml = r#"
name: "Ragged"
agents:
  - id: "a"
    role: "r"
    model: "m"
    prompt: "p"
stages:
  - name: "Research"
    agents: ["a"]
    rag:
      knowledge_base_id: "kb-docs"
      top_k: 10
      threshold: 0.8
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        let rag = wf.stages[0].rag.as_ref().unwrap();
        assert_eq!(rag.knowledge_base_id, "kb-docs");
        assert_eq!(rag.top_k, 10);
        assert_eq!(rag.threshold, 0.8);
    }

    // ── VarExtraction ──

    #[test]
    fn test_parse_var_extraction() {
        let yaml = r#"
name: "Extractor"
agents:
  - id: "a"
    role: "r"
    model: "m"
    prompt: "p"
    extract_vars:
      - name: "confidence"
        pattern: "CONFIDENCE=([0-9.]+)"
stages:
  - name: "S1"
    agents: ["a"]
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        assert_eq!(wf.agents[0].extract_vars.len(), 1);
        assert_eq!(wf.agents[0].extract_vars[0].name, "confidence");
        assert_eq!(
            wf.agents[0].extract_vars[0].pattern,
            "CONFIDENCE=([0-9.]+)"
        );
    }

    // ── Stage model override ──

    #[test]
    fn test_parse_stage_model_override() {
        let yaml = r#"
name: "ModelOverride"
agents:
  - id: "a"
    role: "r"
    model: "m"
    prompt: "p"
stages:
  - name: "Hard"
    agents: ["a"]
    model: "claude-opus-4-5"
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        assert_eq!(
            wf.stages[0].model.as_deref(),
            Some("claude-opus-4-5")
        );
    }

    // ── Continue on error flag ──

    #[test]
    fn test_parse_continue_on_error() {
        let yaml = r#"
name: "Lenient"
agents:
  - id: "a"
    role: "r"
    model: "m"
    prompt: "p"
stages:
  - name: "MayFail"
    agents: ["a"]
    continue_on_error: true
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        assert!(wf.stages[0].continue_on_error);
    }

    #[test]
    fn test_parse_executor_field() {
        let yaml = r#"
name: ex
agents:
  - id: a
    role: r
    model: m
    prompt: p
    executor: api
stages:
  - name: S
    executor: claude_cli
    agents: [a]
"#;
        let wf = WorkflowParser::parse(yaml).unwrap();
        assert_eq!(wf.agents[0].executor, Some(crate::executor::ExecutorKind::Api));
        assert_eq!(wf.stages[0].executor, Some(crate::executor::ExecutorKind::ClaudeCli));
    }
}
