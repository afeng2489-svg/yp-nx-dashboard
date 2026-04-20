//! Command Dispatcher Module
//!
//! Parses and dispatches slash commands to appropriate handlers.

use std::collections::HashMap;

use super::{
    Args, CommandHandler, CommandOutput, CommandRegistry,
    build_registry,
};

/// Type alias for backwards compatibility
pub type CommandDispatcher = SlashCommandDispatcher;

/// Parsed command with subcommand
#[derive(Debug, Clone)]
pub struct ParsedCommand {
    pub command: String,
    pub subcommand: Option<String>,
    pub args: Args,
}

/// Command dispatch result
#[derive(Debug)]
pub struct DispatchResult {
    pub output: CommandOutput,
    pub execution_time_ms: u64,
}

impl DispatchResult {
    pub fn new(output: CommandOutput, execution_time_ms: u64) -> Self {
        Self {
            output,
            execution_time_ms,
        }
    }
}

/// Command parser and dispatcher
pub struct SlashCommandDispatcher {
    registry: CommandRegistry,
    handlers: HashMap<String, Box<dyn CommandHandler>>,
}

impl SlashCommandDispatcher {
    pub fn new() -> Self {
        let registry = build_registry();
        let mut dispatcher = Self {
            registry,
            handlers: HashMap::new(),
        };
        dispatcher.register_handlers();
        dispatcher
    }

    fn register_handlers(&mut self) {
        // Issue handlers
        self.register_handler(super::issue::IssueNewCommand);
        self.register_handler(super::issue::IssueDiscoverCommand);
        self.register_handler(super::issue::IssuePlanCommand);

        // Workflow handlers
        self.register_handler(super::workflow::WorkflowExecuteCommand);
        self.register_handler(super::workflow::WorkflowListCommand);
        self.register_handler(super::workflow::WorkflowStatusCommand);
        self.register_handler(super::workflow::WorkflowStopCommand);

        // Session handlers
        self.register_handler(super::session::SessionResumeCommand);
        self.register_handler(super::session::SessionPauseCommand);
        self.register_handler(super::session::SessionListCommand);
        self.register_handler(super::session::SessionGetCommand);
        self.register_handler(super::session::SessionDeleteCommand);
    }

    fn register_handler<H: CommandHandler + 'static>(&mut self, handler: H) {
        let name = handler.name().to_string();
        self.handlers.insert(name, Box::new(handler));
    }

    /// Parse a slash command string into components
    pub fn parse(raw: &str) -> Result<ParsedCommand, ParseError> {
        let raw = raw.trim();
        if !raw.starts_with('/') {
            return Err(ParseError::NotASlashCommand(raw.to_string()));
        }

        let input = &raw[1..]; // Remove leading '/'
        let parts: Vec<&str> = input.split_whitespace().collect();

        if parts.is_empty() {
            return Err(ParseError::EmptyCommand);
        }

        let command_parts: Vec<&str> = parts[0].split(':').collect();
        let command = command_parts[0].to_string();
        let subcommand = if command_parts.len() > 1 {
            Some(command_parts[1].to_string())
        } else {
            None
        };

        // Parse arguments — handle quoted values (e.g. key="value with spaces")
        let mut args = Args::new();
        let arg_str = parts[1..].join(" ");
        let mut remaining = arg_str.as_str();
        while !remaining.is_empty() {
            remaining = remaining.trim_start();
            if remaining.is_empty() {
                break;
            }
            if let Some(eq_pos) = remaining.find('=') {
                let key = remaining[..eq_pos].trim();
                let after_eq = &remaining[eq_pos + 1..];
                let (value, rest) = if after_eq.starts_with('"') {
                    // Quoted value: scan for closing quote
                    let inner = &after_eq[1..];
                    if let Some(close) = inner.find('"') {
                        (&inner[..close], inner[close + 1..].trim_start())
                    } else {
                        // Unclosed quote — treat rest as value
                        (inner, "")
                    }
                } else {
                    // Unquoted value: ends at next whitespace
                    let end = after_eq.find(char::is_whitespace).unwrap_or(after_eq.len());
                    (after_eq[..end].trim(), after_eq[end..].trim_start())
                };
                if !key.is_empty() {
                    args = args.with_arg(key, value);
                }
                remaining = rest;
            } else {
                // Positional argument (no '=')
                let end = remaining.find(char::is_whitespace).unwrap_or(remaining.len());
                let token = &remaining[..end];
                let arg_name = format!("arg{}", args.keys().count() + 1);
                args = args.with_arg(arg_name, token);
                remaining = remaining[end..].trim_start();
            }
        }

        Ok(ParsedCommand {
            command,
            subcommand,
            args,
        })
    }

    /// Dispatch a parsed command to the appropriate handler
    pub async fn dispatch(&self, parsed: ParsedCommand) -> DispatchResult {
        let start = std::time::Instant::now();

        let output = match &parsed.subcommand {
            Some(subcmd) => {
                let full_name = format!("{}:{}", parsed.command, subcmd);
                self.dispatch_to_handler(&full_name, parsed.args).await
            }
            None => {
                // Dispatch to main command handler
                self.dispatch_to_handler(&parsed.command, parsed.args).await
            }
        };

        let elapsed = start.elapsed().as_millis() as u64;
        DispatchResult::new(output, elapsed)
    }

    /// Dispatch a raw slash command string
    pub async fn dispatch_raw(&self, raw: &str) -> Result<DispatchResult, ParseError> {
        let parsed = Self::parse(raw)?;
        Ok(self.dispatch(parsed).await)
    }

    async fn dispatch_to_handler(&self, name: &str, args: Args) -> CommandOutput {
        if let Some(handler) = self.handlers.get(name) {
            handler.execute(args).await
        } else {
            CommandOutput::error(format!("Unknown command: /{}", name))
        }
    }

    /// Get command suggestions for autocomplete
    pub fn get_suggestions(&self, partial: &str) -> Vec<CommandSuggestion> {
        let partial = partial.trim_start_matches('/');
        let mut suggestions = Vec::new();

        for (name, desc, _) in self.registry.get_all_subcommands() {
            if name.starts_with(partial) || format!("{}:{}", name, "").starts_with(partial) {
                suggestions.push(CommandSuggestion {
                    command: format!("{}:{}", name, "").trim_end_matches(':').to_string(),
                    description: desc.to_string(),
                    arguments: Vec::new(), // TODO: include argument hints from subcommand
                });
            }
        }

        // Sort by command name
        suggestions.sort_by(|a, b| a.command.cmp(&b.command));
        suggestions
    }

    /// Get help text for a command
    pub fn get_help(&self, command_name: &str) -> Option<String> {
        let parts: Vec<&str> = command_name.split(':').collect();
        let cmd_name = parts[0];

        if let Some(cmd) = self.registry.get(cmd_name) {
            let mut help = format!("# /{} - {}\n\n", cmd.name, cmd.description);

            if parts.len() > 1 {
                let subcmd_name = parts[1];
                if let Some(subcmd) = cmd.subcommands.iter().find(|s| s.name == subcmd_name) {
                    help.push_str(&format!("## /{}:{}\n\n{}\n\n", cmd_name, subcmd.name, subcmd.description));
                    if !subcmd.arguments.is_empty() {
                        help.push_str("### Arguments\n\n");
                        for arg in &subcmd.arguments {
                            let required = if arg.required { "(required)" } else { "(optional)" };
                            help.push_str(&format!(
                                "- `{}` {}: {}\n",
                                arg.name, required, arg.description
                            ));
                        }
                    }
                }
            } else {
                help.push_str("## Subcommands\n\n");
                for subcmd in &cmd.subcommands {
                    help.push_str(&format!(
                        "- `/{}:{}` - {}\n",
                        cmd.name, subcmd.name, subcmd.description
                    ));
                }
            }

            Some(help)
        } else {
            None
        }
    }

    /// List all available commands
    pub fn list_all_commands(&self) -> Vec<CommandInfo> {
        self.registry
            .get_all_subcommands()
            .iter()
            .map(|(cmd, subcmd, desc)| CommandInfo {
                command: format!("{}:{}", cmd, subcmd),
                description: desc.to_string(),
            })
            .collect()
    }
}

impl Default for SlashCommandDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Command suggestion for autocomplete
#[derive(Debug, Clone)]
pub struct CommandSuggestion {
    pub command: String,
    pub description: String,
    pub arguments: Vec<ArgumentHint>,
}

#[derive(Debug, Clone)]
pub struct ArgumentHint {
    pub name: String,
    pub description: String,
    pub required: bool,
}

/// Command info
#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub command: String,
    pub description: String,
}

/// Parse error
#[derive(Debug, Clone)]
pub enum ParseError {
    NotASlashCommand(String),
    EmptyCommand,
    Custom(String),
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self::Custom(message.into())
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::NotASlashCommand(s) => write!(f, "Not a slash command: {}", s),
            ParseError::EmptyCommand => write!(f, "Empty command"),
            ParseError::Custom(s) => write!(f, "Parse error: {}", s),
        }
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_command() {
        let result = SlashCommandDispatcher::parse("/issue:new");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.command, "issue");
        assert_eq!(parsed.subcommand, Some("new".to_string()));
    }

    #[test]
    fn test_parse_command_with_args() {
        let result = SlashCommandDispatcher::parse("/issue:new title=\"Test issue\" priority=high");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.command, "issue");
        assert_eq!(parsed.subcommand, Some("new".to_string()));
        assert_eq!(parsed.args.get("title"), Some("Test issue"));
        assert_eq!(parsed.args.get("priority"), Some("high"));
    }

    #[test]
    fn test_parse_command_without_subcommand() {
        let result = SlashCommandDispatcher::parse("/issue");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.command, "issue");
        assert!(parsed.subcommand.is_none());
    }

    #[test]
    fn test_parse_error_not_slash() {
        let result = SlashCommandDispatcher::parse("issue:new");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_empty() {
        let result = SlashCommandDispatcher::parse("/");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_dispatch_issue_new() {
        let dispatcher = SlashCommandDispatcher::new();
        let parsed = ParsedCommand {
            command: "issue".to_string(),
            subcommand: Some("new".to_string()),
            args: Args::new().with_arg("title", "Test issue"),
        };

        let result = dispatcher.dispatch(parsed).await;
        assert!(result.output.success);
    }

    #[tokio::test]
    async fn test_dispatch_unknown_command() {
        let dispatcher = SlashCommandDispatcher::new();
        let parsed = ParsedCommand {
            command: "unknown".to_string(),
            subcommand: None,
            args: Args::new(),
        };

        let result = dispatcher.dispatch(parsed).await;
        assert!(!result.output.success);
    }

    #[test]
    fn test_get_suggestions() {
        let dispatcher = SlashCommandDispatcher::new();
        let suggestions = dispatcher.get_suggestions("/issue");
        assert!(!suggestions.is_empty());
    }

    #[test]
    fn test_get_help() {
        let dispatcher = SlashCommandDispatcher::new();
        let help = dispatcher.get_help("issue:new");
        assert!(help.is_some());
        let help_text = help.unwrap();
        assert!(help_text.contains("issue:new"));
    }

    #[test]
    fn test_list_all_commands() {
        let dispatcher = SlashCommandDispatcher::new();
        let commands = dispatcher.list_all_commands();
        assert!(!commands.is_empty());
        assert!(commands.iter().any(|c| c.command == "issue:new"));
        assert!(commands.iter().any(|c| c.command == "workflow:execute"));
        assert!(commands.iter().any(|c| c.command == "session:resume"));
    }
}
