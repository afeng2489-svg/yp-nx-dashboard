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
    pub fn record_stage(&mut self, stage_name: &str, outputs: Vec<StageOutput>) {
        self.stage_results.push(StageResult {
            stage_name: stage_name.to_string(),
            outputs,
            completed_at: Utc::now(),
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

/// 阶段执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub stage_name: String,
    pub outputs: Vec<StageOutput>,
    pub completed_at: DateTime<Utc>,
}

/// 阶段的输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageOutput {
    /// 输出路径
    pub path: String,
    /// 输出内容
    pub content: Option<String>,
    /// 智能体 ID
    pub agent_id: Option<String>,
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
