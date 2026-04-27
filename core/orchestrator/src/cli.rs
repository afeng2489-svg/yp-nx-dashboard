//! Multi-CLI Orchestrator - Unified interface to multiple CLI AI tools

use crate::error::CliError;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;

/// CLI tool provider identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CliProvider {
    Claude,   // Anthropic
    Codex,    // OpenAI
    Gemini,   // Google
    Ollama,   // Local
    Groq,     // Groq
    LmStudio, // Local
}

impl CliProvider {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "claude" | "anthropic" => Some(Self::Claude),
            "codex" | "openai" => Some(Self::Codex),
            "gemini" | "google" => Some(Self::Gemini),
            "ollama" => Some(Self::Ollama),
            "groq" => Some(Self::Groq),
            "lmstudio" | "lm-studio" => Some(Self::LmStudio),
            _ => None,
        }
    }

    pub fn executable_name(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
            Self::Ollama => "ollama",
            Self::Groq => "groq",
            Self::LmStudio => "lmstudio",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Claude,
            Self::Codex,
            Self::Gemini,
            Self::Ollama,
            Self::Groq,
            Self::LmStudio,
        ]
    }
}

/// CLI execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliRequest {
    pub provider: CliProvider,
    pub prompt: String,
    pub system_prompt: Option<String>,
    pub working_dir: Option<PathBuf>,
    pub env_vars: HashMap<String, String>,
    pub timeout_secs: Option<u64>,
    pub stream: bool,
}

/// CLI execution response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliResponse {
    pub provider: CliProvider,
    pub text: String,
    pub usage: Option<CliTokenUsage>,
    pub duration_ms: u64,
    pub exit_code: i32,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliTokenUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
}

/// Streaming chunk from CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliStreamChunk {
    pub provider: CliProvider,
    pub text: String,
    pub is_final: bool,
}

/// Auto-selection strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionStrategy {
    DefaultOnly,
    TaskAware,
    RoundRobin,
    Ensemble,
    Fallback,
}

/// CLI tool configuration
#[derive(Debug, Clone)]
pub struct CliToolConfig {
    pub provider: CliProvider,
    pub executable_path: Option<String>,
    pub default_model: Option<String>,
    pub supported_models: Vec<String>,
    pub requires_auth: bool,
    pub base_url: Option<String>,
}

impl Default for CliToolConfig {
    fn default() -> Self {
        Self {
            provider: CliProvider::Claude,
            executable_path: None,
            default_model: None,
            supported_models: Vec::new(),
            requires_auth: true,
            base_url: None,
        }
    }
}

/// Manages CLI tool lifecycle and execution
pub struct CliManager {
    tools: RwLock<HashMap<CliProvider, CliToolConfig>>,
    default_provider: RwLock<CliProvider>,
    selection_strategy: RwLock<SelectionStrategy>,
}

impl CliManager {
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
            default_provider: RwLock::new(CliProvider::Claude),
            selection_strategy: RwLock::new(SelectionStrategy::DefaultOnly),
        }
    }

    /// Detect available CLI tools on the system
    pub async fn detect_available_tools(&self) -> Vec<CliProvider> {
        let mut available = Vec::new();
        for provider in CliProvider::all() {
            if self.is_tool_available(provider).await {
                available.push(provider);
            }
        }
        available
    }

    /// Check if a specific CLI tool is available
    pub async fn is_tool_available(&self, provider: CliProvider) -> bool {
        let exe_name = provider.executable_name();
        which::which(exe_name).is_ok()
            || std::path::Path::new(&format!("/usr/local/bin/{}", exe_name)).exists()
            || std::path::Path::new(&format!("/usr/bin/{}", exe_name)).exists()
    }

    /// Execute a CLI request
    pub async fn execute(&self, request: CliRequest) -> Result<CliResponse, CliError> {
        let start = Instant::now();
        let provider = request.provider;

        let exe_path = provider.executable_name().to_string();
        let mut cmd = self.build_command(&provider, &exe_path, &request)?;

        let output = cmd.output().await.map_err(|e| {
            CliError::ExecutionFailed(format!("failed to spawn {}: {}", exe_path, e))
        })?;

        let duration_ms = start.elapsed().as_millis() as u64;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CliError::ExecutionFailed(format!(
                "{} exited with {}: {}",
                exe_path,
                output.status.code().unwrap_or(-1),
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let text = self.parse_output(&provider, &stdout)?;

        Ok(CliResponse {
            provider,
            text,
            usage: None,
            duration_ms,
            exit_code: output.status.code().unwrap_or(0),
        })
    }

    /// Build provider-specific command
    fn build_command(
        &self,
        provider: &CliProvider,
        exe_path: &str,
        request: &CliRequest,
    ) -> Result<Command, CliError> {
        let mut cmd = Command::new(exe_path);

        if let Some(ref dir) = request.working_dir {
            cmd.current_dir(dir);
        }

        for (key, value) in &request.env_vars {
            cmd.env(key, value);
        }

        match provider {
            CliProvider::Claude => {
                cmd.arg("--print");
                if let Some(ref system) = request.system_prompt {
                    cmd.args(["--system-prompt", system]);
                }
                cmd.arg("--").arg(&request.prompt);
            }
            CliProvider::Gemini => {
                cmd.args(["generate", "--prompt"]).arg(&request.prompt);
                if request.stream {
                    cmd.arg("--stream");
                }
            }
            CliProvider::Ollama => {
                let model = "llama3.2";
                cmd.args(["run", model, &request.prompt]);
            }
            CliProvider::Codex => {
                cmd.args(["--print", &request.prompt]);
            }
            CliProvider::Groq => {
                cmd.args(["--prompt", &request.prompt]);
            }
            CliProvider::LmStudio => {
                cmd.args(["--prompt", &request.prompt]);
            }
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        Ok(cmd)
    }

    /// Parse provider-specific output
    fn parse_output(&self, provider: &CliProvider, stdout: &str) -> Result<String, CliError> {
        match provider {
            CliProvider::Claude => Ok(stdout.trim().to_string()),
            CliProvider::Gemini => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(stdout) {
                    Ok(parsed
                        .get("text")
                        .or_else(|| parsed.get("content"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(stdout)
                        .to_string())
                } else {
                    Ok(stdout.trim().to_string())
                }
            }
            _ => Ok(stdout.trim().to_string()),
        }
    }

    /// Set default provider
    pub fn set_default_provider(&self, provider: CliProvider) {
        *self.default_provider.write() = provider;
    }

    /// Get default provider
    pub fn get_default_provider(&self) -> CliProvider {
        *self.default_provider.read()
    }

    /// Set selection strategy
    pub fn set_selection_strategy(&self, strategy: SelectionStrategy) {
        *self.selection_strategy.write() = strategy;
    }
}

impl Default for CliManager {
    fn default() -> Self {
        Self::new()
    }
}
