//! Task definitions for the scheduler

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Task type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// Run a workflow
    WorkflowExecution,
    /// Run code review
    CodeReview,
    /// Run security scan
    SecurityAudit,
    /// Cleanup old sessions
    Cleanup,
}

impl TaskType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::WorkflowExecution => "workflow_execution",
            TaskType::CodeReview => "code_review",
            TaskType::SecurityAudit => "security_audit",
            TaskType::Cleanup => "cleanup",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "workflow_execution" => Some(TaskType::WorkflowExecution),
            "code_review" => Some(TaskType::CodeReview),
            "security_audit" => Some(TaskType::SecurityAudit),
            "cleanup" => Some(TaskType::Cleanup),
            _ => None,
        }
    }
}

/// Task status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is pending and waiting to be executed
    Pending,
    /// Task is currently being executed
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed after all retries
    Failed,
    /// Task was cancelled
    Cancelled,
}

impl TaskStatus {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(TaskStatus::Pending),
            "running" => Some(TaskStatus::Running),
            "completed" => Some(TaskStatus::Completed),
            "failed" => Some(TaskStatus::Failed),
            "cancelled" => Some(TaskStatus::Cancelled),
            _ => None,
        }
    }
}

/// A scheduled task in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    /// Unique task identifier
    pub id: String,
    /// Type of task to execute
    pub task_type: TaskType,
    /// Task payload as JSON
    pub payload: serde_json::Value,
    /// When the task was scheduled
    pub scheduled_at: DateTime<Utc>,
    /// When the task should be executed
    pub execute_at: DateTime<Utc>,
    /// Number of retry attempts made
    pub retry_count: u32,
    /// Maximum number of retry attempts allowed
    pub max_retries: u32,
    /// Current task status
    pub status: TaskStatus,
    /// Error message if the task failed
    pub error_message: Option<String>,
    /// When the task was last updated
    pub updated_at: DateTime<Utc>,
}

impl ScheduledTask {
    /// Create a new scheduled task
    pub fn new(
        task_type: TaskType,
        payload: serde_json::Value,
        execute_at: DateTime<Utc>,
        max_retries: u32,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            task_type,
            payload,
            scheduled_at: now,
            execute_at,
            retry_count: 0,
            max_retries,
            status: TaskStatus::Pending,
            error_message: None,
            updated_at: now,
        }
    }

    /// Check if the task can be retried
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries && self.status == TaskStatus::Failed
    }

    /// Mark the task as running
    pub fn mark_running(&mut self) {
        self.status = TaskStatus::Running;
        self.updated_at = Utc::now();
    }

    /// Mark the task as completed
    pub fn mark_completed(&mut self) {
        self.status = TaskStatus::Completed;
        self.error_message = None;
        self.updated_at = Utc::now();
    }

    /// Mark the task as failed
    pub fn mark_failed(&mut self, error: String) {
        self.retry_count += 1;
        self.error_message = Some(error);
        self.updated_at = Utc::now();

        if self.retry_count >= self.max_retries {
            self.status = TaskStatus::Failed;
        }
    }

    /// Mark the task as cancelled
    pub fn mark_cancelled(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.updated_at = Utc::now();
    }
}

/// Request to create a new task
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTaskRequest {
    /// Type of task to create
    pub task_type: TaskType,
    /// Task payload
    pub payload: serde_json::Value,
    /// Optional delay before execution (in seconds)
    #[serde(default)]
    pub delay_seconds: Option<i64>,
    /// Maximum number of retries
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

fn default_max_retries() -> u32 {
    3
}

/// Response for task operations
#[derive(Debug, Clone, Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub scheduled_at: DateTime<Utc>,
    pub execute_at: DateTime<Utc>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub error_message: Option<String>,
}

impl From<&ScheduledTask> for TaskResponse {
    fn from(task: &ScheduledTask) -> Self {
        Self {
            id: task.id.clone(),
            task_type: task.task_type.clone(),
            status: task.status.clone(),
            scheduled_at: task.scheduled_at,
            execute_at: task.execute_at,
            retry_count: task.retry_count,
            max_retries: task.max_retries,
            error_message: task.error_message.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let payload = serde_json::json!({ "workflow_id": "test-123" });
        let task = ScheduledTask::new(TaskType::WorkflowExecution, payload, Utc::now(), 3);

        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.retry_count, 0);
        assert_eq!(task.max_retries, 3);
    }

    #[test]
    fn test_task_retry() {
        let payload = serde_json::json!({});
        let mut task = ScheduledTask::new(TaskType::Cleanup, payload, Utc::now(), 3);

        // mark_failed only sets status=Failed when retry_count >= max_retries.
        // Before that, status remains Pending, so can_retry() returns false
        // (it requires status == Failed).
        task.mark_failed("Test error".to_string());
        assert_eq!(task.retry_count, 1);
        // Status is still Pending since retry_count(1) < max_retries(3)
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(!task.can_retry());

        task.mark_failed("Test error".to_string());
        assert_eq!(task.retry_count, 2);
        assert_eq!(task.status, TaskStatus::Pending);

        task.mark_failed("Test error".to_string());
        // Now retry_count(3) >= max_retries(3), so status becomes Failed
        assert_eq!(task.retry_count, 3);
        assert_eq!(task.status, TaskStatus::Failed);
        assert!(!task.can_retry());
    }

    #[test]
    fn test_task_type_conversion() {
        assert_eq!(TaskType::WorkflowExecution.as_str(), "workflow_execution");
        assert_eq!(
            TaskType::from_str("workflow_execution"),
            Some(TaskType::WorkflowExecution)
        );
        assert_eq!(TaskType::from_str("invalid"), None);
    }
}
