//! Issue 数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Issue 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IssueStatus {
    #[default]
    Discovered,
    Planned,
    Queued,
    Executing,
    Completed,
    Failed,
}

impl IssueStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            IssueStatus::Discovered => "discovered",
            IssueStatus::Planned => "planned",
            IssueStatus::Queued => "queued",
            IssueStatus::Executing => "executing",
            IssueStatus::Completed => "completed",
            IssueStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "planned" => IssueStatus::Planned,
            "queued" => IssueStatus::Queued,
            "executing" => IssueStatus::Executing,
            "completed" => IssueStatus::Completed,
            "failed" => IssueStatus::Failed,
            _ => IssueStatus::Discovered,
        }
    }
}

/// Issue 优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IssuePriority {
    Critical,
    High,
    #[default]
    Medium,
    Low,
}

impl IssuePriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            IssuePriority::Critical => "critical",
            IssuePriority::High => "high",
            IssuePriority::Medium => "medium",
            IssuePriority::Low => "low",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "critical" => IssuePriority::Critical,
            "high" => IssuePriority::High,
            "low" => IssuePriority::Low,
            _ => IssuePriority::Medium,
        }
    }
}

/// Issue 实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: IssueStatus,
    pub priority: IssuePriority,
    /// 发现此 issue 的视角（如 "bug", "security", "performance"）
    pub perspectives: Vec<String>,
    /// plan 阶段生成的解决方案
    pub solution: Option<String>,
    /// 依赖的其他 issue ID（用于 DAG 排序）
    pub depends_on: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建 Issue 请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateIssueRequest {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub priority: IssuePriority,
    #[serde(default)]
    pub perspectives: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

/// 更新 Issue 请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateIssueRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<IssueStatus>,
    pub priority: Option<IssuePriority>,
    pub solution: Option<String>,
    pub perspectives: Option<Vec<String>>,
    pub depends_on: Option<Vec<String>>,
}

/// Issue 列表过滤器
#[derive(Debug, Clone, Deserialize, Default)]
pub struct IssueFilter {
    pub status: Option<String>,
    pub priority: Option<String>,
}
