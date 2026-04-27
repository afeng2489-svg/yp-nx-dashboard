//! NexusFlow AI Provider Bridge
//!
//! Unified abstraction layer for multiple AI providers:
//! - Anthropic (Claude)
//! - OpenAI (GPT-4)
//! - Google (Gemini)
//! - Ollama (Local models)
//! - Codex (Code generation)
//! - Qwen (Chinese language)
//! - OpenCode (Open source projects)
//! - MiniMax (中国大模型)

pub mod anthropic;
pub mod claude_switch;
pub mod cli_registry;
pub mod codex;
pub mod google;
pub mod manager;
pub mod minimax;
pub mod ollama;
pub mod openai;
pub mod opencode;
pub mod qwen;
pub mod registry;
pub mod selector;
pub mod traits;

pub use anthropic::AnthropicProvider;
pub use claude_switch::{BackendConfig, ClaudeSwitchProvider, SwitchBackend};
pub use cli_registry::*;
pub use codex::CodexProvider;
pub use google::GoogleProvider;
pub use manager::*;
pub use minimax::MiniMaxProvider;
pub use ollama::{OllamaModel, OllamaProvider};
pub use openai::OpenAIProvider;
pub use opencode::OpenCodeProvider;
pub use qwen::QwenProvider;
pub use registry::*;
pub use selector::*;
pub use traits::*;
