//! Orchestrator Error Types

use thiserror::Error;

/// Main orchestrator error type
#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("CLI error: {0}")]
    Cli(#[from] CliError),

    #[error("Team error: {0}")]
    Team(#[from] TeamError),

    #[error("Message bus error: {0}")]
    MessageBus(#[from] BusError),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// CLI-related errors
#[derive(Error, Debug)]
pub enum CliError {
    #[error("CLI not found: {0}")]
    NotFound(String),

    #[error("CLI execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Timeout after {0} seconds")]
    Timeout(u64),

    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Team-related errors
#[derive(Error, Debug)]
pub enum TeamError {
    #[error("Team not found: {0:?}")]
    TeamNotFound(crate::team::TeamId),

    #[error("Agent not found: {0:?}")]
    AgentNotFound(crate::team::AgentId),

    #[error("Task join failed: {0}")]
    TaskJoinFailed(String),

    #[error("CLI execution failed: {0}")]
    CliExecutionFailed(String),

    #[error("Invalid workflow: {0}")]
    InvalidWorkflow(String),

    #[error("Agent error: {0}")]
    Agent(String),
}

/// Message bus errors
#[derive(Error, Debug)]
pub enum BusError {
    #[error("Subscription failed: {0}")]
    SubscriptionFailed(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Timeout")]
    Timeout,

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),
}
