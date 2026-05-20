//! Claude Code CLI Provider
//!
//! Uses the locally installed Claude Code CLI (`claude`) for AI capabilities.
//! No API key required — the CLI uses your local Claude Code authentication.
//!
//! The CLI path is resolved via:
//! 1. `CLAUDE_CLI_PATH_OVERRIDE` env var (set by nx_api during startup)
//! 2. `which claude` / `where claude` fallback

use async_trait::async_trait;
use std::process::Command as SyncCommand;
use tokio::process::Command as AsyncCommand;

use super::{
    AIError, AIProvider, ChatMessage, ChatRequest, ChatResponse, CompletionRequest,
    CompletionResponse, EmbedRequest, EmbedResponse, TokenUsage,
};

#[derive(Debug, Clone)]
pub struct ClaudeCliProvider {
    cli_path: Option<String>,
}

impl ClaudeCliProvider {
    pub fn new() -> Self {
        Self {
            cli_path: Self::detect_cli(),
        }
    }

    fn detect_cli() -> Option<String> {
        if let Ok(path) = std::env::var("CLAUDE_CLI_PATH_OVERRIDE") {
            if !path.is_empty() && std::path::Path::new(&path).exists() {
                tracing::info!(
                    "[ClaudeCliProvider] using CLAUDE_CLI_PATH_OVERRIDE: {}",
                    path
                );
                return Some(path);
            }
        }

        let names: &[&str] = if cfg!(target_os = "windows") {
            &["claude.cmd", "claude.exe", "claude"]
        } else {
            &["claude"]
        };

        let which_cmd = if cfg!(target_os = "windows") {
            "where"
        } else {
            "which"
        };

        for name in names {
            if let Ok(output) = SyncCommand::new(which_cmd).arg(name).output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() && std::path::Path::new(&path).exists() {
                        tracing::info!("[ClaudeCliProvider] found cli at: {}", path);
                        return Some(path);
                    }
                }
            }
        }

        tracing::warn!("[ClaudeCliProvider] Claude Code CLI not found");
        None
    }

    pub fn is_available(&self) -> bool {
        self.cli_path.is_some()
    }

    pub fn cli_path(&self) -> Option<&str> {
        self.cli_path.as_deref()
    }

    async fn run_cli(&self, prompt: &str, timeout_secs: u64) -> Result<String, AIError> {
        let cli = self.cli_path.as_ref().ok_or_else(|| {
            AIError::Provider(
                "Claude Code CLI not found. Install: npm install -g @anthropic-ai/claude-code"
                    .to_string(),
            )
        })?;

        let result = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), async {
            let mut cmd = AsyncCommand::new(cli);
            cmd.args([
                "-p",
                "--dangerously-skip-permissions",
                "--no-session-persistence",
                prompt,
            ]);

            let output = cmd
                .output()
                .await
                .map_err(|e| AIError::Provider(format!("Failed to execute Claude CLI: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(AIError::Provider(format!(
                    "Claude CLI error ({}): {}",
                    output.status, stderr
                )));
            }

            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        })
        .await;

        match result {
            Ok(r) => r,
            Err(_) => Err(AIError::Timeout("Claude CLI timed out".to_string())),
        }
    }
}

impl Default for ClaudeCliProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AIProvider for ClaudeCliProvider {
    fn provider_name(&self) -> &str {
        "claude-cli"
    }

    fn supported_models(&self) -> Vec<&str> {
        vec!["claude-local", "claude-cli-default"]
    }

    fn default_model(&self) -> &str {
        "claude-local"
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, AIError> {
        let mut prompt = String::new();
        if let Some(sys) = &request.system_prompt {
            prompt.push_str(&format!("System: {}\n\n", sys));
        }
        prompt.push_str(&request.prompt);

        let output = self.run_cli(&prompt, 300).await?;

        Ok(CompletionResponse {
            text: output,
            model: request.model,
            usage: TokenUsage {
                input_tokens: prompt.len() / 4,
                output_tokens: 0,
            },
            stop_reason: "stop".to_string(),
        })
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AIError> {
        let mut prompt = String::new();
        for msg in &request.messages {
            match msg.role.as_str() {
                "system" => {
                    prompt.push_str(&format!("<system>\n{}\n</system>\n\n", msg.content));
                }
                "user" => {
                    prompt.push_str(&format!("<user>\n{}\n</user>\n\n", msg.content));
                }
                "assistant" => {
                    prompt.push_str(&format!("<assistant>\n{}\n</assistant>\n\n", msg.content));
                }
                _ => {
                    prompt.push_str(&format!("{}\n\n", msg.content));
                }
            }
        }

        let output = self.run_cli(prompt.trim(), 300).await?;

        Ok(ChatResponse {
            message: ChatMessage {
                role: "assistant".to_string(),
                content: output,
            },
            model: request.model,
            usage: TokenUsage {
                input_tokens: prompt.len() / 4,
                output_tokens: 0,
            },
            stop_reason: "stop".to_string(),
        })
    }

    async fn embed(&self, _request: EmbedRequest) -> Result<EmbedResponse, AIError> {
        Err(AIError::InvalidRequest(
            "Claude CLI provider does not support embeddings".to_string(),
        ))
    }
}
