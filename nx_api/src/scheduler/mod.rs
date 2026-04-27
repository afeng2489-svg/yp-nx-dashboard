//! Scheduler module for background task execution
//!
//! This module provides a task scheduling system for NexusFlow,
//! enabling deferred execution of various task types.

pub mod queue;
pub mod task;
pub mod worker;

pub use queue::{create_task_queue, SharedTaskQueue, TaskQueue};
pub use task::{CreateTaskRequest, ScheduledTask, TaskResponse, TaskStatus, TaskType};
pub use worker::{spawn_workers, TaskResult, TaskWorker};

use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info};

use crate::services::ExecutionService;

/// Scheduler service for managing background tasks
pub struct SchedulerService {
    /// Shared task queue
    queue: SharedTaskQueue,
    /// Shutdown signal sender
    shutdown_tx: broadcast::Sender<()>,
    /// Worker handles
    worker_handles: Arc<std::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>>,
    /// Execution service for workflow tasks
    execution_service: Option<ExecutionService>,
}

impl SchedulerService {
    /// Create a new scheduler service (without spawning workers)
    pub fn new(execution_service: Option<ExecutionService>) -> Self {
        let queue = create_task_queue();
        let (shutdown_tx, _) = broadcast::channel::<()>(1);

        info!("Scheduler service created");

        Self {
            queue,
            shutdown_tx,
            worker_handles: Arc::new(std::sync::Mutex::new(Vec::new())),
            execution_service,
        }
    }

    /// Start workers (must be called from async context)
    pub fn start_workers(&self, num_workers: u32) {
        let _handle = tokio::spawn(spawn_workers(
            Arc::clone(&self.queue),
            num_workers,
            self.shutdown_tx.subscribe(),
            self.execution_service.clone(),
        ));

        info!(num_workers = num_workers, "Scheduler workers started");
    }

    /// Get the shared task queue
    pub fn queue(&self) -> SharedTaskQueue {
        Arc::clone(&self.queue)
    }

    /// Submit a new task for execution
    pub fn submit_task(&self, request: CreateTaskRequest) -> ScheduledTask {
        let execute_at = request
            .delay_seconds
            .map(|seconds| Utc::now() + Duration::seconds(seconds))
            .unwrap_or_else(Utc::now);

        let task = ScheduledTask::new(
            request.task_type,
            request.payload,
            execute_at,
            request.max_retries,
        );

        info!(
            task_id = %task.id,
            task_type = ?task.task_type,
            execute_at = %execute_at,
            "Task submitted"
        );

        self.queue.enqueue(task.clone());
        task
    }

    /// Get a task by ID
    pub fn get_task(&self, id: &str) -> Option<ScheduledTask> {
        self.queue.get(id)
    }

    /// Get all pending tasks
    pub fn get_pending_tasks(&self) -> Vec<ScheduledTask> {
        self.queue.get_pending()
    }

    /// Get all running tasks
    pub fn get_running_tasks(&self) -> Vec<ScheduledTask> {
        self.queue.get_running()
    }

    /// List all tasks (pending, running, recent completed)
    pub fn list_tasks(&self) -> Vec<ScheduledTask> {
        let mut tasks = self.queue.get_pending();
        tasks.extend(self.queue.get_running());
        tasks
    }

    /// Cancel a pending task
    pub fn cancel_task(&self, id: &str) -> bool {
        info!(task_id = %id, "Cancelling task");
        self.queue.cancel(id)
    }

    /// Get queue statistics
    pub fn get_stats(&self) -> QueueStats {
        let (pending, running, completed, failed, cancelled) = self.queue.count_by_status();
        QueueStats {
            pending,
            running,
            completed,
            failed,
            cancelled,
            total: pending + running + completed + failed + cancelled,
        }
    }

    /// Shutdown the scheduler gracefully
    pub async fn shutdown(&self) {
        info!("Shutting down scheduler service");

        // Signal workers to stop
        let _ = self.shutdown_tx.send(());

        // Wait for workers to finish
        let handles: Vec<_> = self.worker_handles.lock().unwrap().drain(..).collect();

        for handle in handles {
            if let Err(e) = handle.await {
                error!(error = %e, "Error waiting for worker");
            }
        }

        info!("Scheduler service shutdown complete");
    }
}

/// Queue statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct QueueStats {
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub total: usize,
}

/// Global scheduler instance (for use in route handlers)
use std::sync::RwLock;

static SCHEDULER: RwLock<Option<SchedulerService>> = RwLock::new(None);

/// Initialize the global scheduler
pub fn init_scheduler(execution_service: Option<ExecutionService>) {
    let mut scheduler = SCHEDULER.write().unwrap();
    *scheduler = Some(SchedulerService::new(execution_service));
}

/// Start the global scheduler workers
pub fn start_scheduler_workers(num_workers: u32) {
    if let Some(scheduler) = SCHEDULER.read().unwrap().as_ref() {
        scheduler.start_workers(num_workers);
    }
}

/// Get the global scheduler instance
pub fn get_scheduler() -> Option<std::sync::RwLockWriteGuard<'static, Option<SchedulerService>>> {
    SCHEDULER.write().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submit_task() {
        // Create a local scheduler for testing
        let queue = create_task_queue();
        let request = CreateTaskRequest {
            task_type: TaskType::WorkflowExecution,
            payload: serde_json::json!({ "workflow_id": "test-123" }),
            delay_seconds: None,
            max_retries: 3,
        };

        let task = ScheduledTask::new(
            request.task_type,
            request.payload,
            Utc::now(),
            request.max_retries,
        );

        queue.enqueue(task.clone());

        let retrieved = queue.get(&task.id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, task.id);
    }
}
