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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::team::{AgentId, TeamId};

    // ── OrchestratorError ───────────────────────────────────────────

    #[test]
    fn test_orchestrator_error_display_cli() {
        let err = OrchestratorError::Cli(CliError::NotFound("claude".into()));
        let msg = err.to_string();
        assert!(msg.contains("CLI error"));
        assert!(msg.contains("claude"));
    }

    #[test]
    fn test_orchestrator_error_display_team() {
        let err = OrchestratorError::Team(TeamError::AgentNotFound(AgentId::new()));
        let msg = err.to_string();
        assert!(msg.contains("Team error"));
    }

    #[test]
    fn test_orchestrator_error_display_message_bus() {
        let err = OrchestratorError::MessageBus(BusError::SendFailed("full".into()));
        let msg = err.to_string();
        assert!(msg.contains("Message bus error"));
    }

    #[test]
    fn test_orchestrator_error_execution() {
        let err = OrchestratorError::Execution("something broke".into());
        assert_eq!(err.to_string(), "Execution error: something broke");
    }

    #[test]
    fn test_orchestrator_error_timeout() {
        let err = OrchestratorError::Timeout("task took too long".into());
        assert_eq!(err.to_string(), "Timeout: task took too long");
    }

    #[test]
    fn test_orchestrator_error_not_found() {
        let err = OrchestratorError::NotFound("workflow".into());
        assert_eq!(err.to_string(), "Not found: workflow");
    }

    #[test]
    fn test_orchestrator_error_invalid_state() {
        let err = OrchestratorError::InvalidState("bad transition".into());
        assert_eq!(err.to_string(), "Invalid state: bad transition");
    }

    #[test]
    fn test_orchestrator_error_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = OrchestratorError::Io(io_err);
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn test_orchestrator_error_from_cli() {
        let cli_err = CliError::Timeout(30);
        let err: OrchestratorError = cli_err.into();
        assert!(err.to_string().contains("CLI error"));
    }

    #[test]
    fn test_orchestrator_error_from_team() {
        let team_err = TeamError::InvalidWorkflow("no stages".into());
        let err: OrchestratorError = team_err.into();
        assert!(err.to_string().contains("Team error"));
    }

    #[test]
    fn test_orchestrator_error_from_bus() {
        let bus_err = BusError::ChannelClosed;
        let err: OrchestratorError = bus_err.into();
        assert!(err.to_string().contains("Message bus error"));
    }

    #[test]
    fn test_orchestrator_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err: OrchestratorError = io_err.into();
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn test_orchestrator_error_debug() {
        let err = OrchestratorError::Execution("test".into());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Execution"));
    }

    #[test]
    fn test_orchestrator_error_all_variants_constructible() {
        let variants: Vec<OrchestratorError> = vec![
            OrchestratorError::Execution("e".into()),
            OrchestratorError::Timeout("t".into()),
            OrchestratorError::NotFound("n".into()),
            OrchestratorError::InvalidState("s".into()),
            OrchestratorError::Cli(CliError::NotFound("c".into())),
            OrchestratorError::Team(TeamError::AgentNotFound(AgentId::new())),
            OrchestratorError::MessageBus(BusError::ChannelClosed),
            OrchestratorError::Io(std::io::Error::new(std::io::ErrorKind::Other, "io")),
        ];
        assert_eq!(variants.len(), 8);
        for v in &variants {
            assert!(!v.to_string().is_empty());
        }
    }

    // ── CliError ────────────────────────────────────────────────────

    #[test]
    fn test_cli_error_not_found() {
        let err = CliError::NotFound("claude".into());
        assert_eq!(err.to_string(), "CLI not found: claude");
    }

    #[test]
    fn test_cli_error_execution_failed() {
        let err = CliError::ExecutionFailed("timeout".into());
        assert_eq!(err.to_string(), "CLI execution failed: timeout");
    }

    #[test]
    fn test_cli_error_timeout() {
        let err = CliError::Timeout(120);
        assert_eq!(err.to_string(), "Timeout after 120 seconds");
    }

    #[test]
    fn test_cli_error_invalid_response() {
        let err = CliError::InvalidResponse("bad json".into());
        assert_eq!(err.to_string(), "Invalid response format: bad json");
    }

    #[test]
    fn test_cli_error_provider() {
        let err = CliError::Provider("auth failed".into());
        assert_eq!(err.to_string(), "Provider error: auth failed");
    }

    #[test]
    fn test_cli_error_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection");
        let err = CliError::Io(io_err);
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn test_cli_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timed out");
        let err: CliError = io_err.into();
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn test_cli_error_all_variants() {
        let variants: Vec<CliError> = vec![
            CliError::NotFound("x".into()),
            CliError::ExecutionFailed("x".into()),
            CliError::Timeout(99),
            CliError::InvalidResponse("x".into()),
            CliError::Provider("x".into()),
            CliError::Io(std::io::Error::new(std::io::ErrorKind::Other, "x")),
        ];
        assert_eq!(variants.len(), 6);
        for v in &variants {
            assert!(!v.to_string().is_empty());
        }
    }

    // ── TeamError ───────────────────────────────────────────────────

    #[test]
    fn test_team_error_team_not_found() {
        let err = TeamError::TeamNotFound(TeamId::new());
        let msg = err.to_string();
        assert!(msg.starts_with("Team not found:"));
    }

    #[test]
    fn test_team_error_agent_not_found() {
        let err = TeamError::AgentNotFound(AgentId::new());
        let msg = err.to_string();
        assert!(msg.starts_with("Agent not found:"));
    }

    #[test]
    fn test_team_error_task_join_failed() {
        let err = TeamError::TaskJoinFailed("panic".into());
        assert_eq!(err.to_string(), "Task join failed: panic");
    }

    #[test]
    fn test_team_error_cli_execution_failed() {
        let err = TeamError::CliExecutionFailed("crash".into());
        assert_eq!(err.to_string(), "CLI execution failed: crash");
    }

    #[test]
    fn test_team_error_invalid_workflow() {
        let err = TeamError::InvalidWorkflow("no stages".into());
        assert_eq!(err.to_string(), "Invalid workflow: no stages");
    }

    #[test]
    fn test_team_error_agent() {
        let err = TeamError::Agent("unavailable".into());
        assert_eq!(err.to_string(), "Agent error: unavailable");
    }

    #[test]
    fn test_team_error_all_variants() {
        let variants: Vec<TeamError> = vec![
            TeamError::TeamNotFound(TeamId::new()),
            TeamError::AgentNotFound(AgentId::new()),
            TeamError::TaskJoinFailed("x".into()),
            TeamError::CliExecutionFailed("x".into()),
            TeamError::InvalidWorkflow("x".into()),
            TeamError::Agent("x".into()),
        ];
        assert_eq!(variants.len(), 6);
        for v in &variants {
            assert!(!v.to_string().is_empty());
        }
    }

    // ── BusError ────────────────────────────────────────────────────

    #[test]
    fn test_bus_error_subscription_failed() {
        let err = BusError::SubscriptionFailed("capacity".into());
        assert_eq!(err.to_string(), "Subscription failed: capacity");
    }

    #[test]
    fn test_bus_error_send_failed() {
        let err = BusError::SendFailed("channel closed".into());
        assert_eq!(err.to_string(), "Send failed: channel closed");
    }

    #[test]
    fn test_bus_error_channel_closed() {
        let err = BusError::ChannelClosed;
        assert_eq!(err.to_string(), "Channel closed");
    }

    #[test]
    fn test_bus_error_timeout() {
        let err = BusError::Timeout;
        assert_eq!(err.to_string(), "Timeout");
    }

    #[test]
    fn test_bus_error_serialization() {
        let err = BusError::Serialization("bad utf-8".into());
        assert_eq!(err.to_string(), "Serialization error: bad utf-8");
    }

    #[test]
    fn test_bus_error_deserialization() {
        let err = BusError::Deserialization("bad json".into());
        assert_eq!(err.to_string(), "Deserialization error: bad json");
    }

    #[test]
    fn test_bus_error_all_variants() {
        let variants: Vec<BusError> = vec![
            BusError::SubscriptionFailed("x".into()),
            BusError::SendFailed("x".into()),
            BusError::ChannelClosed,
            BusError::Timeout,
            BusError::Serialization("x".into()),
            BusError::Deserialization("x".into()),
        ];
        assert_eq!(variants.len(), 6);
        for v in &variants {
            assert!(!v.to_string().is_empty());
        }
    }

    // ── Cross-module ────────────────────────────────────────────────

    #[test]
    fn test_all_error_types_impl_debug_and_display() {
        let errors: Vec<Box<dyn std::error::Error>> = vec![
            Box::new(OrchestratorError::Execution("x".into())),
            Box::new(CliError::NotFound("x".into())),
            Box::new(TeamError::Agent("x".into())),
            Box::new(BusError::ChannelClosed),
        ];
        for err in &errors {
            let _ = format!("{}", err);
            let _ = format!("{:?}", err);
        }
    }
}
