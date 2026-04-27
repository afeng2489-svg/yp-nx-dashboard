//! AI Provider Traits
//!
//! AI 提供商适配器的核心 trait 定义。
//! 所有提供商必须实现这些 trait 以确保不同 AI 服务之间的接口一致性。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// CLI 类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CLI {
    Claude,
    Gemini,
    Codex,
    Qwen,
    OpenCode,
}

impl CLI {
    /// 获取 CLI 显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            CLI::Claude => "Claude",
            CLI::Gemini => "Gemini",
            CLI::Codex => "Codex",
            CLI::Qwen => "Qwen",
            CLI::OpenCode => "OpenCode",
        }
    }

    /// 获取 CLI 标识符
    pub fn identifier(&self) -> &'static str {
        match self {
            CLI::Claude => "claude",
            CLI::Gemini => "gemini",
            CLI::Codex => "codex",
            CLI::Qwen => "qwen",
            CLI::OpenCode => "opencode",
        }
    }
}

impl std::fmt::Display for CLI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.identifier())
    }
}

/// CLI 执行上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLIContext {
    /// 工作目录
    pub working_directory: Option<String>,
    /// 环境变量
    pub env_vars: std::collections::HashMap<String, String>,
    /// 超时时间（秒）
    pub timeout_secs: Option<u64>,
    /// 额外参数
    pub extra_params: std::collections::HashMap<String, String>,
}

impl Default for CLIContext {
    fn default() -> Self {
        Self {
            working_directory: None,
            env_vars: std::collections::HashMap::new(),
            timeout_secs: Some(300),
            extra_params: std::collections::HashMap::new(),
        }
    }
}

/// CLI 执行响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLIResponse {
    /// 执行输出
    pub output: String,
    /// 错误输出
    pub error: Option<String>,
    /// 退出码
    pub exit_code: i32,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
    /// 使用的 CLI
    pub cli: CLI,
}

/// 文本补全请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// 发送给模型的提示词
    pub prompt: String,
    /// 模型标识符 (如 "claude-opus-4-5", "gpt-4-turbo")
    pub model: String,
    /// 最大生成 token 数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    /// 采样温度 (0.0 - 1.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// 可选的停止序列
    #[serde(default)]
    pub stop_sequences: Vec<String>,
    /// 可选的系统提示词
    #[serde(default)]
    pub system_prompt: Option<String>,
}

fn default_max_tokens() -> usize {
    4096
}
fn default_temperature() -> f32 {
    0.7
}

/// 文本补全响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// 生成的文本
    pub text: String,
    /// 响应模型
    pub model: String,
    /// 使用的 token 数量
    pub usage: TokenUsage,
    /// 停止原因
    pub stop_reason: String,
}

/// Token 使用量信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// 消耗的输入 token 数
    pub input_tokens: usize,
    /// 生成的输出 token 数
    pub output_tokens: usize,
}

/// 嵌入请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedRequest {
    /// 要嵌入的文本
    pub texts: Vec<String>,
    /// 用于嵌入的模型
    pub model: String,
}

/// 嵌入响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedResponse {
    /// 生成的嵌入向量
    pub embeddings: Vec<Vec<f32>>,
    /// 使用的模型
    pub model: String,
    /// Token 使用量
    pub usage: TokenUsage,
}

/// 聊天补全请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// 发送的消息列表
    pub messages: Vec<ChatMessage>,
    /// 模型标识符
    pub model: String,
    /// 最大生成 token 数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    /// 采样温度
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

/// 单条聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 角色 (system, user, assistant)
    pub role: String,
    /// 消息内容
    pub content: String,
}

/// 聊天补全响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// 生成的消息
    pub message: ChatMessage,
    /// 使用的模型
    pub model: String,
    /// Token 使用量
    pub usage: TokenUsage,
    /// 停止原因
    pub stop_reason: String,
}

/// AI 提供商核心 trait
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// 获取提供商名称 (如 "anthropic", "openai")
    fn provider_name(&self) -> &str;

    /// 获取支持的模型列表
    fn supported_models(&self) -> Vec<&str>;

    /// 检查模型是否支持
    fn supports_model(&self, model: &str) -> bool {
        self.supported_models().contains(&model)
    }

    /// 执行文本补全
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, AIError>;

    /// 执行聊天补全
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AIError>;

    /// 生成嵌入向量
    async fn embed(&self, request: EmbedRequest) -> Result<EmbedResponse, AIError>;

    /// 获取此提供商的默认模型
    fn default_model(&self) -> &str;

    /// 获取支持的 CLI 列表
    fn supported_clis(&self) -> Vec<CLI> {
        vec![]
    }

    /// 检查 CLI 是否支持
    fn supports_cli(&self, cli: CLI) -> bool {
        self.supported_clis().contains(&cli)
    }

    /// 使用指定 CLI 执行
    async fn execute_with_cli(
        &self,
        prompt: &str,
        cli: CLI,
        context: &CLIContext,
    ) -> Result<CLIResponse, AIError> {
        let _ = (prompt, cli, context);
        Err(AIError::InvalidRequest(
            "该提供商不支持 CLI 执行".to_string(),
        ))
    }
}

/// AI 操作可能发生的错误
#[derive(Debug, thiserror::Error)]
pub enum AIError {
    #[error("认证失败: {0}")]
    Authentication(String),

    #[error("请求频率超限: {0}")]
    RateLimit(String),

    #[error("模型不支持: {0}")]
    ModelNotSupported(String),

    #[error("无效请求: {0}")]
    InvalidRequest(String),

    #[error("提供商错误: {0}")]
    Provider(String),

    #[error("网络错误: {0}")]
    Network(String),

    #[error("解析错误: {0}")]
    Parse(String),

    #[error("超时: {0}")]
    Timeout(String),
}

impl AIError {
    /// 检查错误是否可重试
    pub fn is_retryable(&self) -> bool {
        matches!(self, AIError::RateLimit(_) | AIError::Timeout(_))
    }

    /// 检查是否是认证相关错误 (不可重试)
    pub fn is_auth_error(&self) -> bool {
        matches!(self, AIError::Authentication(_) | AIError::RateLimit(_))
    }
}

use std::fmt;
impl fmt::Display for CompletionRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CompletionRequest(model={}, max_tokens={})",
            self.model, self.max_tokens
        )
    }
}
