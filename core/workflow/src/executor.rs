//! Stage/agent executor routing (AF-04b): Claude CLI vs API.

use crate::parser::{AgentDefinition, StageDefinition, WorkflowError};

/// 执行器类型：`claude_cli` 读写仓库；`api` 纯文本补全。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorKind {
    #[default]
    ClaudeCli,
    Api,
    Auto,
}

impl ExecutorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ExecutorKind::ClaudeCli => "claude_cli",
            ExecutorKind::Api => "api",
            ExecutorKind::Auto => "auto",
        }
    }
}

/// 解析 YAML executor 字符串
pub fn parse_executor_value(raw: &str) -> Result<ExecutorKind, WorkflowError> {
    match raw {
        "claude_cli" | "cli" => Ok(ExecutorKind::ClaudeCli),
        "api" => Ok(ExecutorKind::Api),
        "auto" => Ok(ExecutorKind::Auto),
        other => Err(WorkflowError::Validation(format!(
            "非法 executor 值: '{other}'（允许 claude_cli | api | auto）"
        ))),
    }
}

/// agent.executor > stage.executor > 默认 claude_cli；`auto` 走规则引擎。
pub fn resolve_executor(stage: &StageDefinition, agent: &AgentDefinition) -> ExecutorKind {
    let picked = agent.executor.or(stage.executor).unwrap_or(ExecutorKind::ClaudeCli);
    if picked == ExecutorKind::Auto {
        resolve_auto(stage, agent)
    } else {
        picked
    }
}

fn resolve_auto(stage: &StageDefinition, agent: &AgentDefinition) -> ExecutorKind {
    if stage.quality_gate.is_some() {
        return ExecutorKind::ClaudeCli;
    }
    let blob = format!(
        "{} {} {} {}",
        stage.name, agent.role, agent.id, agent.prompt
    )
    .to_lowercase();
    if blob.contains("quality_gate") {
        return ExecutorKind::ClaudeCli;
    }
    if blob.contains("summary")
        || blob.contains("摘要")
        || blob.contains("report")
        || blob.contains("交付摘要")
    {
        return ExecutorKind::Api;
    }
    if blob.contains("read /")
        || blob.contains("read/")
        || blob.contains("edit /")
        || blob.contains("bash")
        || blob.contains("glob")
        || blob.contains("write ")
    {
        return ExecutorKind::ClaudeCli;
    }
    ExecutorKind::ClaudeCli
}

/// API 车道请求（由 nx_api 注入 AIModelManager 实现）
#[derive(Debug, Clone)]
pub struct ApiCompletionRequest {
    pub system_prompt: String,
    pub user_message: String,
    pub model: String,
    pub max_tokens: usize,
    pub temperature: f32,
    /// 用于成本路由（AF-MM-04）
    pub stage_name: Option<String>,
    pub cost_mode: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ApiCompletionResult {
    pub text: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub provider: String,
    pub estimated_cost_usd: f64,
}

#[async_trait::async_trait]
pub trait ApiExecutor: Send + Sync {
    async fn complete(
        &self,
        request: ApiCompletionRequest,
    ) -> Result<ApiCompletionResult, WorkflowError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::AgentConfig;

    fn stage(name: &str, gate: bool, exec: Option<ExecutorKind>) -> StageDefinition {
        StageDefinition {
            name: name.to_string(),
            executor: exec,
            quality_gate: if gate {
                Some(crate::parser::QualityGate {
                    checks: vec![],
                    on_fail: crate::parser::OnFail::Fail,
                    max_retries: 0,
                    template: None,
                })
            } else {
                None
            },
            ..Default::default()
        }
    }

    fn agent(id: &str, prompt: &str, exec: Option<ExecutorKind>) -> AgentDefinition {
        AgentDefinition {
            id: id.to_string(),
            role: "dev".into(),
            model: "test".into(),
            prompt: prompt.to_string(),
            depends_on: vec![],
            config: AgentConfig::default(),
            extract_vars: vec![],
            output_format: None,
            executor: exec,
        }
    }

    #[test]
    fn default_is_claude_cli() {
        let s = stage("实现", false, None);
        let a = agent("solo", "用 Edit 改代码", None);
        assert_eq!(resolve_executor(&s, &a), ExecutorKind::ClaudeCli);
    }

    #[test]
    fn explicit_api_on_stage() {
        let s = stage("交付摘要", false, Some(ExecutorKind::Api));
        let a = agent("solo", "anything", None);
        assert_eq!(resolve_executor(&s, &a), ExecutorKind::Api);
    }

    #[test]
    fn agent_overrides_stage() {
        let s = stage("s", false, Some(ExecutorKind::Api));
        let a = agent("solo", "x", Some(ExecutorKind::ClaudeCli));
        assert_eq!(resolve_executor(&s, &a), ExecutorKind::ClaudeCli);
    }

    #[test]
    fn quality_gate_forces_cli() {
        let s = stage("实现", true, Some(ExecutorKind::Api));
        let a = agent("solo", "摘要", Some(ExecutorKind::Auto));
        assert_eq!(resolve_executor(&s, &a), ExecutorKind::ClaudeCli);
    }

    #[test]
    fn auto_summary_to_api() {
        let s = stage("交付摘要", false, Some(ExecutorKind::Auto));
        let a = agent("summary", "输出 JSON 交付摘要", None);
        assert_eq!(resolve_executor(&s, &a), ExecutorKind::Api);
    }

    #[test]
    fn auto_bash_to_cli() {
        let s = stage("实现", false, Some(ExecutorKind::Auto));
        let a = agent("solo", "用 Bash 跑测试", None);
        assert_eq!(resolve_executor(&s, &a), ExecutorKind::ClaudeCli);
    }

    #[test]
    fn parse_executor_values() {
        assert_eq!(
            parse_executor_value("claude_cli").unwrap(),
            ExecutorKind::ClaudeCli
        );
        assert_eq!(parse_executor_value("api").unwrap(), ExecutorKind::Api);
        assert!(parse_executor_value("invalid").is_err());
    }

    #[test]
    fn auto_read_tool_to_cli() {
        let s = stage("规划", false, Some(ExecutorKind::Auto));
        let a = agent("solo", "用 Read / Glob 读代码", None);
        assert_eq!(resolve_executor(&s, &a), ExecutorKind::ClaudeCli);
    }
}
