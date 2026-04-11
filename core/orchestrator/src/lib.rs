//! NexusFlow Orchestrator - Multi-Agent Orchestration Framework
//!
//! This module provides three core components:
//! - **Multi-CLI Orchestrator**: Unified interface to multiple CLI AI tools
//! - **Team Architecture v2**: Role-based agent collaboration
//! - **Message Bus Protocol**: Async pub-sub communication

pub mod cli;
pub mod team;
pub mod message_bus;
pub mod executor;
pub mod scheduler;
pub mod error;

pub use cli::{CliManager, CliProvider, CliRequest, CliResponse, CliTokenUsage};
pub use team::{TeamManager, AgentRole, AgentId, TeamId, Team, TeamMember, Capability};
pub use message_bus::{MessageBus, Channel, BusMessage, MessagePayload};
pub use executor::{WorkflowExecutor, ExecutionResult, ExecutionStatus, WorkflowDefinition, StageDefinition};
pub use scheduler::{
    TaskScheduler, TaskPriority, QueuedTask, QueueStatus, SchedulerStats,
    SchedulerError, ScheduledJob, CronSchedule, RetryConfig,
};
pub use error::{OrchestratorError, CliError, TeamError, BusError};
