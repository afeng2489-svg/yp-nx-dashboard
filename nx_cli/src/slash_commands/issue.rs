//! Issue Commands Module
//!
//! Provides slash commands for issue tracking: /issue:new, /issue:discover, /issue:plan

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{
    Command, CommandArgument, CommandCategory, CommandHandler, CommandOutput, Args,
    SubCommand,
};

/// Issue data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: IssueStatus,
    pub priority: Priority,
    pub assignee: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IssueStatus {
    Open,
    InProgress,
    Closed,
}

impl std::fmt::Display for IssueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueStatus::Open => write!(f, "open"),
            IssueStatus::InProgress => write!(f, "in_progress"),
            IssueStatus::Closed => write!(f, "closed"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Low => write!(f, "low"),
            Priority::Medium => write!(f, "medium"),
            Priority::High => write!(f, "high"),
            Priority::Critical => write!(f, "critical"),
        }
    }
}

/// Issue commands handler
pub struct IssueCommands;

impl IssueCommands {
    pub fn new() -> Self {
        Self
    }

    pub fn command_definition() -> Command {
        Command::new("issue", "Issue tracking commands", CommandCategory::Issue)
            .with_subcommand(
                SubCommand::new("new", "Create a new issue")
                    .with_arg(CommandArgument::new("title", "Issue title", true))
                    .with_arg(CommandArgument::new("description", "Issue description", false))
                    .with_arg(CommandArgument::new("priority", "Priority (low, medium, high, critical)", false).with_default("medium"))
                    .with_arg(CommandArgument::new("assignee", "Assignee username", false)),
            )
            .with_subcommand(
                SubCommand::new("discover", "Discover issues in the codebase")
                    .with_arg(CommandArgument::new("path", "Path to scan", false).with_default(".")),
            )
            .with_subcommand(
                SubCommand::new("plan", "Create a plan for resolving an issue")
                    .with_arg(CommandArgument::new("id", "Issue ID (e.g., ISS-001)", true)),
            )
            .with_subcommand(
                SubCommand::new("list", "List all issues")
                    .with_arg(CommandArgument::new("status", "Filter by status (open, in_progress, closed)", false)),
            )
            .with_subcommand(
                SubCommand::new("get", "Get issue details")
                    .with_arg(CommandArgument::new("id", "Issue ID", true)),
            )
    }
}

impl Default for IssueCommands {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandHandler for IssueCommands {
    fn name(&self) -> &str {
        "issue"
    }

    fn description(&self) -> &str {
        "Issue tracking commands"
    }

    fn arguments(&self) -> Vec<CommandArgument> {
        Vec::new()
    }

    async fn execute(&self, args: Args) -> CommandOutput {
        // This is called when the main command is used without subcommand
        CommandOutput::success("Available issue commands: /issue:new, /issue:discover, /issue:plan, /issue:list, /issue:get")
    }
}

/// Issue:New command handler
pub struct IssueNewCommand;

#[async_trait]
impl CommandHandler for IssueNewCommand {
    fn name(&self) -> &str {
        "issue:new"
    }

    fn description(&self) -> &str {
        "Create a new issue"
    }

    fn arguments(&self) -> Vec<CommandArgument> {
        vec![
            CommandArgument::new("title", "Issue title", true),
            CommandArgument::new("description", "Issue description", false),
            CommandArgument::new("priority", "Priority (low, medium, high, critical)", false)
                .with_default("medium"),
            CommandArgument::new("assignee", "Assignee username", false),
        ]
    }

    async fn execute(&self, args: Args) -> CommandOutput {
        let title = match args.get_required("title") {
            Ok(t) => t,
            Err(e) => return CommandOutput::error(e),
        };

        let description = args.get("description");
        let priority = args.get("priority").unwrap_or("medium");
        let assignee = args.get("assignee");

        // Generate issue ID (in real impl, this would be from database)
        let issue_id = "ISS-001".to_string();

        let issue = Issue {
            id: issue_id.clone(),
            title: title.to_string(),
            description: description.map(String::from),
            status: IssueStatus::Open,
            priority: match priority {
                "low" => Priority::Low,
                "high" => Priority::High,
                "critical" => Priority::Critical,
                _ => Priority::Medium,
            },
            assignee: assignee.map(String::from),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        tracing::info!("Created issue: {} - {}", issue.id, issue.title);

        CommandOutput::success_with_data(
            format!("Issue {} created successfully", issue_id),
            serde_json::to_value(&issue).unwrap_or_default(),
        )
    }
}

/// Issue:Discover command handler
pub struct IssueDiscoverCommand;

#[async_trait]
impl CommandHandler for IssueDiscoverCommand {
    fn name(&self) -> &str {
        "issue:discover"
    }

    fn description(&self) -> &str {
        "Discover issues in the codebase"
    }

    fn arguments(&self) -> Vec<CommandArgument> {
        vec![
            CommandArgument::new("path", "Path to scan", false).with_default("."),
        ]
    }

    async fn execute(&self, args: Args) -> CommandOutput {
        let path = args.get("path").unwrap_or(".");

        tracing::info!("Discovering issues in: {}", path);

        // Simulated discovered issues
        let discovered = serde_json::json!([
            {
                "id": "DISC-001",
                "title": "TODO comment in main.rs",
                "file": "nx_cli/src/main.rs",
                "line": 42,
                "severity": "low"
            },
            {
                "id": "DISC-002",
                "title": "Error handling could be improved",
                "file": "nx_cli/src/commands.rs",
                "line": 128,
                "severity": "medium"
            },
            {
                "id": "DISC-003",
                "title": "Potential memory leak in session manager",
                "file": "nx_session/src/lib.rs",
                "line": 256,
                "severity": "high"
            }
        ]);

        CommandOutput::success_with_data(
            format!("Discovered 3 potential issues in {}", path),
            discovered,
        )
    }
}

/// Issue:Plan command handler
pub struct IssuePlanCommand;

#[async_trait]
impl CommandHandler for IssuePlanCommand {
    fn name(&self) -> &str {
        "issue:plan"
    }

    fn description(&self) -> &str {
        "Create a plan for resolving an issue"
    }

    fn arguments(&self) -> Vec<CommandArgument> {
        vec![
            CommandArgument::new("id", "Issue ID (e.g., ISS-001)", true),
        ]
    }

    async fn execute(&self, args: Args) -> CommandOutput {
        let issue_id = match args.get_required("id") {
            Ok(id) => id,
            Err(e) => return CommandOutput::error(e),
        };

        tracing::info!("Creating plan for issue: {}", issue_id);

        let plan = serde_json::json!({
            "issue_id": issue_id,
            "title": "Plan for resolving issue",
            "steps": [
                {
                    "order": 1,
                    "description": "Analyze the root cause",
                    "estimated_time": "30 minutes",
                    "agent": "researcher"
                },
                {
                    "order": 2,
                    "description": "Implement the fix",
                    "estimated_time": "1 hour",
                    "agent": "coder"
                },
                {
                    "order": 3,
                    "description": "Write or update tests",
                    "estimated_time": "30 minutes",
                    "agent": "tester"
                },
                {
                    "order": 4,
                    "description": "Review and merge",
                    "estimated_time": "15 minutes",
                    "agent": "reviewer"
                }
            ],
            "total_estimated_time": "2 hours 15 minutes"
        });

        CommandOutput::success_with_data(
            format!("Plan created for issue {}", issue_id),
            plan,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_issue_new_command() {
        let cmd = IssueNewCommand;
        let args = Args::new()
            .with_arg("title", "Test issue")
            .with_arg("priority", "high");

        let output = cmd.execute(args).await;
        assert!(output.success);
        assert!(output.data.is_some());
    }

    #[tokio::test]
    async fn test_issue_new_command_missing_title() {
        let cmd = IssueNewCommand;
        let args = Args::new();

        let output = cmd.execute(args).await;
        assert!(!output.success);
    }

    #[tokio::test]
    async fn test_issue_discover_command() {
        let cmd = IssueDiscoverCommand;
        let args = Args::new().with_arg("path", "./src");

        let output = cmd.execute(args).await;
        assert!(output.success);
        assert!(output.data.is_some());
    }

    #[tokio::test]
    async fn test_issue_plan_command() {
        let cmd = IssuePlanCommand;
        let args = Args::new().with_arg("id", "ISS-001");

        let output = cmd.execute(args).await;
        assert!(output.success);
        assert!(output.data.is_some());
    }
}
