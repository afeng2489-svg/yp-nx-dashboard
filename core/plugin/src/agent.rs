use crate::plugin_trait::{Plugin, PluginContext, PluginError, PluginMetadata};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::any::Any;

/// Agent configuration for an agent plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub model: Option<String>,
    pub prompt_template: Option<String>,
    pub tools: Vec<String>,
    pub max_iterations: Option<usize>,
}

impl AgentConfig {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            model: None,
            prompt_template: None,
            tools: Vec::new(),
            max_iterations: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools = tools;
        self
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = Some(max);
        self
    }
}

/// Input for agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInput {
    pub task: String,
    pub context: Option<serde_json::Value>,
    pub variables: std::collections::HashMap<String, String>,
}

/// Output from agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub result: String,
    pub artifacts: Vec<serde_json::Value>,
    pub metrics: ExecutionMetrics,
}

/// Metrics collected during agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub iterations: usize,
    pub tokens_used: Option<u64>,
    pub duration_ms: u64,
}

/// Mutable context for agent execution
pub struct AgentExecutionContext {
    pub session_id: String,
    pub workflow_id: Option<String>,
    pub current_stage: Option<String>,
    pub variables: std::collections::HashMap<String, String>,
    pub artifacts: Vec<serde_json::Value>,
}

impl AgentExecutionContext {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            workflow_id: None,
            current_stage: None,
            variables: std::collections::HashMap::new(),
            artifacts: Vec::new(),
        }
    }

    pub fn with_workflow(mut self, workflow_id: impl Into<String>) -> Self {
        self.workflow_id = Some(workflow_id.into());
        self
    }

    pub fn with_stage(mut self, stage: impl Into<String>) -> Self {
        self.current_stage = Some(stage.into());
        self
    }

    pub fn set_variable(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(key.into(), value.into());
    }

    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    pub fn add_artifact(&mut self, artifact: serde_json::Value) {
        self.artifacts.push(artifact);
    }
}

/// Agent plugin error
#[derive(Debug, thiserror::Error)]
pub enum AgentPluginError {
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Context error: {0}")]
    ContextError(String),

    #[error("Tool error: {0}")]
    ToolError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Max iterations exceeded: {0}")]
    MaxIterationsExceeded(usize),
}

/// Extension trait for plugins that provide agent functionality
pub trait AgentPlugin: Plugin {
    /// Returns the agent configuration
    fn agent_config(&self) -> &AgentConfig;

    /// Execute the agent with the given input
    #[allow(async_fn_in_trait)]
    async fn execute(
        &self,
        input: &AgentInput,
        context: &mut AgentExecutionContext,
    ) -> Result<AgentOutput, AgentPluginError>;

    /// Execute with a custom system prompt override
    #[allow(async_fn_in_trait)]
    async fn execute_with_prompt(
        &self,
        input: &AgentInput,
        system_prompt: &str,
        context: &mut AgentExecutionContext,
    ) -> Result<AgentOutput, AgentPluginError>;

    /// Get the default system prompt for this agent
    fn default_system_prompt(&self) -> Option<&str> {
        None
    }
}

/// Simple agent plugin implementation
pub struct SimpleAgentPlugin {
    metadata: PluginMetadata,
    config: AgentConfig,
}

impl SimpleAgentPlugin {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        let id_str = id.into();
        let name_str = name.into();
        Self {
            metadata: PluginMetadata::new(id_str.clone(), &name_str, "1.0.0"),
            config: AgentConfig::new(id_str, name_str),
        }
    }

    pub fn with_config(mut self, config: AgentConfig) -> Self {
        self.config = config;
        self
    }
}

#[async_trait]
impl Plugin for SimpleAgentPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), PluginError> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl AgentPlugin for SimpleAgentPlugin {
    fn agent_config(&self) -> &AgentConfig {
        &self.config
    }

    async fn execute(
        &self,
        input: &AgentInput,
        context: &mut AgentExecutionContext,
    ) -> Result<AgentOutput, AgentPluginError> {
        self.execute_with_prompt(input, "", context).await
    }

    async fn execute_with_prompt(
        &self,
        input: &AgentInput,
        _system_prompt: &str,
        context: &mut AgentExecutionContext,
    ) -> Result<AgentOutput, AgentPluginError> {
        let start = std::time::Instant::now();

        // Simple implementation - in production this would call an LLM
        let result = format!(
            "Processed task: {} with {} variables",
            input.task,
            input.variables.len()
        );

        // Simulate some work
        context.add_artifact(serde_json::json!({
            "type": "result",
            "content": &result
        }));

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(AgentOutput {
            result,
            artifacts: std::mem::take(&mut context.artifacts),
            metrics: ExecutionMetrics {
                iterations: 1,
                tokens_used: None,
                duration_ms,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_agent_execution() {
        let plugin = SimpleAgentPlugin::new("test-agent", "Test Agent");
        let mut context = AgentExecutionContext::new("session-1");

        let input = AgentInput {
            task: "Test task".to_string(),
            context: None,
            variables: std::collections::HashMap::new(),
        };

        let output = plugin.execute(&input, &mut context).await.unwrap();

        assert!(output.result.contains("Test task"));
        assert_eq!(output.metrics.iterations, 1);
        assert!(!output.artifacts.is_empty());
    }

    #[tokio::test]
    async fn test_agent_with_variables() {
        let plugin = SimpleAgentPlugin::new("test-agent", "Test Agent");
        let mut context = AgentExecutionContext::new("session-1");
        context.set_variable("key", "value");

        let input = AgentInput {
            task: "Task with vars".to_string(),
            context: None,
            variables: std::collections::HashMap::new(),
        };

        let output = plugin.execute(&input, &mut context).await.unwrap();

        assert!(output.result.contains("1 variables")); // 1 variable set above
    }
}
