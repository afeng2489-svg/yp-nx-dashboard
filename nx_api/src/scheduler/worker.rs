//! Worker for processing scheduled tasks

use crate::scheduler::queue::{SharedTaskQueue, TaskQueue};
use crate::scheduler::task::{ScheduledTask, TaskStatus, TaskType};
use crate::services::execution_service::ExecutionService;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{error, info, warn};

/// Task execution result
#[derive(Debug)]
pub enum TaskResult {
    /// Task completed successfully
    Success,
    /// Task failed with error message
    Failure(String),
    /// Task was cancelled
    Cancelled,
}

/// Worker for processing tasks from the queue
pub struct TaskWorker {
    /// Shared task queue
    queue: SharedTaskQueue,
    /// Shutdown signal receiver
    shutdown_rx: broadcast::Receiver<()>,
    /// Worker ID
    worker_id: u32,
    /// Task timeout duration
    task_timeout: Duration,
    /// Polling interval
    poll_interval: Duration,
    /// Execution service (for workflow tasks)
    execution_service: Option<ExecutionService>,
}

impl TaskWorker {
    /// Create a new task worker
    pub fn new(
        queue: SharedTaskQueue,
        shutdown_rx: broadcast::Receiver<()>,
        worker_id: u32,
        execution_service: Option<ExecutionService>,
    ) -> Self {
        Self {
            queue,
            shutdown_rx,
            worker_id,
            task_timeout: Duration::from_secs(300), // 5 minutes default
            poll_interval: Duration::from_millis(500),
            execution_service,
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        queue: SharedTaskQueue,
        shutdown_rx: broadcast::Receiver<()>,
        worker_id: u32,
        task_timeout: Duration,
        poll_interval: Duration,
        execution_service: Option<ExecutionService>,
    ) -> Self {
        Self {
            queue,
            shutdown_rx,
            worker_id,
            task_timeout,
            poll_interval,
            execution_service,
        }
    }

    /// Start the worker loop
    pub async fn run(&mut self) {
        info!(worker_id = self.worker_id, "Task worker started");

        let mut poll_timer = interval(self.poll_interval);
        let mut shutdown = self.shutdown_rx.resubscribe();

        loop {
            tokio::select! {
                // Poll for shutdown signal
                _ = shutdown.recv() => {
                    info!(worker_id = self.worker_id, "Task worker received shutdown signal");
                    break;
                }
                // Poll for tasks
                _ = poll_timer.tick() => {
                    if let Err(e) = self.process_next_task().await {
                        error!(worker_id = self.worker_id, error = %e, "Error processing task");
                    }
                }
            }
        }

        info!(worker_id = self.worker_id, "Task worker stopped");
    }

    /// Process the next task in the queue
    async fn process_next_task(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Try to dequeue a task
        let task = match self.queue.dequeue() {
            Some(t) => t,
            None => return Ok(()), // No tasks available
        };

        info!(
            worker_id = self.worker_id,
            task_id = %task.id,
            task_type = ?task.task_type,
            "Processing task"
        );

        // Execute the task with timeout
        let result = self.execute_task_with_timeout(task.clone()).await;

        // Update task status based on result
        match result {
            Ok(TaskResult::Success) => {
                info!(
                    worker_id = self.worker_id,
                    task_id = %task.id,
                    "Task completed successfully"
                );
                let mut task = task.clone();
                task.mark_completed();
                self.queue.update_task(task);
            }
            Ok(TaskResult::Failure(error_msg)) => {
                warn!(
                    worker_id = self.worker_id,
                    task_id = %task.id,
                    error = %error_msg,
                    "Task failed"
                );
                let mut task = task.clone();
                task.mark_failed(error_msg);
                self.queue.update_task(task);
            }
            Ok(TaskResult::Cancelled) => {
                info!(
                    worker_id = self.worker_id,
                    task_id = %task.id,
                    "Task was cancelled"
                );
                let mut task = task.clone();
                task.mark_cancelled();
                self.queue.update_task(task);
            }
            Err(e) => {
                error!(
                    worker_id = self.worker_id,
                    task_id = %task.id,
                    error = %e,
                    "Task execution error"
                );
                let mut task = task.clone();
                task.mark_failed(e.to_string());
                self.queue.update_task(task);
            }
        }

        Ok(())
    }

    /// Execute a task with timeout
    async fn execute_task_with_timeout(
        &mut self,
        mut task: ScheduledTask,
    ) -> Result<TaskResult, Box<dyn std::error::Error + Send + Sync>> {
        // Mark task as running
        task.mark_running();
        self.queue.update_task(task.clone());

        tokio::select! {
            result = self.execute_task(&task) => {
                result
            }
            _ = tokio::time::sleep(self.task_timeout) => {
                Ok(TaskResult::Failure(format!(
                    "Task timed out after {:?}",
                    self.task_timeout
                )))
            }
        }
    }

    /// Execute a single task
    async fn execute_task(
        &self,
        task: &ScheduledTask,
    ) -> Result<TaskResult, Box<dyn std::error::Error + Send + Sync>> {
        match task.task_type {
            TaskType::WorkflowExecution => self.execute_workflow(task).await,
            TaskType::CodeReview => self.execute_code_review(task).await,
            TaskType::SecurityAudit => self.execute_security_audit(task).await,
            TaskType::Cleanup => self.execute_cleanup(task).await,
        }
    }

    /// Execute a workflow task
    async fn execute_workflow(
        &self,
        task: &ScheduledTask,
    ) -> Result<TaskResult, Box<dyn std::error::Error + Send + Sync>> {
        let workflow_id = task
            .payload
            .get("workflow_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing workflow_id in payload")?;

        let workflow_yaml = task
            .payload
            .get("workflow_yaml")
            .and_then(|v| v.as_str())
            .ok_or("Missing workflow_yaml in payload")?;

        let variables = task
            .payload
            .get("variables")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        info!(
            worker_id = self.worker_id,
            task_id = %task.id,
            workflow_id = %workflow_id,
            "Executing workflow"
        );

        let exec_service = self.execution_service.as_ref()
            .ok_or("ExecutionService not configured for this worker")?;

        exec_service
            .execute_workflow(
                workflow_id.to_string(),
                workflow_yaml,
                variables,
                None,
                None,
            )
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;

        Ok(TaskResult::Success)
    }

    /// Execute a code review task
    async fn execute_code_review(
        &self,
        task: &ScheduledTask,
    ) -> Result<TaskResult, Box<dyn std::error::Error + Send + Sync>> {
        let repo_url = task
            .payload
            .get("repo_url")
            .and_then(|v| v.as_str())
            .ok_or("Missing repo_url in payload")?;

        info!(
            worker_id = self.worker_id,
            task_id = %task.id,
            repo_url = %repo_url,
            "Executing code review"
        );

        // TODO: Integrate with actual code review service
        tokio::time::sleep(Duration::from_secs(2)).await;

        Ok(TaskResult::Success)
    }

    /// Execute a security audit task
    async fn execute_security_audit(
        &self,
        task: &ScheduledTask,
    ) -> Result<TaskResult, Box<dyn std::error::Error + Send + Sync>> {
        let target = task
            .payload
            .get("target")
            .and_then(|v| v.as_str())
            .ok_or("Missing target in payload")?;

        info!(
            worker_id = self.worker_id,
            task_id = %task.id,
            target = %target,
            "Executing security audit"
        );

        // TODO: Integrate with actual security audit service
        tokio::time::sleep(Duration::from_secs(2)).await;

        Ok(TaskResult::Success)
    }

    /// Execute a cleanup task
    async fn execute_cleanup(
        &self,
        task: &ScheduledTask,
    ) -> Result<TaskResult, Box<dyn std::error::Error + Send + Sync>> {
        let older_than_days = task
            .payload
            .get("older_than_days")
            .and_then(|v| v.as_u64())
            .unwrap_or(7) as i64;

        info!(
            worker_id = self.worker_id,
            task_id = %task.id,
            older_than_days = %older_than_days,
            "Executing cleanup"
        );

        // Clean up completed tasks older than the specified duration
        self.queue.cleanup_completed(chrono::Duration::days(older_than_days));

        Ok(TaskResult::Success)
    }
}

/// Spawn multiple worker tasks
pub async fn spawn_workers(
    queue: SharedTaskQueue,
    num_workers: u32,
    shutdown_rx: broadcast::Receiver<()>,
    execution_service: Option<ExecutionService>,
) -> Vec<tokio::task::JoinHandle<()>> {
    let mut handles = Vec::new();

    for worker_id in 0..num_workers {
        let queue = Arc::clone(&queue);
        let shutdown = shutdown_rx.resubscribe();
        let exec_service = execution_service.clone();

        let handle = tokio::spawn(async move {
            let mut worker = TaskWorker::new(queue, shutdown, worker_id, exec_service);
            worker.run().await;
        });

        handles.push(handle);
    }

    info!(num_workers = num_workers, "Spawned task workers");
    handles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::queue::create_task_queue;
    use crate::scheduler::task::TaskType;
    use chrono::Utc;

    #[tokio::test]
    async fn test_task_execution() {
        let queue = create_task_queue();
        let (_shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

        // Enqueue a cleanup task
        let task = ScheduledTask::new(
            TaskType::Cleanup,
            serde_json::json!({ "older_than_days": 7 }),
            Utc::now(),
            3,
        );
        queue.enqueue(task);

        // Create worker
        let mut worker = TaskWorker::new(queue.clone(), shutdown_rx, 0, None);

        // Process one task
        worker.process_next_task().await.unwrap();

        // Verify task was completed
        let completed = queue.get_running();
        assert_eq!(completed.len(), 0);
    }
}
