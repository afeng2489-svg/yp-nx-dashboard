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

pub mod traits;
pub mod registry;
pub mod cli_registry;
pub mod anthropic;
pub mod openai;
pub mod google;
pub mod ollama;
pub mod codex;
pub mod qwen;
pub mod opencode;
pub mod minimax;
pub mod claude_switch;
pub mod manager;
pub mod selector;

pub use traits::*;
pub use registry::*;
pub use cli_registry::*;
pub use anthropic::AnthropicProvider;
pub use openai::OpenAIProvider;
pub use google::GoogleProvider;
pub use ollama::{OllamaProvider, OllamaModel};
pub use codex::CodexProvider;
pub use qwen::QwenProvider;
pub use opencode::OpenCodeProvider;
pub use minimax::MiniMaxProvider;
pub use claude_switch::{ClaudeSwitchProvider, SwitchBackend, BackendConfig};
pub use manager::*;
pub use selector::*;
