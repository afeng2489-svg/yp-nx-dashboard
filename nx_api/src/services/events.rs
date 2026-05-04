//! 执行事件定义
//!
//! 所有执行相关的事件类型定义。

use serde::{Deserialize, Serialize};

/// 用户输入选项（pause 时展示给前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowOption {
    pub label: String,
    pub value: String,
}

/// 执行事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExecutionEvent {
    /// 执行开始
    Started {
        execution_id: String,
        workflow_id: String,
    },
    /// 状态更新
    StatusChanged {
        execution_id: String,
        status: ExecutionStatus,
    },
    /// 阶段开始
    StageStarted {
        execution_id: String,
        stage_name: String,
    },
    /// 阶段完成
    StageCompleted {
        execution_id: String,
        stage_name: String,
        output: serde_json::Value,
        quality_gate_result: Option<serde_json::Value>,
    },
    /// 输出行
    Output { execution_id: String, line: String },
    /// 完成
    Completed { execution_id: String },
    /// 失败
    Failed { execution_id: String, error: String },
    /// 工作流暂停，等待用户选择
    WorkflowPaused {
        execution_id: String,
        stage_name: String,
        question: String,
        options: Vec<WorkflowOption>,
    },
    /// 工作流从暂停中恢复
    WorkflowResumed {
        execution_id: String,
        stage_name: String,
        chosen_value: String,
    },
    /// Token/Cost 用量更新
    TokenUsage {
        execution_id: String,
        total_tokens: i64,
        total_cost_usd: f64,
    },
    /// 预算告警（超 80%）
    BudgetWarning {
        execution_id: String,
        current_usd: f64,
        limit_usd: f64,
        percentage: f64,
    },
    /// 预算超限（超 100%，已自动取消）
    BudgetExceeded {
        execution_id: String,
        current_usd: f64,
        limit_usd: f64,
    },
}

/// 执行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    /// 等待中
    Pending,
    /// 运行中
    Running,
    /// 暂停
    Paused,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}
