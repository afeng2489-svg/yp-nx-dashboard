//! Team Evolution 独立错误类型
//!
//! 错误码格式: [TEV-Nxxx]
//! - TEV-1xxx: Snapshot 相关
//! - TEV-2xxx: Resume / 崩溃恢复
//! - TEV-3xxx: FileWatch 文件监控
//! - TEV-4xxx: FeatureFlag 功能开关
//! - TEV-5xxx: ProcessIsolation 进程隔离
//! - TEV-6xxx: Pipeline 任务编排
//!
//! 这些错误不传播到核心 `crate::error`，在 route handler 层转 HTTP 响应。

use std::fmt;

#[derive(Debug)]
pub enum TeamEvolutionError {
    // TEV-4xxx: Feature Flag
    /// [TEV-4001] Feature flag not found
    FlagNotFound(String),
    /// [TEV-4002] Feature is disabled
    FeatureDisabled(String),
    /// [TEV-4003] Feature is in readonly mode
    FeatureReadOnly(String),
    /// [TEV-4004] Circuit breaker tripped
    CircuitBreakerTripped { key: String, error_count: u32 },

    // TEV-6xxx: Pipeline
    /// [TEV-6001] Pipeline not found
    PipelineNotFound(String),
    /// [TEV-6002] Pipeline already running
    PipelineAlreadyRunning(String),
    /// [TEV-6003] Pipeline step not found
    StepNotFound { pipeline_id: String, step_id: String },
    /// [TEV-6004] Phase gate not satisfied
    PhaseGateBlocked { phase: String, reason: String },
    /// [TEV-6005] Step dependencies not met
    DependenciesNotMet { step_id: String, blocked_by: Vec<String> },
    /// [TEV-6006] Pipeline is paused
    PipelinePaused(String),
    /// [TEV-6007] Step cannot be retried
    StepNotRetriable { step_id: String, status: String },

    // TEV-1xxx: Snapshot
    /// [TEV-1001] Snapshot save failed
    SnapshotSaveFailed(String),
    /// [TEV-1002] Snapshot not found
    SnapshotNotFound(String),

    // TEV-2xxx: Resume
    /// [TEV-2001] Checkpoint not found
    CheckpointNotFound(String),
    /// [TEV-2002] Resume failed
    ResumeFailed(String),

    // TEV-5xxx: Process
    /// [TEV-5001] Resource limit reached
    ResourceLimitReached { current: usize, max: usize },
    /// [TEV-5002] Process not found
    ProcessNotFound(String),

    // TEV-3xxx: File Watch
    /// [TEV-3001] File watch error
    FileWatchError(String),

    // General
    /// Database error
    Database(String),
    /// Internal error
    Internal(String),
}

impl fmt::Display for TeamEvolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Feature Flag
            Self::FlagNotFound(key) => write!(f, "[TEV-4001] Feature flag not found: {key}"),
            Self::FeatureDisabled(key) => write!(f, "[TEV-4002] Feature is disabled: {key}"),
            Self::FeatureReadOnly(key) => write!(f, "[TEV-4003] Feature is in readonly mode: {key}"),
            Self::CircuitBreakerTripped { key, error_count } => {
                write!(f, "[TEV-4004] Circuit breaker tripped for '{key}' after {error_count} errors")
            }

            // Pipeline
            Self::PipelineNotFound(id) => write!(f, "[TEV-6001] Pipeline not found: {id}"),
            Self::PipelineAlreadyRunning(id) => write!(f, "[TEV-6002] Pipeline already running: {id}"),
            Self::StepNotFound { pipeline_id, step_id } => {
                write!(f, "[TEV-6003] Step '{step_id}' not found in pipeline '{pipeline_id}'")
            }
            Self::PhaseGateBlocked { phase, reason } => {
                write!(f, "[TEV-6004] Phase gate blocked at '{phase}': {reason}")
            }
            Self::DependenciesNotMet { step_id, blocked_by } => {
                write!(f, "[TEV-6005] Step '{step_id}' blocked by: {}", blocked_by.join(", "))
            }
            Self::PipelinePaused(id) => write!(f, "[TEV-6006] Pipeline is paused: {id}"),
            Self::StepNotRetriable { step_id, status } => {
                write!(f, "[TEV-6007] Step '{step_id}' cannot be retried (status: {status})")
            }

            // Snapshot
            Self::SnapshotSaveFailed(msg) => write!(f, "[TEV-1001] Snapshot save failed: {msg}"),
            Self::SnapshotNotFound(id) => write!(f, "[TEV-1002] Snapshot not found: {id}"),

            // Resume
            Self::CheckpointNotFound(id) => write!(f, "[TEV-2001] Checkpoint not found: {id}"),
            Self::ResumeFailed(msg) => write!(f, "[TEV-2002] Resume failed: {msg}"),

            // Process
            Self::ResourceLimitReached { current, max } => {
                write!(f, "[TEV-5001] Resource limit reached: {current}/{max}")
            }
            Self::ProcessNotFound(id) => write!(f, "[TEV-5002] Process not found: {id}"),

            // File Watch
            Self::FileWatchError(msg) => write!(f, "[TEV-3001] File watch error: {msg}"),

            // General
            Self::Database(msg) => write!(f, "[TEV-0001] Database error: {msg}"),
            Self::Internal(msg) => write!(f, "[TEV-0002] Internal error: {msg}"),
        }
    }
}

impl std::error::Error for TeamEvolutionError {}

impl From<rusqlite::Error> for TeamEvolutionError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Database(err.to_string())
    }
}
