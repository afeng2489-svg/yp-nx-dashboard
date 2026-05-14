//! NexusFlow Orchestrator - Multi-Agent Orchestration Framework
//!
//! This module provides three core components:
//! - **Multi-CLI Orchestrator**: Unified interface to multiple CLI AI tools
//! - **Team Architecture v2**: Role-based agent collaboration
//! - **Message Bus Protocol**: Async pub-sub communication

#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    clippy::redundant_closure,
    clippy::too_many_arguments,
    clippy::should_implement_trait,
    clippy::needless_borrow,
    clippy::if_same_then_else
)]

pub mod cli;
pub mod error;
pub mod executor;
pub mod message_bus;
pub mod planner;
pub mod scheduler;
pub mod session;
pub mod session_store;
pub mod team;

pub use cli::{CliManager, CliProvider, CliRequest, CliResponse, CliTokenUsage};
pub use error::{BusError, CliError, OrchestratorError, TeamError};
pub use executor::{
    ExecutionResult, ExecutionStatus, StageDefinition, WorkflowDefinition, WorkflowExecutor,
};
pub use message_bus::{BusMessage, Channel, MessageBus, MessagePayload};
pub use planner::{ExecutionPlan, Planner, PlannerError};
pub use scheduler::{
    CronSchedule, QueueStatus, QueuedTask, RetryConfig, ScheduledJob, SchedulerError,
    SchedulerStats, TaskExecutor, TaskPriority, TaskScheduler,
};
pub use session::{AgentResult, ChainEvent, ChainResult, TeamSessionActor};
pub use session_store::{SessionStore, SessionSummary, StoreError};
pub use team::{AgentId, AgentRole, Capability, Team, TeamId, TeamManager, TeamMember};
