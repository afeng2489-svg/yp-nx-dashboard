//! NexusFlow Orchestrator - Multi-Agent Orchestration Framework
//!
//! This module provides three core components:
//! - **Multi-CLI Orchestrator**: Unified interface to multiple CLI AI tools
//! - **Team Architecture v2**: Role-based agent collaboration
//! - **Message Bus Protocol**: Async pub-sub communication

pub mod cli;
pub mod error;
pub mod executor;
pub mod message_bus;
pub mod scheduler;
pub mod team;

pub use cli::{CliManager, CliProvider, CliRequest, CliResponse, CliTokenUsage};
pub use error::{BusError, CliError, OrchestratorError, TeamError};
pub use executor::{
    ExecutionResult, ExecutionStatus, StageDefinition, WorkflowDefinition, WorkflowExecutor,
};
pub use message_bus::{BusMessage, Channel, MessageBus, MessagePayload};
pub use scheduler::{
    CronSchedule, QueueStatus, QueuedTask, RetryConfig, ScheduledJob, SchedulerError,
    SchedulerStats, TaskPriority, TaskScheduler,
};
pub use team::{AgentId, AgentRole, Capability, Team, TeamId, TeamManager, TeamMember};
