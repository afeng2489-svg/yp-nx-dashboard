//! NexusFlow Slash Commands Module
//!
//! Provides a extensible slash command system for CLI and frontend integration.

mod dispatch;
mod issue;
mod session;
mod workflow;

pub use dispatch::{CommandDispatcher, DispatchResult};
pub use issue::IssueCommands;
pub use session::SessionCommands;
pub use workflow::WorkflowCommands;

use async_trait::async_trait;
use std::collections::HashMap;

/// Command argument definition
#[derive(Debug, Clone)]
pub struct CommandArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

impl CommandArgument {
    pub fn new(name: impl Into<String>, description: impl Into<String>, required: bool) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            required,
            default: None,
        }
    }

    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }
}

/// Command output definition
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl CommandOutput {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
        }
    }

    pub fn success_with_data(
        message: impl Into<String>,
        data: impl Into<serde_json::Value>,
    ) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data.into()),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }

    pub fn error_with_data(message: impl Into<String>, data: impl Into<serde_json::Value>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: Some(data.into()),
        }
    }
}

/// Parsed command arguments
#[derive(Debug, Clone, Default)]
pub struct Args {
    args: HashMap<String, String>,
}

impl Args {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_arg(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.args.insert(key.into(), value.into());
        self
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.args.get(key).map(String::as_str)
    }

    pub fn get_required(&self, key: &str) -> Result<&str, String> {
        self.args
            .get(key)
            .map(String::as_str)
            .ok_or_else(|| format!("Missing required argument: {}", key))
    }

    pub fn contains(&self, key: &str) -> bool {
        self.args.contains_key(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.args.keys().map(String::as_str)
    }
}

/// Command handler trait
#[async_trait]
pub trait CommandHandler: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn arguments(&self) -> Vec<CommandArgument>;
    async fn execute(&self, args: Args) -> CommandOutput;
}

/// Command definition
#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub category: CommandCategory,
    pub subcommands: Vec<SubCommand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    Issue,
    Workflow,
    Session,
    System,
}

impl Command {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        category: CommandCategory,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            category,
            subcommands: Vec::new(),
        }
    }

    pub fn with_subcommand(mut self, subcommand: SubCommand) -> Self {
        self.subcommands.push(subcommand);
        self
    }
}

/// Subcommand definition
#[derive(Debug, Clone)]
pub struct SubCommand {
    pub name: String,
    pub description: String,
    pub arguments: Vec<CommandArgument>,
}

impl SubCommand {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            arguments: Vec::new(),
        }
    }

    pub fn with_arg(mut self, arg: CommandArgument) -> Self {
        self.arguments.push(arg);
        self
    }
}

/// Command registry for managing available commands
#[derive(Debug, Default)]
pub struct CommandRegistry {
    commands: HashMap<String, Command>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, command: Command) {
        self.commands.insert(command.name.clone(), command);
    }

    pub fn get(&self, name: &str) -> Option<&Command> {
        self.commands.get(name)
    }

    pub fn get_subcommand(&self, command_name: &str, subcommand_name: &str) -> Option<&SubCommand> {
        self.commands
            .get(command_name)
            .and_then(|cmd| cmd.subcommands.iter().find(|s| s.name == subcommand_name))
    }

    pub fn list_commands(&self) -> Vec<&Command> {
        self.commands.values().collect()
    }

    pub fn list_by_category(&self, category: CommandCategory) -> Vec<&Command> {
        self.commands
            .values()
            .filter(|cmd| cmd.category == category)
            .collect()
    }

    pub fn get_all_subcommands(&self) -> Vec<(&str, &str, &str)> {
        self.commands
            .values()
            .flat_map(|cmd| {
                cmd.subcommands
                    .iter()
                    .map(|s| (cmd.name.as_str(), s.name.as_str(), s.description.as_str()))
            })
            .collect()
    }
}

/// Build the default command registry
pub fn build_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();

    // Issue commands
    registry.register(IssueCommands::command_definition());
    registry.register(WorkflowCommands::command_definition());
    registry.register(SessionCommands::command_definition());

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args() {
        let args = Args::new().with_arg("name", "test").with_arg("id", "123");

        assert_eq!(args.get("name"), Some("test"));
        assert_eq!(args.get("id"), Some("123"));
        assert_eq!(args.get("missing"), None);
    }

    #[test]
    fn test_args_required() {
        let args = Args::new().with_arg("name", "test");

        assert_eq!(args.get_required("name"), Ok("test"));
        assert!(args.get_required("missing").is_err());
    }

    #[test]
    fn test_command_output() {
        let output = CommandOutput::success("Done");
        assert!(output.success);
        assert_eq!(output.message, "Done");

        let error = CommandOutput::error("Failed");
        assert!(!error.success);
        assert_eq!(error.message, "Failed");
    }

    #[test]
    fn test_registry() {
        let mut registry = CommandRegistry::new();
        registry.register(Command::new(
            "issue",
            "Issue commands",
            CommandCategory::Issue,
        ));

        assert!(registry.get("issue").is_some());
        assert!(registry.get("missing").is_none());
    }
}
