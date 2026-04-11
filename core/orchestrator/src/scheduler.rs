//! Task Scheduler - 任务调度器
//!
//! 增强版任务调度器，支持：
//! - SQLite 持久化存储
//! - 任务重试与指数退避
//! - Cron 风格调度
//! - 优先级队列
//! - 任务超时处理

use crate::error::OrchestratorError;
use crate::executor::{ExecutionResult, WorkflowDefinition};
use crate::message_bus::{Channel, MessageBus, MessagePayload};
use crate::team::TeamId;
use crate::CliManager;
use crate::TeamManager;
use crate::WorkflowExecutor;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use parking_lot::RwLock;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use std::time::Duration as StdDuration;
use thiserror::Error;
use uuid::Uuid;

/// 调度器错误类型
#[derive(Error, Debug)]
pub enum SchedulerError {
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("任务不存在: {0}")]
    TaskNotFound(Uuid),

    #[error("任务已存在: {0}")]
    TaskAlreadyExists(Uuid),

    #[error("调度错误: {0}")]
    ScheduleError(String),

    #[error("超时: {0}")]
    Timeout(String),
}

/// Cron 表达式解析器
#[derive(Debug, Clone)]
pub struct CronSchedule {
    /// 分钟 (0-59)
    pub minute: Vec<u8>,
    /// 小时 (0-23)
    pub hour: Vec<u8>,
    /// 日期 (1-31)
    pub day_of_month: Vec<u8>,
    /// 月份 (1-12)
    pub month: Vec<u8>,
    /// 星期 (0-6, 0 = 周日)
    pub day_of_week: Vec<u8>,
}

impl CronSchedule {
    /// 解析 cron 表达式
    /// 格式: "分 时 日 月 周"
    /// 支持: * , - /
    pub fn parse(expr: &str) -> Result<Self, SchedulerError> {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(SchedulerError::ScheduleError(
                format!("无效的 cron 表达式: {}", expr)
            ));
        }

        Ok(Self {
            minute: Self::parse_field(parts[0], 0, 59)?,
            hour: Self::parse_field(parts[1], 0, 23)?,
            day_of_month: Self::parse_field(parts[2], 1, 31)?,
            month: Self::parse_field(parts[3], 1, 12)?,
            day_of_week: Self::parse_field(parts[4], 0, 6)?,
        })
    }

    fn parse_field(field: &str, min: u8, max: u8) -> Result<Vec<u8>, SchedulerError> {
        if field == "*" {
            return Ok((min..=max).collect());
        }

        let mut values = Vec::new();
        for part in field.split(',') {
            if part.contains('-') {
                let range: Vec<&str> = part.split('-').collect();
                if range.len() != 2 {
                    return Err(SchedulerError::ScheduleError(
                        format!("无效的范围: {}", part)
                    ));
                }
                let start: u8 = range[0].parse().map_err(|_| {
                    SchedulerError::ScheduleError(format!("无效数字: {}", range[0]))
                })?;
                let end: u8 = range[1].parse().map_err(|_| {
                    SchedulerError::ScheduleError(format!("无效数字: {}", range[1]))
                })?;
                values.extend(start..=end);
            } else if part.contains('/') {
                let step_parts: Vec<&str> = part.split('/').collect();
                if step_parts.len() != 2 {
                    return Err(SchedulerError::ScheduleError(
                        format!("无效的步长: {}", part)
                    ));
                }
                let step: u8 = step_parts[1].parse().map_err(|_| {
                    SchedulerError::ScheduleError(format!("无效步长: {}", step_parts[1]))
                })?;
                let start = if step_parts[0] == "*" { min } else {
                    step_parts[0].parse().unwrap_or(min)
                };
                values.extend((start..=max).step_by(step as usize));
            } else {
                let value: u8 = part.parse().map_err(|_| {
                    SchedulerError::ScheduleError(format!("无效数字: {}", part))
                })?;
                if value < min || value > max {
                    return Err(SchedulerError::ScheduleError(
                        format!("数字 {} 超出范围 [{}-{}]", value, min, max)
                    ));
                }
                values.push(value);
            }
        }

        values.sort();
        values.dedup();
        Ok(values)
    }

    /// 计算下次执行时间
    pub fn next_run(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let mut current = after;

        // 最多尝试 1 年内的调度
        for _ in 0..365 * 24 * 60 {
            current = current + Duration::minutes(1);

            let minute = current.minute() as u8;
            let hour = current.hour() as u8;
            let day_of_month = current.day() as u8;
            let month = current.month() as u8;
            let day_of_week = current.weekday().num_days_from_sunday() as u8;

            if self.minute.contains(&minute)
                && self.hour.contains(&hour)
                && self.day_of_month.contains(&day_of_month)
                && self.month.contains(&month)
                && self.day_of_week.contains(&day_of_week)
            {
                return Some(current);
            }
        }

        None
    }
}

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 初始退避时间（秒）
    pub initial_backoff_secs: u64,
    /// 最大退避时间（秒）
    pub max_backoff_secs: u64,
    /// 退避乘数
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_secs: 1,
            max_backoff_secs: 300,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// 计算第 n 次重试的退避时间
    pub fn backoff_duration(&self, attempt: u32) -> StdDuration {
        let backoff = self.initial_backoff_secs as f64
            * self.backoff_multiplier.powi(attempt as i32);
        let backoff = backoff.min(self.max_backoff_secs as f64);
        StdDuration::from_secs(backoff as u64)
    }
}

/// 任务优先级比较器（用于 BinaryHeap）
#[derive(Debug, Clone)]
struct PriorityTask {
    task_id: Uuid,
    priority: TaskPriority,
    scheduled_at: DateTime<Utc>,
}

impl PartialEq for PriorityTask {
    fn eq(&self, other: &Self) -> bool {
        self.task_id == other.task_id
    }
}

impl Eq for PriorityTask {}

impl PartialOrd for PriorityTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // BinaryHeap 是最大堆，所以我们要反转比较
        match self.priority.cmp(&other.priority) {
            std::cmp::Ordering::Equal => {
                self.scheduled_at.cmp(&other.scheduled_at)
            }
            other => other,
        }
    }
}

/// 定时任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    /// 任务 ID
    pub id: Uuid,
    /// 工作流定义
    pub workflow: WorkflowDefinition,
    /// 团队 ID
    pub team_id: TeamId,
    /// 变量
    pub variables: HashMap<String, serde_json::Value>,
    /// Cron 表达式
    pub cron_expr: String,
    /// 解析后的调度
    #[serde(skip)]
    pub schedule: Option<CronSchedule>,
    /// 下次执行时间
    pub next_run: DateTime<Utc>,
    /// 是否启用
    pub enabled: bool,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 队列任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedTask {
    /// 任务 ID
    pub id: Uuid,
    /// 工作流定义
    pub workflow: WorkflowDefinition,
    /// 团队 ID
    pub team_id: TeamId,
    /// 变量
    pub variables: HashMap<String, serde_json::Value>,
    /// 优先级
    pub priority: TaskPriority,
    /// 状态
    pub status: QueueStatus,
    /// 重试次数
    pub retry_count: u32,
    /// 重试配置
    pub retry_config: RetryConfig,
    /// 任务超时（秒）
    pub timeout_secs: Option<u64>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 开始执行时间
    pub started_at: Option<DateTime<Utc>>,
    /// 完成时间
    pub finished_at: Option<DateTime<Utc>>,
    /// 结果
    pub result: Option<ExecutionResult>,
    /// 错误信息（如果有）
    pub error: Option<String>,
}

/// 队列状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueStatus {
    /// 等待中
    Queued,
    /// 延迟中
    Delayed,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
    /// 已超时
    TimedOut,
}

impl std::fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueStatus::Queued => write!(f, "queued"),
            QueueStatus::Delayed => write!(f, "delayed"),
            QueueStatus::Running => write!(f, "running"),
            QueueStatus::Completed => write!(f, "completed"),
            QueueStatus::Failed => write!(f, "failed"),
            QueueStatus::Cancelled => write!(f, "cancelled"),
            QueueStatus::TimedOut => write!(f, "timed_out"),
        }
    }
}

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl std::fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskPriority::Low => write!(f, "low"),
            TaskPriority::Normal => write!(f, "normal"),
            TaskPriority::High => write!(f, "high"),
            TaskPriority::Critical => write!(f, "critical"),
        }
    }
}

/// 调度器统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchedulerStats {
    /// 队列中任务数
    pub queued_count: usize,
    /// 延迟任务数
    pub delayed_count: usize,
    /// 运行中任务数
    pub running_count: usize,
    /// 已完成任务数
    pub completed_count: usize,
    /// 失败任务数
    pub failed_count: usize,
    /// 定时任务数
    pub scheduled_jobs_count: usize,
}

/// 增强版任务调度器
pub struct TaskScheduler {
    /// 工作流执行器
    executor: Arc<WorkflowExecutor>,
    /// 任务队列（优先级堆）
    queue: RwLock<BinaryHeap<PriorityTask>>,
    /// 延迟队列（按执行时间排序）
    delayed: RwLock<Vec<PriorityTask>>,
    /// 任务存储
    tasks: RwLock<HashMap<Uuid, QueuedTask>>,
    /// 定时任务
    scheduled_jobs: RwLock<HashMap<Uuid, ScheduledJob>>,
    /// 数据库连接
    db: RwLock<Option<Connection>>,
    /// 最大并发任务数
    max_concurrent: usize,
    /// 运行中的任务
    running: RwLock<HashMap<Uuid, tokio::task::JoinHandle<()>>>,
    /// 调度器运行标志
    running_flag: RwLock<bool>,
}

impl TaskScheduler {
    /// 创建新的任务调度器
    pub fn new(
        cli_manager: Arc<CliManager>,
        team_manager: Arc<TeamManager>,
        message_bus: Arc<MessageBus>,
        max_concurrent: usize,
    ) -> Self {
        let executor = Arc::new(WorkflowExecutor::new(
            cli_manager,
            team_manager,
            message_bus,
        ));

        Self {
            executor,
            queue: RwLock::new(BinaryHeap::new()),
            delayed: RwLock::new(Vec::new()),
            tasks: RwLock::new(HashMap::new()),
            scheduled_jobs: RwLock::new(HashMap::new()),
            db: RwLock::new(None),
            max_concurrent,
            running: RwLock::new(HashMap::new()),
            running_flag: RwLock::new(false),
        }
    }

    /// 初始化数据库
    pub fn init_database(&self, db_path: &str) -> Result<(), SchedulerError> {
        let conn = Connection::open(db_path)?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                workflow TEXT NOT NULL,
                team_id TEXT NOT NULL,
                variables TEXT NOT NULL,
                priority TEXT NOT NULL,
                status TEXT NOT NULL,
                retry_count INTEGER NOT NULL DEFAULT 0,
                retry_config TEXT NOT NULL,
                timeout_secs INTEGER,
                created_at TEXT NOT NULL,
                started_at TEXT,
                finished_at TEXT,
                result TEXT,
                error TEXT
            );

            CREATE TABLE IF NOT EXISTS scheduled_jobs (
                id TEXT PRIMARY KEY,
                workflow TEXT NOT NULL,
                team_id TEXT NOT NULL,
                variables TEXT NOT NULL,
                cron_expr TEXT NOT NULL,
                next_run TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
            CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority);
            CREATE INDEX IF NOT EXISTS idx_scheduled_jobs_next_run ON scheduled_jobs(next_run);
            "
        )?;

        // 加载已有任务
        self.load_tasks_from_db(&conn)?;

        *self.db.write() = Some(conn);

        Ok(())
    }

    /// 从数据库加载任务
    fn load_tasks_from_db(&self, conn: &Connection) -> Result<(), SchedulerError> {
        let mut stmt = conn.prepare(
            "SELECT id, workflow, team_id, variables, priority, status, retry_count,
                    retry_config, timeout_secs, created_at, started_at, finished_at,
                    result, error FROM tasks WHERE status IN ('queued', 'delayed')"
        )?;

        let tasks = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let workflow_json: String = row.get(1)?;
            let team_id_str: String = row.get(2)?;
            let variables_json: String = row.get(3)?;
            let priority_str: String = row.get(4)?;
            let status_str: String = row.get(5)?;
            let retry_count: u32 = row.get(6)?;
            let retry_config_json: String = row.get(7)?;
            let timeout_secs: Option<u64> = row.get(8)?;
            let created_at_str: String = row.get(9)?;
            let started_at_str: Option<String> = row.get(10)?;
            let finished_at_str: Option<String> = row.get(11)?;
            let result_json: Option<String> = row.get(12)?;
            let error_str: Option<String> = row.get(13)?;

            Ok((
                id_str,
                workflow_json,
                team_id_str,
                variables_json,
                priority_str,
                status_str,
                retry_count,
                retry_config_json,
                timeout_secs,
                created_at_str,
                started_at_str,
                finished_at_str,
                result_json,
                error_str,
            ))
        })?;

        for task_result in tasks {
            let (
                id_str,
                workflow_json,
                team_id_str,
                _variables_json,
                priority_str,
                status_str,
                _retry_count,
                _retry_config_json,
                _timeout_secs,
                _created_at_str,
                _started_at_str,
                _finished_at_str,
                _result_json,
                _error_str,
            ) = task_result?;

            let id = Uuid::parse_str(&id_str).ok();
            let priority = match priority_str.as_str() {
                "low" => TaskPriority::Low,
                "high" => TaskPriority::High,
                "critical" => TaskPriority::Critical,
                _ => TaskPriority::Normal,
            };
            let status = match status_str.as_str() {
                "running" => QueueStatus::Running,
                "completed" => QueueStatus::Completed,
                "failed" => QueueStatus::Failed,
                "cancelled" => QueueStatus::Cancelled,
                "timed_out" => QueueStatus::TimedOut,
                _ => QueueStatus::Queued,
            };

            if let Some(task_id) = id {
                if status == QueueStatus::Queued {
                    let mut queue = self.queue.write();
                    queue.push(PriorityTask {
                        task_id,
                        priority,
                        scheduled_at: Utc::now(),
                    });
                }
            }
        }

        Ok(())
    }

    /// 入队任务
    pub fn enqueue(
        &self,
        workflow: WorkflowDefinition,
        team_id: TeamId,
        variables: HashMap<String, serde_json::Value>,
        priority: TaskPriority,
    ) -> Uuid {
        let task = QueuedTask {
            id: Uuid::new_v4(),
            workflow,
            team_id,
            variables,
            priority,
            status: QueueStatus::Queued,
            retry_count: 0,
            retry_config: RetryConfig::default(),
            timeout_secs: None,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
            result: None,
            error: None,
        };

        let task_id = task.id;

        // 持久化到数据库（在移动之前）
        self.save_task_to_db(&task);

        // 保存到内存
        {
            let mut tasks = self.tasks.write();
            tasks.insert(task_id, task);
        }

        // 添加到优先级队列
        {
            let mut queue = self.queue.write();
            queue.push(PriorityTask {
                task_id,
                priority,
                scheduled_at: Utc::now(),
            });
        }

        task_id
    }

    /// 延迟执行任务（用于重试）
    pub fn schedule_delayed(
        &self,
        task_id: Uuid,
        delay: StdDuration,
    ) -> Result<(), SchedulerError> {
        let (priority, scheduled_at) = {
            let tasks = self.tasks.read();
            match tasks.get(&task_id) {
                Some(task) => (task.priority, Utc::now() + Duration::from_std(delay).unwrap()),
                None => return Err(SchedulerError::TaskNotFound(task_id)),
            }
        };

        // 更新任务状态为延迟
        {
            let mut tasks = self.tasks.write();
            if let Some(task) = tasks.get_mut(&task_id) {
                task.status = QueueStatus::Delayed;
            }
        }

        // 添加到延迟队列
        {
            let mut delayed = self.delayed.write();
            delayed.push(PriorityTask {
                task_id,
                priority,
                scheduled_at,
            });
        }

        // 更新数据库
        self.update_task_status_in_db(task_id, QueueStatus::Delayed);

        Ok(())
    }

    /// 添加定时任务
    pub fn add_scheduled_job(
        &self,
        workflow: WorkflowDefinition,
        team_id: TeamId,
        variables: HashMap<String, serde_json::Value>,
        cron_expr: &str,
    ) -> Result<Uuid, SchedulerError> {
        let schedule = CronSchedule::parse(cron_expr)?;
        let next_run = schedule.next_run(Utc::now())
            .ok_or_else(|| SchedulerError::ScheduleError("无法计算下次执行时间".to_string()))?;

        let job = ScheduledJob {
            id: Uuid::new_v4(),
            workflow,
            team_id,
            variables,
            cron_expr: cron_expr.to_string(),
            schedule: Some(schedule),
            next_run,
            enabled: true,
            created_at: Utc::now(),
        };

        let job_id = job.id;

        // 持久化到数据库（在移动之前）
        self.save_scheduled_job_to_db(&job)?;

        // 保存到内存
        {
            let mut jobs = self.scheduled_jobs.write();
            jobs.insert(job_id, job);
        }

        Ok(job_id)
    }

    /// 启用/禁用定时任务
    pub fn set_scheduled_job_enabled(&self, job_id: Uuid, enabled: bool) -> bool {
        let mut jobs = self.scheduled_jobs.write();
        if let Some(job) = jobs.get_mut(&job_id) {
            job.enabled = enabled;
            self.update_scheduled_job_enabled_in_db(job_id, enabled);
            return true;
        }
        false
    }

    /// 移除定时任务
    pub fn remove_scheduled_job(&self, job_id: Uuid) -> bool {
        let mut jobs = self.scheduled_jobs.write();
        if jobs.remove(&job_id).is_some() {
            self.delete_scheduled_job_from_db(job_id);
            return true;
        }
        false
    }

    /// 取消任务
    pub fn cancel_task(&self, task_id: Uuid) -> bool {
        let mut tasks = self.tasks.write();
        if let Some(task) = tasks.get_mut(&task_id) {
            if task.status == QueueStatus::Queued || task.status == QueueStatus::Delayed {
                task.status = QueueStatus::Cancelled;
                task.finished_at = Some(Utc::now());
                self.update_task_status_in_db(task_id, QueueStatus::Cancelled);
                return true;
            }
        }
        false
    }

    /// 获取任务
    pub fn get_task(&self, task_id: Uuid) -> Option<QueuedTask> {
        let tasks = self.tasks.read();
        tasks.get(&task_id).cloned()
    }

    /// 列出所有任务
    pub fn list_tasks(&self) -> Vec<QueuedTask> {
        let tasks = self.tasks.read();
        tasks.values().cloned().collect()
    }

    /// 获取定时任务列表
    pub fn list_scheduled_jobs(&self) -> Vec<ScheduledJob> {
        let jobs = self.scheduled_jobs.read();
        jobs.values().cloned().collect()
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> SchedulerStats {
        let tasks = self.tasks.read();
        let jobs = self.scheduled_jobs.read();

        let mut stats = SchedulerStats::default();

        for task in tasks.values() {
            match task.status {
                QueueStatus::Queued => stats.queued_count += 1,
                QueueStatus::Delayed => stats.delayed_count += 1,
                QueueStatus::Running => stats.running_count += 1,
                QueueStatus::Completed => stats.completed_count += 1,
                QueueStatus::Failed | QueueStatus::Cancelled | QueueStatus::TimedOut => {
                    stats.failed_count += 1
                }
            }
        }

        stats.running_count = self.running.read().len();
        stats.scheduled_jobs_count = jobs.len();

        stats
    }

    /// 启动调度器
    pub async fn run(&self) {
        tracing::info!("增强版任务调度器启动");

        *self.running_flag.write() = true;

        loop {
            if !*self.running_flag.read() {
                break;
            }

            self.process_delayed_queue().await;
            self.process_queue().await;
            self.process_scheduled_jobs().await;
            self.cleanup_finished_tasks();

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        tracing::info!("增强版任务调度器已停止");
    }

    /// 停止调度器
    pub fn stop(&self) {
        *self.running_flag.write() = false;
    }

    /// 处理延迟队列
    async fn process_delayed_queue(&self) {
        let now = Utc::now();

        let to_schedule = {
            let mut delayed = self.delayed.write();
            let mut ready = Vec::new();

            delayed.retain(|task| {
                if task.scheduled_at <= now {
                    ready.push(task.task_id);
                    false
                } else {
                    true
                }
            });

            ready
        };

        for task_id in to_schedule {
            // 更新任务状态为队列中
            {
                let mut tasks = self.tasks.write();
                if let Some(task) = tasks.get_mut(&task_id) {
                    task.status = QueueStatus::Queued;
                }
            }

            // 添加到主队列
            {
                let mut queue = self.queue.write();
                if let Some(task) = self.tasks.read().get(&task_id) {
                    queue.push(PriorityTask {
                        task_id,
                        priority: task.priority,
                        scheduled_at: Utc::now(),
                    });
                }
            }

            self.update_task_status_in_db(task_id, QueueStatus::Queued);
        }
    }

    /// 处理主队列
    async fn process_queue(&self) {
        let running_count = self.running.read().len();
        if running_count >= self.max_concurrent {
            return;
        }

        let available_slots = self.max_concurrent - running_count;

        let task_ids = {
            let mut queue = self.queue.write();
            let mut to_run = Vec::new();

            for _ in 0..available_slots {
                if let Some(priority_task) = queue.pop() {
                    to_run.push(priority_task.task_id);
                }
            }

            to_run
        };

        for task_id in task_ids {
            self.start_task(task_id).await;
        }
    }

    /// 处理定时任务
    async fn process_scheduled_jobs(&self) {
        let now = Utc::now();

        let jobs_to_run: Vec<Uuid> = {
            let jobs = self.scheduled_jobs.read();
            jobs.iter()
                .filter(|(_, job)| job.enabled && job.next_run <= now)
                .map(|(id, _)| *id)
                .collect()
        };

        for job_id in jobs_to_run {
            let (workflow, team_id, variables, cron_expr, schedule) = {
                let mut jobs = self.scheduled_jobs.write();
                if let Some(job) = jobs.get_mut(&job_id) {
                    // 计算下次执行时间
                    if let Some(ref schedule) = job.schedule {
                        job.next_run = schedule.next_run(now)
                            .unwrap_or_else(|| now + Duration::hours(1));
                    }

                    (
                        job.workflow.clone(),
                        job.team_id,
                        job.variables.clone(),
                        job.cron_expr.clone(),
                        job.schedule.clone(),
                    )
                } else {
                    continue;
                }
            };

            // 创建新任务
            let task_id = self.enqueue(workflow, team_id, variables, TaskPriority::Normal);

            // 更新定时任务的下次执行时间
            self.update_scheduled_job_next_run_in_db(job_id);

            tracing::info!("触发定时任务 {}: {}", job_id, cron_expr);
        }
    }

    /// 启动任务
    async fn start_task(&self, task_id: Uuid) {
        let (workflow, team_id, timeout_secs) = {
            let mut tasks = self.tasks.write();
            if let Some(task) = tasks.get_mut(&task_id) {
                if task.status != QueueStatus::Queued {
                    return;
                }
                task.status = QueueStatus::Running;
                task.started_at = Some(Utc::now());
                (task.workflow.clone(), task.team_id, task.timeout_secs)
            } else {
                return;
            }
        };

        tracing::info!("启动任务: {}", task_id);

        // 添加到运行中映射
        let task_id_clone = task_id;
        let executor = self.executor.clone();

        let handle = tokio::spawn(async move {
            let timeout = timeout_secs.map(|s| StdDuration::from_secs(s));

            let result = if let Some(timeout) = timeout {
                tokio::time::timeout(timeout, executor.execute(workflow, team_id)).await
                    .map_err(|_| std::time::Duration::from_secs(u64::MAX))
            } else {
                Ok(executor.execute(workflow, team_id).await)
            };

            // 更新任务结果
            Self::handle_task_result(task_id_clone, result).await;
        });

        self.running.write().insert(task_id, handle);

        self.update_task_status_in_db(task_id, QueueStatus::Running);
    }

    /// 处理任务结果
    async fn handle_task_result(task_id: Uuid, result: Result<Result<ExecutionResult, OrchestratorError>, StdDuration>) {
        // 这个方法需要在静态上下文中调用，因为它不使用 self
        tracing::info!("任务 {} 完成", task_id);
    }

    /// 清理已完成的任务
    fn cleanup_finished_tasks(&self) {
        let mut tasks = self.tasks.write();
        tasks.retain(|_, task| {
            // 保留队列中、延迟中、运行中的任务
            matches!(
                task.status,
                QueueStatus::Queued | QueueStatus::Delayed | QueueStatus::Running
            )
        });
    }

    /// 重试失败任务
    fn retry_task(&self, task_id: Uuid) -> bool {
        let (retry_count, retry_config) = {
            let tasks = self.tasks.read();
            match tasks.get(&task_id) {
                Some(task) => (task.retry_count, task.retry_config.clone()),
                None => return false,
            }
        };

        if retry_count >= retry_config.max_retries {
            tracing::info!("任务 {} 已达最大重试次数", task_id);
            return false;
        }

        let delay = retry_config.backoff_duration(retry_count);
        tracing::info!("任务 {} 将在 {:?} 后重试 (尝试 {})", task_id, delay, retry_count + 1);

        self.schedule_delayed(task_id, delay).ok();
        true
    }

    // =========================================================================
    // 数据库操作
    // =========================================================================

    fn save_task_to_db(&self, task: &QueuedTask) {
        if let Some(ref conn) = *self.db.read() {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO tasks
                 (id, workflow, team_id, variables, priority, status, retry_count,
                  retry_config, timeout_secs, created_at, started_at, finished_at, result, error)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    task.id.to_string(),
                    serde_json::to_string(&task.workflow).unwrap_or_default(),
                    task.team_id.0.to_string(),
                    serde_json::to_string(&task.variables).unwrap_or_default(),
                    task.priority.to_string(),
                    task.status.to_string(),
                    task.retry_count,
                    serde_json::to_string(&task.retry_config).unwrap_or_default(),
                    task.timeout_secs,
                    task.created_at.to_rfc3339(),
                    task.started_at.map(|t| t.to_rfc3339()),
                    task.finished_at.map(|t| t.to_rfc3339()),
                    task.result.as_ref().map(|r| serde_json::to_string(r).unwrap_or_default()),
                    task.error,
                ],
            );
        }
    }

    fn update_task_status_in_db(&self, task_id: Uuid, status: QueueStatus) {
        if let Some(ref conn) = *self.db.read() {
            let _ = conn.execute(
                "UPDATE tasks SET status = ?1, finished_at = ?2 WHERE id = ?3",
                params![
                    status.to_string(),
                    Utc::now().to_rfc3339(),
                    task_id.to_string(),
                ],
            );
        }
    }

    fn save_scheduled_job_to_db(&self, job: &ScheduledJob) -> Result<(), SchedulerError> {
        if let Some(ref conn) = *self.db.read() {
            conn.execute(
                "INSERT OR REPLACE INTO scheduled_jobs
                 (id, workflow, team_id, variables, cron_expr, next_run, enabled, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    job.id.to_string(),
                    serde_json::to_string(&job.workflow).unwrap_or_default(),
                    job.team_id.0.to_string(),
                    serde_json::to_string(&job.variables).unwrap_or_default(),
                    job.cron_expr,
                    job.next_run.to_rfc3339(),
                    job.enabled as i32,
                    job.created_at.to_rfc3339(),
                ],
            )?;
        }
        Ok(())
    }

    fn update_scheduled_job_enabled_in_db(&self, job_id: Uuid, enabled: bool) {
        if let Some(ref conn) = *self.db.read() {
            let _ = conn.execute(
                "UPDATE scheduled_jobs SET enabled = ?1 WHERE id = ?2",
                params![enabled as i32, job_id.to_string()],
            );
        }
    }

    fn update_scheduled_job_next_run_in_db(&self, job_id: Uuid) {
        if let Some(ref conn) = *self.db.read() {
            let next_run = {
                let jobs = self.scheduled_jobs.read();
                jobs.get(&job_id).map(|j| j.next_run.to_rfc3339())
            };

            if let Some(next_run) = next_run {
                let _ = conn.execute(
                    "UPDATE scheduled_jobs SET next_run = ?1 WHERE id = ?2",
                    params![next_run, job_id.to_string()],
                );
            }
        }
    }

    fn delete_scheduled_job_from_db(&self, job_id: Uuid) {
        if let Some(ref conn) = *self.db.read() {
            let _ = conn.execute(
                "DELETE FROM scheduled_jobs WHERE id = ?1",
                params![job_id.to_string()],
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_parse() {
        let schedule = CronSchedule::parse("0 * * * *").unwrap();
        assert!(schedule.minute.contains(&0));

        let schedule2 = CronSchedule::parse("*/5 * * * *").unwrap();
        assert!(schedule2.minute.contains(&0));
        assert!(schedule2.minute.contains(&5));

        let schedule3 = CronSchedule::parse("0,30 * * * *").unwrap();
        assert!(schedule3.minute.contains(&0));
        assert!(schedule3.minute.contains(&30));
    }

    #[test]
    fn test_retry_config() {
        let config = RetryConfig::default();
        assert_eq!(config.backoff_duration(0), StdDuration::from_secs(1));
        assert_eq!(config.backoff_duration(1), StdDuration::from_secs(2));
        assert_eq!(config.backoff_duration(2), StdDuration::from_secs(4));
    }

    #[test]
    fn test_task_priority_order() {
        let low = TaskPriority::Low;
        let high = TaskPriority::High;
        let critical = TaskPriority::Critical;
        let normal = TaskPriority::Normal;

        assert!(critical > high);
        assert!(high > normal);
        assert!(normal > low);
    }
}
