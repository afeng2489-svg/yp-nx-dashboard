//! Task queue implementation

use crate::scheduler::task::{ScheduledTask, TaskStatus};
use std::collections::VecDeque;
use std::sync::Arc;
use parking_lot::RwLock;

/// Thread-safe task queue
pub struct TaskQueue {
    /// Pending tasks waiting to be executed
    pending: RwLock<VecDeque<ScheduledTask>>,
    /// Tasks currently being executed
    running: RwLock<VecDeque<ScheduledTask>>,
    /// Completed or failed tasks (for history)
    completed: RwLock<VecDeque<ScheduledTask>>,
    /// Maximum history size
    max_history: usize,
}

impl TaskQueue {
    /// Create a new task queue
    pub fn new() -> Self {
        Self {
            pending: RwLock::new(VecDeque::new()),
            running: RwLock::new(VecDeque::new()),
            completed: RwLock::new(VecDeque::new()),
            max_history: 1000,
        }
    }

    /// Create with custom max history size
    pub fn with_max_history(max_history: usize) -> Self {
        Self {
            pending: RwLock::new(VecDeque::new()),
            running: RwLock::new(VecDeque::new()),
            completed: RwLock::new(VecDeque::new()),
            max_history,
        }
    }

    /// Enqueue a new task
    pub fn enqueue(&self, task: ScheduledTask) {
        let mut pending = self.pending.write();
        pending.push_back(task);
    }

    /// Dequeue the next task that is ready to execute
    pub fn dequeue(&self) -> Option<ScheduledTask> {
        let now = chrono::Utc::now();

        let mut pending = self.pending.write();
        let index = pending.iter().position(|t| t.execute_at <= now && t.status == TaskStatus::Pending)?;

        let task = pending.remove(index)?;

        drop(pending);

        let mut running = self.running.write();
        running.push_back(task.clone());

        Some(task)
    }

    /// Get a task by ID
    pub fn get(&self, id: &str) -> Option<ScheduledTask> {
        // Check pending
        let pending = self.pending.read();
        if let Some(task) = pending.iter().find(|t| t.id == id) {
            return Some(task.clone());
        }
        drop(pending);

        // Check running
        let running = self.running.read();
        if let Some(task) = running.iter().find(|t| t.id == id) {
            return Some(task.clone());
        }
        drop(running);

        // Check completed
        let completed = self.completed.read();
        completed.iter().find(|t| t.id == id).cloned()
    }

    /// Get all pending tasks
    pub fn get_pending(&self) -> Vec<ScheduledTask> {
        let pending = self.pending.read();
        pending.iter().cloned().collect()
    }

    /// Get all running tasks
    pub fn get_running(&self) -> Vec<ScheduledTask> {
        let running = self.running.read();
        running.iter().cloned().collect()
    }

    /// Get task count by status
    pub fn count_by_status(&self) -> (usize, usize, usize, usize, usize) {
        let pending = self.pending.read();
        let running = self.running.read();
        let completed = self.completed.read();

        let mut pending_count = 0;
        let mut running_count = 0;
        let mut completed_count = 0;
        let mut failed_count = 0;
        let mut cancelled_count = 0;

        for task in pending.iter() {
            match task.status {
                TaskStatus::Pending => pending_count += 1,
                _ => {}
            }
        }

        for task in running.iter() {
            match task.status {
                TaskStatus::Running => running_count += 1,
                _ => {}
            }
        }

        for task in completed.iter() {
            match task.status {
                TaskStatus::Completed => completed_count += 1,
                TaskStatus::Failed => failed_count += 1,
                TaskStatus::Cancelled => cancelled_count += 1,
                _ => {}
            }
        }

        (pending_count, running_count, completed_count, failed_count, cancelled_count)
    }

    /// Update a task's status
    pub fn update_task(&self, task: ScheduledTask) {
        let task_id = task.id.clone();
        let task_status = task.status.clone();

        // Check if in running queue
        {
            let mut running = self.running.write();
            if let Some(pos) = running.iter().position(|t| t.id == task_id) {
                if task_status != TaskStatus::Running {
                    let removed = running.remove(pos).unwrap();
                    drop(running);
                    self.add_to_completed(removed);
                    return;
                } else {
                    // Update in place
                    running[pos] = task;
                    return;
                }
            }
        }

        // Check if in pending queue
        {
            let mut pending = self.pending.write();
            if let Some(pos) = pending.iter().position(|t| t.id == task_id) {
                pending[pos] = task;
            }
        }
    }

    /// Remove a task from running and add to completed
    fn add_to_completed(&self, task: ScheduledTask) {
        let mut completed = self.completed.write();
        completed.push_back(task);

        // Trim history if needed
        while completed.len() > self.max_history {
            completed.pop_front();
        }
    }

    /// Cancel a pending task
    pub fn cancel(&self, id: &str) -> bool {
        let mut pending = self.pending.write();
        if let Some(pos) = pending.iter().position(|t| t.id == id) {
            let mut task = pending.remove(pos).unwrap();
            task.mark_cancelled();
            drop(pending);
            self.add_to_completed(task);
            return true;
        }
        false
    }

    /// Remove completed tasks older than the specified duration
    pub fn cleanup_completed(&self, older_than: chrono::Duration) {
        let cutoff = chrono::Utc::now() - older_than;
        let mut completed = self.completed.write();
        completed.retain(|t| t.updated_at > cutoff);
    }

    /// Get total queue size
    pub fn len(&self) -> usize {
        self.pending.read().len() + self.running.read().len() + self.completed.read().len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared task queue pointer
pub type SharedTaskQueue = Arc<TaskQueue>;

/// Create a new shared task queue
pub fn create_task_queue() -> SharedTaskQueue {
    Arc::new(TaskQueue::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::task::TaskType;
    use chrono::Utc;

    #[test]
    fn test_enqueue_dequeue() {
        let queue = TaskQueue::new();
        let task = ScheduledTask::new(
            TaskType::WorkflowExecution,
            serde_json::json!({}),
            Utc::now(),
            3,
        );
        let id = task.id.clone();

        queue.enqueue(task);
        assert_eq!(queue.len(), 1);

        let dequeued = queue.dequeue();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().id, id);
    }

    #[test]
    fn test_get_task() {
        let queue = TaskQueue::new();
        let task = ScheduledTask::new(
            TaskType::CodeReview,
            serde_json::json!({}),
            Utc::now(),
            3,
        );
        let id = task.id.clone();

        queue.enqueue(task);

        let retrieved = queue.get(&id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, id);
    }

    #[test]
    fn test_cancel_task() {
        let queue = TaskQueue::new();
        let task = ScheduledTask::new(
            TaskType::Cleanup,
            serde_json::json!({}),
            Utc::now(),
            3,
        );
        let id = task.id.clone();

        queue.enqueue(task);
        assert!(queue.cancel(&id));

        let cancelled = queue.get(&id);
        assert!(cancelled.is_some());
        assert_eq!(cancelled.unwrap().status, TaskStatus::Cancelled);
    }
}