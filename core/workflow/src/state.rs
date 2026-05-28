//! 工作流状态管理

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 工作流执行状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    /// 唯一执行 ID
    pub execution_id: Uuid,
    /// 工作流 ID
    pub workflow_id: String,
    /// 当前状态
    pub status: WorkflowStatus,
    /// 执行过程中累积的变量
    pub variables: HashMap<String, serde_json::Value>,
    /// 当前阶段索引
    pub current_stage: usize,
    /// 阶段结果
    pub stage_results: Vec<StageResult>,
    /// 智能体状态
    pub agent_states: HashMap<String, AgentState>,
    /// 开始时间
    pub started_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 完成时间 (如果已完成)
    pub finished_at: Option<DateTime<Utc>>,
    /// 错误信息 (如果失败)
    pub error: Option<String>,
}

impl WorkflowState {
    /// 创建新的工作流状态
    pub fn new(workflow_id: &str) -> Self {
        let now = Utc::now();
        Self {
            execution_id: Uuid::new_v4(),
            workflow_id: workflow_id.to_string(),
            status: WorkflowStatus::Pending,
            variables: HashMap::new(),
            current_stage: 0,
            stage_results: Vec::new(),
            agent_states: HashMap::new(),
            started_at: now,
            updated_at: now,
            finished_at: None,
            error: None,
        }
    }

    /// 设置变量值
    pub fn set_var(&mut self, key: &str, value: serde_json::Value) {
        self.variables.insert(key.to_string(), value);
        self.updated_at = Utc::now();
    }

    /// 获取变量值
    pub fn get_var(&self, key: &str) -> Option<&serde_json::Value> {
        self.variables.get(key)
    }

    /// 使用当前变量解析模板字符串
    ///
    /// 支持两类占位符：
    /// - `{{var_name}}`        → variables HashMap 中的值
    /// - `{{agent_id_output}}` → agent_states[agent_id].last_message（agent 完整输出）
    pub fn resolve_template(&self, template: &str) -> String {
        let mut result = template.to_string();

        // 1. 先替换 variables（extract_vars 提取的值优先级高）
        for (key, value) in &self.variables {
            let value_str = match value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            let placeholder1 = format!("{{{{{}}}}}", key);
            let placeholder2 = format!("{{{{ {} }}}}", key);
            result = result.replace(&placeholder1, &value_str);
            result = result.replace(&placeholder2, &value_str);
        }

        // 2. 替换 {{agent_id_output}} → agent 的完整输出（last_message）
        for (agent_id, agent_state) in &self.agent_states {
            if let Some(ref output) = agent_state.last_message {
                let key = format!("{}_output", agent_id);
                let placeholder1 = format!("{{{{{}}}}}", key);
                let placeholder2 = format!("{{{{ {} }}}}", key);
                result = result.replace(&placeholder1, output);
                result = result.replace(&placeholder2, output);
            }
        }

        result
    }

    /// 记录阶段结果
    pub fn record_stage(
        &mut self,
        stage_name: &str,
        outputs: Vec<StageOutput>,
        quality_gate_result: Option<QualityGateResult>,
    ) {
        self.stage_results.push(StageResult {
            stage_name: stage_name.to_string(),
            outputs,
            completed_at: Utc::now(),
            quality_gate_result,
        });
        self.current_stage += 1;
        self.updated_at = Utc::now();
    }

    /// 更新智能体状态
    pub fn update_agent(&mut self, agent_id: &str, state: AgentState) {
        self.agent_states.insert(agent_id.to_string(), state);
        self.updated_at = Utc::now();
    }

    /// 标记工作流已开始
    pub fn start(&mut self) {
        self.status = WorkflowStatus::Running;
        self.updated_at = Utc::now();
    }

    /// 标记工作流已完成
    pub fn complete(&mut self) {
        self.status = WorkflowStatus::Completed;
        self.finished_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// 标记工作流失败
    pub fn fail(&mut self, error: String) {
        self.status = WorkflowStatus::Failed;
        self.error = Some(error);
        self.finished_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// 标记工作流已取消
    pub fn cancel(&mut self) {
        self.status = WorkflowStatus::Cancelled;
        self.finished_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// 检查工作流是否应该停止
    pub fn should_stop(&self) -> bool {
        matches!(
            self.status,
            WorkflowStatus::Completed | WorkflowStatus::Failed | WorkflowStatus::Cancelled
        )
    }
}

/// 工作流执行状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    /// 等待中
    Pending,
    /// 运行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

impl std::fmt::Display for WorkflowStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowStatus::Pending => write!(f, "pending"),
            WorkflowStatus::Running => write!(f, "running"),
            WorkflowStatus::Completed => write!(f, "completed"),
            WorkflowStatus::Failed => write!(f, "failed"),
            WorkflowStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// 质量门单条检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityCheckResult {
    /// 执行的命令
    pub cmd: String,
    /// 是否通过
    pub passed: bool,
    /// 退出码
    pub exit_code: Option<i32>,
    /// stdout（截断到前 2000 字符）
    pub stdout: String,
    /// stderr（截断到前 2000 字符）
    pub stderr: String,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
}

/// 质量门整体结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGateResult {
    /// 是否全部通过
    pub passed: bool,
    /// 各条检查结果
    pub checks: Vec<QualityCheckResult>,
    /// 当前重试次数（0 表示首次）
    pub retry_count: usize,
}

/// 阶段执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub stage_name: String,
    pub outputs: Vec<StageOutput>,
    pub completed_at: DateTime<Utc>,
    /// 质量门结果（如果 stage 配置了 quality_gate）
    #[serde(default)]
    pub quality_gate_result: Option<QualityGateResult>,
}

/// 阶段的输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageOutput {
    /// 输出路径
    pub path: String,
    /// 输出内容（原始文本）
    pub content: Option<String>,
    /// 智能体 ID
    pub agent_id: Option<String>,
    /// 结构化摘要
    #[serde(default)]
    pub summary: Option<String>,
    /// 变更的文件列表
    #[serde(default)]
    pub files_changed: Vec<String>,
}

/// 执行过程中的智能体状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub agent_id: String,
    pub role: String,
    pub status: AgentStatus,
    pub last_message: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── WorkflowState::new ──

    #[test]
    fn state_new_sets_defaults() {
        let state = WorkflowState::new("my_workflow");
        assert_eq!(state.workflow_id, "my_workflow");
        assert_eq!(state.status, WorkflowStatus::Pending);
        assert!(state.variables.is_empty());
        assert_eq!(state.current_stage, 0);
        assert!(state.stage_results.is_empty());
        assert!(state.agent_states.is_empty());
        assert!(state.finished_at.is_none());
        assert!(state.error.is_none());
    }

    #[test]
    fn state_new_generates_unique_execution_ids() {
        let s1 = WorkflowState::new("wf");
        let s2 = WorkflowState::new("wf");
        assert_ne!(s1.execution_id, s2.execution_id);
    }

    // ── set_var / get_var ──

    #[test]
    fn set_and_get_var() {
        let mut state = WorkflowState::new("wf");
        state.set_var("key1", serde_json::Value::String("val1".into()));
        assert_eq!(
            state.get_var("key1").unwrap().as_str().unwrap(),
            "val1"
        );
    }

    #[test]
    fn get_var_missing() {
        let state = WorkflowState::new("wf");
        assert!(state.get_var("nonexistent").is_none());
    }

    #[test]
    fn set_var_overwrites() {
        let mut state = WorkflowState::new("wf");
        state.set_var("x", serde_json::Value::String("a".into()));
        state.set_var("x", serde_json::Value::String("b".into()));
        assert_eq!(state.get_var("x").unwrap().as_str().unwrap(), "b");
    }

    #[test]
    fn set_var_updates_timestamp() {
        let mut state = WorkflowState::new("wf");
        let before = state.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        state.set_var("x", serde_json::Value::Bool(true));
        assert!(state.updated_at > before);
    }

    // ── resolve_template ──

    #[test]
    fn resolve_basic_variable() {
        let mut state = WorkflowState::new("wf");
        state.set_var("name", serde_json::Value::String("Alice".into()));
        let result = state.resolve_template("Hello, {{name}}!");
        assert_eq!(result, "Hello, Alice!");
    }

    #[test]
    fn resolve_variable_with_spaces() {
        let mut state = WorkflowState::new("wf");
        state.set_var("name", serde_json::Value::String("Bob".into()));
        let result = state.resolve_template("Hello, {{ name }}!");
        assert_eq!(result, "Hello, Bob!");
    }

    #[test]
    fn resolve_multiple_variables() {
        let mut state = WorkflowState::new("wf");
        state.set_var("first", serde_json::Value::String("John".into()));
        state.set_var("last", serde_json::Value::String("Doe".into()));
        let result = state.resolve_template("{{first}} {{last}}");
        assert_eq!(result, "John Doe");
    }

    #[test]
    fn resolve_numeric_variable() {
        let mut state = WorkflowState::new("wf");
        state.set_var("count", serde_json::Value::Number(serde_json::Number::from(42)));
        let result = state.resolve_template("Count: {{count}}");
        assert_eq!(result, "Count: 42");
    }

    #[test]
    fn resolve_agent_output() {
        let mut state = WorkflowState::new("wf");
        state.update_agent(
            "planner",
            AgentState {
                agent_id: "planner".into(),
                role: "architect".into(),
                status: AgentStatus::Completed,
                last_message: Some("Design complete".into()),
                updated_at: Utc::now(),
            },
        );
        let result = state.resolve_template("Output: {{planner_output}}");
        assert_eq!(result, "Output: Design complete");
    }

    #[test]
    fn resolve_no_placeholders() {
        let state = WorkflowState::new("wf");
        let result = state.resolve_template("plain text");
        assert_eq!(result, "plain text");
    }

    #[test]
    fn resolve_variable_priority_over_agent_output() {
        let mut state = WorkflowState::new("wf");
        state.set_var(
            "coder_output",
            serde_json::Value::String("from vars".into()),
        );
        state.update_agent(
            "coder",
            AgentState {
                agent_id: "coder".into(),
                role: "dev".into(),
                status: AgentStatus::Completed,
                last_message: Some("from agent".into()),
                updated_at: Utc::now(),
            },
        );
        let result = state.resolve_template("{{coder_output}}");
        assert_eq!(result, "from vars");
    }

    // ── record_stage ──

    #[test]
    fn record_stage_appends_and_increments() {
        let mut state = WorkflowState::new("wf");
        state.record_stage("plan", vec![], None);
        assert_eq!(state.stage_results.len(), 1);
        assert_eq!(state.stage_results[0].stage_name, "plan");
        assert_eq!(state.current_stage, 1);

        state.record_stage("build", vec![], None);
        assert_eq!(state.stage_results.len(), 2);
        assert_eq!(state.current_stage, 2);
    }

    #[test]
    fn record_stage_with_outputs() {
        let mut state = WorkflowState::new("wf");
        let output = StageOutput {
            path: "test.txt".into(),
            content: Some("hello".into()),
            agent_id: Some("agent1".into()),
            summary: None,
            files_changed: vec![],
        };
        state.record_stage("s1", vec![output], None);
        assert_eq!(state.stage_results[0].outputs.len(), 1);
        assert_eq!(state.stage_results[0].outputs[0].path, "test.txt");
    }

    // ── update_agent ──

    #[test]
    fn update_agent_inserts_and_updates() {
        let mut state = WorkflowState::new("wf");
        let agent = AgentState {
            agent_id: "a1".into(),
            role: "dev".into(),
            status: AgentStatus::Running,
            last_message: None,
            updated_at: Utc::now(),
        };
        state.update_agent("a1", agent.clone());
        assert_eq!(state.agent_states.len(), 1);
        assert_eq!(state.agent_states["a1"].status, AgentStatus::Running);

        let completed = AgentState {
            status: AgentStatus::Completed,
            ..agent
        };
        state.update_agent("a1", completed);
        assert_eq!(state.agent_states["a1"].status, AgentStatus::Completed);
    }

    // ── status transitions ──

    #[test]
    fn start_sets_running() {
        let mut state = WorkflowState::new("wf");
        state.start();
        assert_eq!(state.status, WorkflowStatus::Running);
    }

    #[test]
    fn complete_sets_finished() {
        let mut state = WorkflowState::new("wf");
        state.complete();
        assert_eq!(state.status, WorkflowStatus::Completed);
        assert!(state.finished_at.is_some());
    }

    #[test]
    fn fail_sets_error() {
        let mut state = WorkflowState::new("wf");
        state.fail("something went wrong".into());
        assert_eq!(state.status, WorkflowStatus::Failed);
        assert_eq!(state.error, Some("something went wrong".into()));
        assert!(state.finished_at.is_some());
    }

    #[test]
    fn cancel_sets_cancelled() {
        let mut state = WorkflowState::new("wf");
        state.cancel();
        assert_eq!(state.status, WorkflowStatus::Cancelled);
        assert!(state.finished_at.is_some());
    }

    // ── should_stop ──

    #[test]
    fn should_stop_false_for_pending() {
        let state = WorkflowState::new("wf");
        assert!(!state.should_stop());
    }

    #[test]
    fn should_stop_false_for_running() {
        let mut state = WorkflowState::new("wf");
        state.start();
        assert!(!state.should_stop());
    }

    #[test]
    fn should_stop_true_for_completed() {
        let mut state = WorkflowState::new("wf");
        state.complete();
        assert!(state.should_stop());
    }

    #[test]
    fn should_stop_true_for_failed() {
        let mut state = WorkflowState::new("wf");
        state.fail("err".into());
        assert!(state.should_stop());
    }

    #[test]
    fn should_stop_true_for_cancelled() {
        let mut state = WorkflowState::new("wf");
        state.cancel();
        assert!(state.should_stop());
    }

    // ── WorkflowStatus Display ──

    #[test]
    fn status_display() {
        assert_eq!(WorkflowStatus::Pending.to_string(), "pending");
        assert_eq!(WorkflowStatus::Running.to_string(), "running");
        assert_eq!(WorkflowStatus::Completed.to_string(), "completed");
        assert_eq!(WorkflowStatus::Failed.to_string(), "failed");
        assert_eq!(WorkflowStatus::Cancelled.to_string(), "cancelled");
    }
}

/// 智能体执行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    /// 空闲
    Idle,
    /// 运行中
    Running,
    /// 等待中
    Waiting,
    /// 已完成
    Completed,
    /// 失败
    Failed,
}
