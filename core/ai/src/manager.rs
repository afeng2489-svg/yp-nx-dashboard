//! AI Model Manager
//!
//! 统一管理多种 AI 大模型的配置和调用。
//! 支持：Anthropic (Claude), OpenAI (GPT-4), Google (Gemini), Ollama (本地模型)
//! CLI 支持：Claude, Gemini, Codex, Qwen, OpenCode

use parking_lot::RwLock as SyncRwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::claude_switch::{BackendConfig, ClaudeSwitchProvider, SwitchBackend};
use super::cli_registry::{CLIRegistry, SemanticCLISelector};
use super::registry::AIProviderRegistry;
use super::selector::{GlobalModelSelector, ModelInfo};
use super::{
    AIError, AIProvider, CLIContext, CLIResponse, ChatMessage, ChatRequest, ChatResponse,
    CompletionRequest, CompletionResponse, CLI,
};

/// AI 提供商类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Anthropic,
    OpenAI,
    Google,
    Ollama,
    Codex,
    Qwen,
    OpenCode,
    MiniMax,
    ClaudeSwitch, // Claude Switch - 用 Claude 格式调用其他后端
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Google => write!(f, "google"),
            ProviderType::Ollama => write!(f, "ollama"),
            ProviderType::Codex => write!(f, "codex"),
            ProviderType::Qwen => write!(f, "qwen"),
            ProviderType::OpenCode => write!(f, "opencode"),
            ProviderType::MiniMax => write!(f, "minimax"),
            ProviderType::ClaudeSwitch => write!(f, "claude-switch"),
        }
    }
}

/// 模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// 模型标识符 (如 "claude-opus-4-5", "gpt-4-turbo")
    pub model_id: String,
    /// 提供商类型
    pub provider: ProviderType,
    /// 最大生成 token 数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    /// 采样温度 (0.0 - 1.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// 停止序列
    #[serde(default)]
    pub stop_sequences: Vec<String>,
    /// 额外参数
    #[serde(default)]
    pub extra_params: HashMap<String, String>,
}

fn default_max_tokens() -> usize {
    4096
}
fn default_temperature() -> f32 {
    0.7
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_id: "claude-sonnet-4-5".to_string(),
            provider: ProviderType::Anthropic,
            max_tokens: 4096,
            temperature: 0.7,
            stop_sequences: vec![],
            extra_params: HashMap::new(),
        }
    }
}

/// API 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIConfig {
    /// API 密钥
    #[serde(default)]
    pub api_key: String,
    /// API 基础 URL (用于自定义端点或代理)
    #[serde(default)]
    pub base_url: String,
    /// 组织 ID (用于 OpenAI)
    #[serde(default)]
    pub organization_id: String,
    /// 超时秒数
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 {
    120
}

impl Default for APIConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: String::new(),
            organization_id: String::new(),
            timeout_secs: 120,
        }
    }
}

/// AI 管理器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIManagerConfig {
    /// 默认模型配置
    #[serde(default)]
    pub default_model: ModelConfig,
    /// API 配置
    #[serde(default)]
    pub api_config: HashMap<ProviderType, APIConfig>,
    /// 是否启用提供商
    #[serde(default)]
    pub enabled_providers: Vec<ProviderType>,
}

impl Default for AIManagerConfig {
    fn default() -> Self {
        Self {
            default_model: ModelConfig::default(),
            api_config: HashMap::new(),
            enabled_providers: vec![
                ProviderType::Anthropic,
                ProviderType::OpenAI,
                ProviderType::Google,
                ProviderType::Ollama,
                ProviderType::Codex,
                ProviderType::Qwen,
                ProviderType::OpenCode,
                ProviderType::MiniMax,
            ],
        }
    }
}

/// AI 请求参数
#[derive(Debug, Clone)]
pub struct AIRequestParams {
    /// 模型配置
    pub model: ModelConfig,
    /// 系统提示词
    pub system_prompt: Option<String>,
    /// 用户消息
    pub user_message: String,
    /// 聊天历史
    pub chat_history: Vec<ChatMessage>,
    /// 是否使用流式输出
    pub stream: bool,
}

impl AIRequestParams {
    /// 创建简单的文本补全请求
    pub fn completion(model: ModelConfig, prompt: String) -> Self {
        Self {
            model,
            system_prompt: None,
            user_message: prompt,
            chat_history: vec![],
            stream: false,
        }
    }

    /// 创建聊天请求
    pub fn chat(model: ModelConfig, user_message: String) -> Self {
        Self {
            model,
            system_prompt: None,
            user_message,
            chat_history: vec![],
            stream: false,
        }
    }

    /// 设置系统提示词
    pub fn with_system_prompt(mut self, prompt: String) -> Self {
        self.system_prompt = Some(prompt);
        self
    }

    /// 添加聊天历史
    pub fn with_history(mut self, history: Vec<ChatMessage>) -> Self {
        self.chat_history = history;
        self
    }

    /// 启用流式输出
    pub fn with_stream(mut self) -> Self {
        self.stream = true;
        self
    }
}

/// AI 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIResponse {
    /// 生成的文本
    pub text: String,
    /// 使用的模型
    pub model: String,
    /// 提供商
    pub provider: String,
    /// 输入 token 数
    pub input_tokens: usize,
    /// 输出 token 数
    pub output_tokens: usize,
    /// 停止原因
    pub stop_reason: String,
}

/// 模型刷新状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRefreshStatus {
    /// 是否需要刷新
    pub needs_refresh: bool,
    /// 距离下次刷新的秒数
    pub seconds_until_refresh: u64,
    /// 上次刷新时间
    pub last_refresh_time: String,
}

/// AI 模型管理器
pub struct AIModelManager {
    /// 提供商注册表
    registry: Arc<AIProviderRegistry>,
    /// CLI 注册表
    cli_registry: Arc<CLIRegistry>,
    /// 当前配置
    config: SyncRwLock<AIManagerConfig>,
    /// 模型配置缓存
    model_configs: SyncRwLock<HashMap<String, ModelConfig>>,
    /// 全局模型选择器
    selector: GlobalModelSelector,
    /// Claude Switch 提供商 (使用 tokio::sync::RwLock 以支持 async)
    claude_switch: RwLock<Option<ClaudeSwitchProvider>>,
    /// 当前激活的后端
    active_backend: RwLock<SwitchBackend>,
    /// 是否已初始化
    initialized: SyncRwLock<bool>,
}

impl AIModelManager {
    /// 创建新的 AI 管理器
    pub fn new() -> Self {
        let registry = Arc::new(AIProviderRegistry::new());
        let selector = GlobalModelSelector::new(registry.clone());
        let manager = Self {
            registry,
            cli_registry: Arc::new(CLIRegistry::new()),
            config: SyncRwLock::new(AIManagerConfig::default()),
            model_configs: SyncRwLock::new(HashMap::new()),
            selector,
            claude_switch: RwLock::new(None),
            active_backend: RwLock::new(SwitchBackend::MiniMax),
            initialized: SyncRwLock::new(false),
        };
        // 注册默认模型（无需 API key 即可显示）
        manager.register_default_models();
        *manager.initialized.write() = true;
        manager
    }

    /// 从配置文件加载
    pub fn from_config(config: AIManagerConfig) -> Self {
        let registry = Arc::new(AIProviderRegistry::new());
        let selector = GlobalModelSelector::new(registry.clone());
        let manager = Self {
            registry,
            cli_registry: Arc::new(CLIRegistry::new()),
            config: SyncRwLock::new(AIManagerConfig::default()),
            model_configs: SyncRwLock::new(HashMap::new()),
            selector,
            claude_switch: RwLock::new(None),
            active_backend: RwLock::new(SwitchBackend::MiniMax),
            initialized: SyncRwLock::new(false),
        };
        manager.update_config(config);
        *manager.initialized.write() = true;
        manager
    }

    /// 更新配置
    pub fn update_config(&self, config: AIManagerConfig) {
        *self.config.write() = config.clone();

        // 清除旧的模型配置缓存
        self.model_configs.write().clear();

        // 重新初始化提供商
        self.initialize_providers(&config);
    }

    /// 获取当前配置
    pub fn get_config(&self) -> AIManagerConfig {
        self.config.read().clone()
    }

    /// 初始化提供商
    fn initialize_providers(&self, config: &AIManagerConfig) {
        for provider_type in &config.enabled_providers {
            let api_config = config.api_config.get(provider_type);

            match provider_type {
                ProviderType::Anthropic => {
                    if let Some(cfg) = api_config {
                        if !cfg.api_key.is_empty() {
                            let provider = super::AnthropicProvider::with_model(
                                cfg.api_key.clone(),
                                &config.default_model.model_id,
                            );
                            self.registry.register(Arc::new(provider));
                        }
                    }
                }
                ProviderType::OpenAI => {
                    if let Some(cfg) = api_config {
                        if !cfg.api_key.is_empty() {
                            let provider = super::OpenAIProvider::new(cfg.api_key.clone());
                            self.registry.register(Arc::new(provider));
                        }
                    }
                }
                ProviderType::Google => {
                    if let Some(cfg) = api_config {
                        if !cfg.api_key.is_empty() {
                            let provider = super::GoogleProvider::new(cfg.api_key.clone());
                            self.registry.register(Arc::new(provider));
                        }
                    }
                }
                ProviderType::Ollama => {
                    let provider = super::OllamaProvider::new();
                    self.registry.register(Arc::new(provider));
                }
                ProviderType::Codex => {
                    if let Some(cfg) = api_config {
                        if !cfg.api_key.is_empty() {
                            let provider = super::CodexProvider::new(cfg.api_key.clone());
                            self.registry.register(Arc::new(provider));
                        }
                    }
                }
                ProviderType::Qwen => {
                    if let Some(cfg) = api_config {
                        if !cfg.api_key.is_empty() {
                            let provider = super::QwenProvider::new(cfg.api_key.clone());
                            self.registry.register(Arc::new(provider));
                        }
                    }
                }
                ProviderType::MiniMax => {
                    if let Some(cfg) = api_config {
                        if !cfg.api_key.is_empty() {
                            let provider = super::MiniMaxProvider::new(cfg.api_key.clone());
                            self.registry.register(Arc::new(provider));
                        }
                    }
                }
                ProviderType::OpenCode => {
                    if let Some(cfg) = api_config {
                        if !cfg.api_key.is_empty() {
                            let provider = super::OpenCodeProvider::new(cfg.api_key.clone());
                            self.registry.register(Arc::new(provider));
                        }
                    }
                }
                ProviderType::ClaudeSwitch => {
                    // Claude Switch 通过专门的方法配置，不在这里处理
                }
            }
        }

        // 设置默认提供商
        let default_provider = match config.default_model.provider {
            ProviderType::Anthropic => "anthropic",
            ProviderType::OpenAI => "openai",
            ProviderType::Google => "google",
            ProviderType::Ollama => "ollama",
            ProviderType::Codex => "codex",
            ProviderType::Qwen => "qwen",
            ProviderType::OpenCode => "opencode",
            ProviderType::MiniMax => "minimax",
            ProviderType::ClaudeSwitch => "claude-switch",
        };
        let _ = self.registry.set_default(default_provider);

        // 缓存模型配置并注册到选择器
        let model_id = config.default_model.model_id.clone();
        self.model_configs
            .write()
            .insert(model_id.clone(), config.default_model.clone());

        // 注册模型到全局选择器
        self.selector.register_model(ModelInfo {
            model_id: model_id.clone(),
            provider: default_provider.to_string(),
            display_name: format!("{:?} Default", config.default_model.provider),
            description: format!("{:?} 模型", config.default_model.provider),
            supports_chat: true,
            supports_completion: true,
            is_default: true,
        });

        // 设置默认模型为选定模型
        self.selector.set_selected_model(&model_id).ok();
    }

    /// 注册新的模型配置
    pub fn register_model(&self, config: ModelConfig) {
        self.model_configs
            .write()
            .insert(config.model_id.clone(), config.clone());
    }

    /// 获取模型配置
    pub fn get_model_config(&self, model_id: &str) -> Option<ModelConfig> {
        self.model_configs.read().get(model_id).cloned()
    }

    /// 列出所有可用的模型
    pub fn list_models(&self) -> Vec<ModelConfig> {
        self.model_configs.read().values().cloned().collect()
    }

    /// 列出所有可用的提供商
    pub fn list_providers(&self) -> Vec<String> {
        self.registry.list_providers()
    }

    /// 执行补全请求
    pub async fn complete(&self, params: AIRequestParams) -> Result<AIResponse, AIError> {
        let request = CompletionRequest {
            prompt: params.user_message,
            model: params.model.model_id.clone(),
            max_tokens: params.model.max_tokens,
            temperature: params.model.temperature,
            stop_sequences: params.model.stop_sequences.clone(),
            system_prompt: params.system_prompt,
        };

        let response = self.registry.complete(request).await?;

        Ok(AIResponse {
            text: response.text,
            model: response.model,
            provider: self.config.read().default_model.provider.to_string(),
            input_tokens: response.usage.input_tokens,
            output_tokens: response.usage.output_tokens,
            stop_reason: response.stop_reason,
        })
    }

    /// 执行聊天请求
    pub async fn chat(&self, params: AIRequestParams) -> Result<AIResponse, AIError> {
        let mut messages: Vec<ChatMessage> = vec![];

        // 添加系统提示词
        if let Some(system_prompt) = &params.system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            });
        }

        // 添加聊天历史
        messages.extend(params.chat_history);

        // 添加当前用户消息
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: params.user_message,
        });

        let request = ChatRequest {
            messages,
            model: params.model.model_id.clone(),
            max_tokens: params.model.max_tokens,
            temperature: params.model.temperature,
        };

        let response = self.registry.chat(request).await?;

        Ok(AIResponse {
            text: response.message.content,
            model: response.model,
            provider: self.config.read().default_model.provider.to_string(),
            input_tokens: response.usage.input_tokens,
            output_tokens: response.usage.output_tokens,
            stop_reason: response.stop_reason,
        })
    }

    /// 根据模型 ID 选择提供商并执行
    pub async fn call(&self, model_id: &str, prompt: String) -> Result<AIResponse, AIError> {
        // 首先尝试从 model_configs 获取完整配置
        let config = self
            .model_configs
            .read()
            .get(model_id)
            .cloned()
            .unwrap_or_else(|| {
                // 如果没有专门配置，使用默认模型的配置
                // 路由会通过 registry 处理
                self.config.read().default_model.clone()
            });

        self.complete(AIRequestParams::completion(config, prompt))
            .await
    }

    /// 获取提供商注册表
    pub fn registry(&self) -> Arc<AIProviderRegistry> {
        self.registry.clone()
    }

    /// 获取 CLI 注册表
    pub fn cli_registry(&self) -> Arc<CLIRegistry> {
        self.cli_registry.clone()
    }

    /// 获取语义选择器
    pub fn semantic_selector(&self) -> SemanticCLISelector {
        SemanticCLISelector::new(self.cli_registry.clone())
    }

    /// 使用 CLI 执行提示词
    pub async fn execute_with_cli(
        &self,
        prompt: &str,
        cli: CLI,
        context: &CLIContext,
    ) -> Result<CLIResponse, AIError> {
        // 找到支持该 CLI 的提供商
        for provider in self.registry.get_all_providers() {
            if provider.supports_cli(cli) {
                return provider.execute_with_cli(prompt, cli, context).await;
            }
        }

        Err(AIError::InvalidRequest(format!(
            "没有提供商支持 CLI: {:?}",
            cli
        )))
    }

    /// 自动选择 CLI 并执行
    pub async fn execute_auto(
        &self,
        prompt: &str,
        context: &CLIContext,
    ) -> Result<CLIResponse, AIError> {
        let cli = self.cli_registry.select_cli_for_prompt(prompt);
        self.execute_with_cli(prompt, cli, context).await
    }

    /// 列出所有可用的 CLI
    pub fn list_available_clis(&self) -> Vec<CLI> {
        self.cli_registry.available_clis()
    }

    /// 获取 CLI 能力
    pub fn get_cli_capability(&self, cli: CLI) -> Option<super::CLICapability> {
        self.cli_registry.get_capability(cli)
    }

    /// 注册默认模型（预定义，无需 API key 即可显示）
    fn register_default_models(&self) {
        use super::selector::ModelInfo;

        let default_models = vec![
            // Anthropic
            ModelInfo {
                model_id: "claude-opus-4-5".to_string(),
                provider: "claude-official".to_string(),
                display_name: "Claude Opus 4.5".to_string(),
                description: "Most intelligent model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "claude-sonnet-4-5".to_string(),
                provider: "claude-official".to_string(),
                display_name: "Claude Sonnet 4.5".to_string(),
                description: "Balanced model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: true,
            },
            ModelInfo {
                model_id: "claude-haiku-4-5".to_string(),
                provider: "claude-official".to_string(),
                display_name: "Claude Haiku 4.5".to_string(),
                description: "Fastest model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // DeepSeek
            ModelInfo {
                model_id: "deepseek-chat".to_string(),
                provider: "deepseek".to_string(),
                display_name: "DeepSeek Chat".to_string(),
                description: "DeepSeek general model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "deepseek-coder".to_string(),
                provider: "deepseek".to_string(),
                display_name: "DeepSeek Coder".to_string(),
                description: "DeepSeek code model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Zhipu GLM
            ModelInfo {
                model_id: "glm-4".to_string(),
                provider: "zhipu-glm".to_string(),
                display_name: "GLM-4".to_string(),
                description: "Zhipu GLM-4".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "glm-4-plus".to_string(),
                provider: "zhipu-glm".to_string(),
                display_name: "GLM-4 Plus".to_string(),
                description: "Zhipu GLM-4 Plus".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "glm-4-air".to_string(),
                provider: "zhipu-glm".to_string(),
                display_name: "GLM-4 Air".to_string(),
                description: "Zhipu GLM-4 Air".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "glm-4-airx".to_string(),
                provider: "zhipu-glm".to_string(),
                display_name: "GLM-4 AirX".to_string(),
                description: "Zhipu GLM-4 AirX".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Bailian
            ModelInfo {
                model_id: "qwen-plus".to_string(),
                provider: "bailian".to_string(),
                display_name: "Bailian Qwen Plus".to_string(),
                description: "Alibaba Qwen Plus".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "qwen-turbo".to_string(),
                provider: "bailian".to_string(),
                display_name: "Bailian Qwen Turbo".to_string(),
                description: "Alibaba Qwen Turbo".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "qwen-max".to_string(),
                provider: "bailian".to_string(),
                display_name: "Bailian Qwen Max".to_string(),
                description: "Alibaba Qwen Max".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "bailian-coder".to_string(),
                provider: "bailian".to_string(),
                display_name: "Bailian For Coding".to_string(),
                description: "Alibaba code model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Kimi
            ModelInfo {
                model_id: "moonshot-v1-8k".to_string(),
                provider: "kimi".to_string(),
                display_name: "Kimi 8K".to_string(),
                description: "Moonshot 8K context".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "moonshot-v1-32k".to_string(),
                provider: "kimi".to_string(),
                display_name: "Kimi 32K".to_string(),
                description: "Moonshot 32K context".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "moonshot-v1-128k".to_string(),
                provider: "kimi".to_string(),
                display_name: "Kimi 128K".to_string(),
                description: "Moonshot 128K context".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // StepFun
            ModelInfo {
                model_id: "step-1v-8k".to_string(),
                provider: "stepfun".to_string(),
                display_name: "Step-1V 8K".to_string(),
                description: "StepFun vision model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "step-1v-32k".to_string(),
                provider: "stepfun".to_string(),
                display_name: "Step-1V 32K".to_string(),
                description: "StepFun vision 32K".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "step-1-flash".to_string(),
                provider: "stepfun".to_string(),
                display_name: "Step-1 Flash".to_string(),
                description: "StepFun fast model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // KAT-Coder
            ModelInfo {
                model_id: "kat-coder".to_string(),
                provider: "kat-coder".to_string(),
                display_name: "KAT-Coder".to_string(),
                description: "KAT Coder".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Longcat
            ModelInfo {
                model_id: "longcat-chat".to_string(),
                provider: "longcat".to_string(),
                display_name: "Longcat Chat".to_string(),
                description: "Longcat chat model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // MiniMax
            ModelInfo {
                model_id: "MiniMax-M2.7".to_string(),
                provider: "minimax".to_string(),
                display_name: "MiniMax-M2.7".to_string(),
                description: "MiniMax M2.7 latest model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "abab6-chat".to_string(),
                provider: "minimax".to_string(),
                display_name: "MiniMax Chat".to_string(),
                description: "MiniMax chat model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "abab6-gs".to_string(),
                provider: "minimax".to_string(),
                display_name: "MiniMax GS".to_string(),
                description: "MiniMax GS model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // DouBaoSeed
            ModelInfo {
                model_id: "doubao-seed".to_string(),
                provider: "doubao".to_string(),
                display_name: "DouBao Seed".to_string(),
                description: "DouBao seed model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // BaiLing
            ModelInfo {
                model_id: "bailing-chat".to_string(),
                provider: "bailing".to_string(),
                display_name: "BaiLing Chat".to_string(),
                description: "BaiLing chat model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Xiaomi MiMo
            ModelInfo {
                model_id: "mimo-chat".to_string(),
                provider: "xiaomi".to_string(),
                display_name: "MiMo Chat".to_string(),
                description: "Xiaomi MiMo".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // ModelScope
            ModelInfo {
                model_id: "modelscope-chat".to_string(),
                provider: "modelscope".to_string(),
                display_name: "ModelScope Chat".to_string(),
                description: "ModelScope chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // AiHubMix
            ModelInfo {
                model_id: "aihubmix-chat".to_string(),
                provider: "aihubmix".to_string(),
                display_name: "AiHubMix Chat".to_string(),
                description: "AiHubMix chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // SiliconFlow
            ModelInfo {
                model_id: "siliconflow-chat".to_string(),
                provider: "siliconflow".to_string(),
                display_name: "SiliconFlow Chat".to_string(),
                description: "SiliconFlow chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // OpenRouter
            ModelInfo {
                model_id: "openrouter-chat".to_string(),
                provider: "openrouter".to_string(),
                display_name: "OpenRouter Chat".to_string(),
                description: "OpenRouter chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Novita AI
            ModelInfo {
                model_id: "novita-chat".to_string(),
                provider: "novita".to_string(),
                display_name: "Novita AI Chat".to_string(),
                description: "Novita AI chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Nvidia
            ModelInfo {
                model_id: "nvidia-chat".to_string(),
                provider: "nvidia".to_string(),
                display_name: "Nvidia Chat".to_string(),
                description: "Nvidia NIM".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // PackyCode
            ModelInfo {
                model_id: "packycode-chat".to_string(),
                provider: "packycode".to_string(),
                display_name: "PackyCode Chat".to_string(),
                description: "PackyCode chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Cubence
            ModelInfo {
                model_id: "cubence-chat".to_string(),
                provider: "cubence".to_string(),
                display_name: "Cubence Chat".to_string(),
                description: "Cubence chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // AIGoCode
            ModelInfo {
                model_id: "aigocode-chat".to_string(),
                provider: "aigocode".to_string(),
                display_name: "AIGoCode Chat".to_string(),
                description: "AIGoCode chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // RightCode
            ModelInfo {
                model_id: "rightcode-chat".to_string(),
                provider: "rightcode".to_string(),
                display_name: "RightCode Chat".to_string(),
                description: "RightCode chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // AICodeMirror
            ModelInfo {
                model_id: "aicodemirror-chat".to_string(),
                provider: "aicodemirror".to_string(),
                display_name: "AICodeMirror Chat".to_string(),
                description: "AICodeMirror chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // AICoding
            ModelInfo {
                model_id: "aicoding-chat".to_string(),
                provider: "aicoding".to_string(),
                display_name: "AICoding Chat".to_string(),
                description: "AICoding chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // CrazyRouter
            ModelInfo {
                model_id: "crazyrouter-chat".to_string(),
                provider: "crazyrouter".to_string(),
                display_name: "CrazyRouter Chat".to_string(),
                description: "CrazyRouter chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // SSSAiCode
            ModelInfo {
                model_id: "sssaicode-chat".to_string(),
                provider: "sssaicode".to_string(),
                display_name: "SSSAiCode Chat".to_string(),
                description: "SSSAiCode chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Micu
            ModelInfo {
                model_id: "micu-chat".to_string(),
                provider: "micu".to_string(),
                display_name: "Micu Chat".to_string(),
                description: "Micu chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // X-Code API
            ModelInfo {
                model_id: "xcodeapi-chat".to_string(),
                provider: "xcodeapi".to_string(),
                display_name: "X-Code API Chat".to_string(),
                description: "X-Code API chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // CTok.ai
            ModelInfo {
                model_id: "ctok-chat".to_string(),
                provider: "ctok".to_string(),
                display_name: "CTok.ai Chat".to_string(),
                description: "CTok.ai chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // GitHub Copilot
            ModelInfo {
                model_id: "github-copilot".to_string(),
                provider: "github-copilot".to_string(),
                display_name: "GitHub Copilot".to_string(),
                description: "GitHub Copilot".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // AWS Bedrock
            ModelInfo {
                model_id: "bedrock-claude".to_string(),
                provider: "aws-bedrock".to_string(),
                display_name: "Claude on Bedrock".to_string(),
                description: "AWS Bedrock Claude".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // OpenAI (original)
            ModelInfo {
                model_id: "gpt-4o".to_string(),
                provider: "openai".to_string(),
                display_name: "GPT-4o".to_string(),
                description: "OpenAI GPT-4o".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "gpt-4-turbo".to_string(),
                provider: "openai".to_string(),
                display_name: "GPT-4 Turbo".to_string(),
                description: "OpenAI GPT-4 Turbo".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Google
            ModelInfo {
                model_id: "gemini-1.5-pro".to_string(),
                provider: "google".to_string(),
                display_name: "Gemini 1.5 Pro".to_string(),
                description: "Google Gemini 1.5 Pro".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "gemini-1.5-flash".to_string(),
                provider: "google".to_string(),
                display_name: "Gemini 1.5 Flash".to_string(),
                description: "Google Gemini 1.5 Flash".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Ollama
            ModelInfo {
                model_id: "llama3".to_string(),
                provider: "ollama".to_string(),
                display_name: "Llama 3".to_string(),
                description: "Meta Llama 3".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "codellama".to_string(),
                provider: "ollama".to_string(),
                display_name: "Code Llama".to_string(),
                description: "Code Llama".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
        ];

        for model in default_models {
            self.selector.register_model(model);
        }
    }

    /// 检查并自动刷新模型列表（如果距离上次刷新超过24小时）
    fn check_and_refresh_models(&self) {
        if self.selector.needs_refresh() {
            self.refresh_models();
        }
    }

    /// 手动刷新模型列表
    /// 返回刷新前后的模型数量
    pub fn refresh_models(&self) -> (usize, usize) {
        let (before, after) = self.selector.refresh_models(|selector| {
            // 重新注册所有默认模型
            self.register_default_models_internal(selector);
        });
        tracing::info!("模型列表已刷新: {} -> {} 个模型", before, after);
        (before, after)
    }

    /// 内部方法: 注册默认模型到指定的选择器
    fn register_default_models_internal(&self, selector: &GlobalModelSelector) {
        use super::selector::ModelInfo;

        let default_models = vec![
            // Anthropic
            ModelInfo {
                model_id: "claude-opus-4-5".to_string(),
                provider: "claude-official".to_string(),
                display_name: "Claude Opus 4.5".to_string(),
                description: "Most intelligent model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "claude-sonnet-4-5".to_string(),
                provider: "claude-official".to_string(),
                display_name: "Claude Sonnet 4.5".to_string(),
                description: "Balanced model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: true,
            },
            ModelInfo {
                model_id: "claude-haiku-4-5".to_string(),
                provider: "claude-official".to_string(),
                display_name: "Claude Haiku 4.5".to_string(),
                description: "Fastest model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // DeepSeek
            ModelInfo {
                model_id: "deepseek-chat".to_string(),
                provider: "deepseek".to_string(),
                display_name: "DeepSeek Chat".to_string(),
                description: "DeepSeek general model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "deepseek-coder".to_string(),
                provider: "deepseek".to_string(),
                display_name: "DeepSeek Coder".to_string(),
                description: "DeepSeek code model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Zhipu GLM
            ModelInfo {
                model_id: "glm-4".to_string(),
                provider: "zhipu-glm".to_string(),
                display_name: "GLM-4".to_string(),
                description: "Zhipu GLM-4".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "glm-4-plus".to_string(),
                provider: "zhipu-glm".to_string(),
                display_name: "GLM-4 Plus".to_string(),
                description: "Zhipu GLM-4 Plus".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "glm-4-air".to_string(),
                provider: "zhipu-glm".to_string(),
                display_name: "GLM-4 Air".to_string(),
                description: "Zhipu GLM-4 Air".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "glm-4-airx".to_string(),
                provider: "zhipu-glm".to_string(),
                display_name: "GLM-4 AirX".to_string(),
                description: "Zhipu GLM-4 AirX".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Bailian
            ModelInfo {
                model_id: "qwen-plus".to_string(),
                provider: "bailian".to_string(),
                display_name: "Bailian Qwen Plus".to_string(),
                description: "Alibaba Qwen Plus".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "qwen-turbo".to_string(),
                provider: "bailian".to_string(),
                display_name: "Bailian Qwen Turbo".to_string(),
                description: "Alibaba Qwen Turbo".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "qwen-max".to_string(),
                provider: "bailian".to_string(),
                display_name: "Bailian Qwen Max".to_string(),
                description: "Alibaba Qwen Max".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "bailian-coder".to_string(),
                provider: "bailian".to_string(),
                display_name: "Bailian For Coding".to_string(),
                description: "Alibaba code model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Kimi
            ModelInfo {
                model_id: "moonshot-v1-8k".to_string(),
                provider: "kimi".to_string(),
                display_name: "Kimi 8K".to_string(),
                description: "Moonshot 8K context".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "moonshot-v1-32k".to_string(),
                provider: "kimi".to_string(),
                display_name: "Kimi 32K".to_string(),
                description: "Moonshot 32K context".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "moonshot-v1-128k".to_string(),
                provider: "kimi".to_string(),
                display_name: "Kimi 128K".to_string(),
                description: "Moonshot 128K context".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // StepFun
            ModelInfo {
                model_id: "step-1v-8k".to_string(),
                provider: "stepfun".to_string(),
                display_name: "Step-1V 8K".to_string(),
                description: "StepFun vision model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "step-1v-32k".to_string(),
                provider: "stepfun".to_string(),
                display_name: "Step-1V 32K".to_string(),
                description: "StepFun vision 32K".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "step-1-flash".to_string(),
                provider: "stepfun".to_string(),
                display_name: "Step-1 Flash".to_string(),
                description: "StepFun fast model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // KAT-Coder
            ModelInfo {
                model_id: "kat-coder".to_string(),
                provider: "kat-coder".to_string(),
                display_name: "KAT-Coder".to_string(),
                description: "KAT Coder".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Longcat
            ModelInfo {
                model_id: "longcat-chat".to_string(),
                provider: "longcat".to_string(),
                display_name: "Longcat Chat".to_string(),
                description: "Longcat chat model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // MiniMax
            ModelInfo {
                model_id: "MiniMax-M2.7".to_string(),
                provider: "minimax".to_string(),
                display_name: "MiniMax-M2.7".to_string(),
                description: "MiniMax M2.7 latest model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "abab6-chat".to_string(),
                provider: "minimax".to_string(),
                display_name: "MiniMax Chat".to_string(),
                description: "MiniMax chat model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "abab6-gs".to_string(),
                provider: "minimax".to_string(),
                display_name: "MiniMax GS".to_string(),
                description: "MiniMax GS model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // DouBaoSeed
            ModelInfo {
                model_id: "doubao-seed".to_string(),
                provider: "doubao".to_string(),
                display_name: "DouBao Seed".to_string(),
                description: "DouBao seed model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // BaiLing
            ModelInfo {
                model_id: "bailing-chat".to_string(),
                provider: "bailing".to_string(),
                display_name: "BaiLing Chat".to_string(),
                description: "BaiLing chat model".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Xiaomi MiMo
            ModelInfo {
                model_id: "mimo-chat".to_string(),
                provider: "xiaomi".to_string(),
                display_name: "MiMo Chat".to_string(),
                description: "Xiaomi MiMo".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // ModelScope
            ModelInfo {
                model_id: "modelscope-chat".to_string(),
                provider: "modelscope".to_string(),
                display_name: "ModelScope Chat".to_string(),
                description: "ModelScope chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // AiHubMix
            ModelInfo {
                model_id: "aihubmix-chat".to_string(),
                provider: "aihubmix".to_string(),
                display_name: "AiHubMix Chat".to_string(),
                description: "AiHubMix chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // SiliconFlow
            ModelInfo {
                model_id: "siliconflow-chat".to_string(),
                provider: "siliconflow".to_string(),
                display_name: "SiliconFlow Chat".to_string(),
                description: "SiliconFlow chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // OpenRouter
            ModelInfo {
                model_id: "openrouter-chat".to_string(),
                provider: "openrouter".to_string(),
                display_name: "OpenRouter Chat".to_string(),
                description: "OpenRouter chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Novita AI
            ModelInfo {
                model_id: "novita-chat".to_string(),
                provider: "novita".to_string(),
                display_name: "Novita AI Chat".to_string(),
                description: "Novita AI chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Nvidia
            ModelInfo {
                model_id: "nvidia-chat".to_string(),
                provider: "nvidia".to_string(),
                display_name: "Nvidia Chat".to_string(),
                description: "Nvidia NIM".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // PackyCode
            ModelInfo {
                model_id: "packycode-chat".to_string(),
                provider: "packycode".to_string(),
                display_name: "PackyCode Chat".to_string(),
                description: "PackyCode chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Cubence
            ModelInfo {
                model_id: "cubence-chat".to_string(),
                provider: "cubence".to_string(),
                display_name: "Cubence Chat".to_string(),
                description: "Cubence chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // AIGoCode
            ModelInfo {
                model_id: "aigocode-chat".to_string(),
                provider: "aigocode".to_string(),
                display_name: "AIGoCode Chat".to_string(),
                description: "AIGoCode chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // RightCode
            ModelInfo {
                model_id: "rightcode-chat".to_string(),
                provider: "rightcode".to_string(),
                display_name: "RightCode Chat".to_string(),
                description: "RightCode chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // AICodeMirror
            ModelInfo {
                model_id: "aicodemirror-chat".to_string(),
                provider: "aicodemirror".to_string(),
                display_name: "AICodeMirror Chat".to_string(),
                description: "AICodeMirror chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // AICoding
            ModelInfo {
                model_id: "aicoding-chat".to_string(),
                provider: "aicoding".to_string(),
                display_name: "AICoding Chat".to_string(),
                description: "AICoding chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // CrazyRouter
            ModelInfo {
                model_id: "crazyrouter-chat".to_string(),
                provider: "crazyrouter".to_string(),
                display_name: "CrazyRouter Chat".to_string(),
                description: "CrazyRouter chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // SSSAiCode
            ModelInfo {
                model_id: "sssaicode-chat".to_string(),
                provider: "sssaicode".to_string(),
                display_name: "SSSAiCode Chat".to_string(),
                description: "SSSAiCode chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Micu
            ModelInfo {
                model_id: "micu-chat".to_string(),
                provider: "micu".to_string(),
                display_name: "Micu Chat".to_string(),
                description: "Micu chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // X-Code API
            ModelInfo {
                model_id: "xcodeapi-chat".to_string(),
                provider: "xcodeapi".to_string(),
                display_name: "X-Code API Chat".to_string(),
                description: "X-Code API chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // CTok.ai
            ModelInfo {
                model_id: "ctok-chat".to_string(),
                provider: "ctok".to_string(),
                display_name: "CTok.ai Chat".to_string(),
                description: "CTok.ai chat".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // GitHub Copilot
            ModelInfo {
                model_id: "github-copilot".to_string(),
                provider: "github-copilot".to_string(),
                display_name: "GitHub Copilot".to_string(),
                description: "GitHub Copilot".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // AWS Bedrock
            ModelInfo {
                model_id: "bedrock-claude".to_string(),
                provider: "aws-bedrock".to_string(),
                display_name: "Claude on Bedrock".to_string(),
                description: "AWS Bedrock Claude".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // OpenAI (original)
            ModelInfo {
                model_id: "gpt-4o".to_string(),
                provider: "openai".to_string(),
                display_name: "GPT-4o".to_string(),
                description: "OpenAI GPT-4o".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "gpt-4-turbo".to_string(),
                provider: "openai".to_string(),
                display_name: "GPT-4 Turbo".to_string(),
                description: "OpenAI GPT-4 Turbo".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Google
            ModelInfo {
                model_id: "gemini-1.5-pro".to_string(),
                provider: "google".to_string(),
                display_name: "Gemini 1.5 Pro".to_string(),
                description: "Google Gemini 1.5 Pro".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "gemini-1.5-flash".to_string(),
                provider: "google".to_string(),
                display_name: "Gemini 1.5 Flash".to_string(),
                description: "Google Gemini 1.5 Flash".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            // Ollama
            ModelInfo {
                model_id: "llama3".to_string(),
                provider: "ollama".to_string(),
                display_name: "Llama 3".to_string(),
                description: "Meta Llama 3".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
            ModelInfo {
                model_id: "codellama".to_string(),
                provider: "ollama".to_string(),
                display_name: "Code Llama".to_string(),
                description: "Code Llama".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            },
        ];

        for model in default_models {
            selector.register_model(model);
        }
    }

    /// 获取模型刷新状态信息
    pub fn get_refresh_status(&self) -> ModelRefreshStatus {
        ModelRefreshStatus {
            needs_refresh: self.selector.needs_refresh(),
            seconds_until_refresh: self.selector.time_until_refresh(),
            last_refresh_time: format!("{:?}", self.selector.last_refresh_time()),
        }
    }

    /// 获取当前模型列表（自动检查刷新）
    pub fn list_available_models(&self) -> Vec<ModelInfo> {
        // 检查是否需要刷新
        self.check_and_refresh_models();
        self.selector.list_models()
    }

    /// 获取全局模型选择器
    pub fn selector(&self) -> &GlobalModelSelector {
        &self.selector
    }

    /// 获取当前选定的模型 ID
    pub fn get_selected_model(&self) -> String {
        self.selector.get_selected_model()
    }

    /// 设置当前选定的模型
    pub fn set_selected_model(&self, model_id: &str) -> Result<(), AIError> {
        self.selector.set_selected_model(model_id)
    }

    /// 获取当前选定模型的详细信息
    pub fn get_selected_model_info(&self) -> Option<ModelInfo> {
        self.selector.get_selected_model_info()
    }

    /// 使用全局选定模型执行聊天(所有 AI 调用的统一入口)
    pub async fn chat_with_selected(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<ChatResponse, AIError> {
        // 检查 Claude Switch 是否已配置，提取结果以避免跨 await 持有 RwLockReadGuard
        let has_claude_switch = {
            let switch = self.claude_switch.read().await;
            switch.is_some()
        };

        // 如果 Claude Switch 已配置，优先使用它
        if has_claude_switch {
            return self.chat_with_claude_switch(messages).await;
        }

        // Claude Switch 未配置，使用常规的 registry 路由
        let model_id = self.selector.get_selected_model();
        let request = ChatRequest {
            messages,
            model: model_id.clone(),
            max_tokens: 4096,
            temperature: 0.7,
        };
        self.registry.chat(request).await
    }

    /// 使用全局选定模型执行补全(所有 AI 调用的统一入口)
    pub async fn complete_with_selected(
        &self,
        prompt: String,
    ) -> Result<CompletionResponse, AIError> {
        let model_id = self.selector.get_selected_model();
        let request = CompletionRequest {
            prompt,
            model: model_id.clone(),
            max_tokens: 4096,
            temperature: 0.7,
            stop_sequences: vec![],
            system_prompt: None,
        };
        self.registry.complete(request).await
    }

    // ==================== Claude Switch 方法 ====================

    /// 配置 Claude Switch 提供商（添加后端）
    pub fn configure_claude_switch(&self, backends: Vec<BackendConfig>) -> Result<(), AIError> {
        if backends.is_empty() {
            return Err(AIError::InvalidRequest("至少需要一个后端配置".to_string()));
        }

        // 创建 Claude Switch 提供商，使用第一个后端作为默认
        let mut provider = ClaudeSwitchProvider::new(backends[0].clone());

        // 添加其他后端
        for backend in backends.iter().skip(1) {
            provider = provider.with_backend(backend.clone());
        }

        // 注册到全局选择器
        let model_id = format!("claude-switch-{}", backends[0].backend.as_str());
        self.selector.register_model(ModelInfo {
            model_id: model_id.clone(),
            provider: "claude-switch".to_string(),
            display_name: format!("Claude Switch ({})", backends[0].backend.as_str()),
            description: "使用 Claude 接口，调用其他后端".to_string(),
            supports_chat: true,
            supports_completion: true,
            is_default: false,
        });

        // 注册到 registry 的 model_to_provider 映射（关键！否则 route() 找不到）
        self.registry
            .register_model_mapping(&model_id, "claude-switch");

        // 存储提供商
        *self.claude_switch.blocking_write() = Some(provider);
        *self.active_backend.blocking_write() = backends[0].backend;

        // 注册 Claude Switch 模型到 registry
        let switch_provider = self.claude_switch.blocking_read();
        if let Some(ref provider) = *switch_provider {
            self.registry.register(Arc::new(provider.clone()));
            self.registry.set_default("claude-switch")?;
        }

        Ok(())
    }

    /// 添加 Claude Switch 后端
    pub fn add_claude_switch_backend(&self, config: BackendConfig) -> Result<(), AIError> {
        let switch = self.claude_switch.blocking_read();

        if let Some(ref provider) = *switch {
            let mut new_provider = provider.clone();
            new_provider = new_provider.with_backend(config.clone());
            drop(switch);

            let model_id = format!("claude-switch-{}", config.backend.as_str());

            // 注册到 selector
            self.selector.register_model(ModelInfo {
                model_id: model_id.clone(),
                provider: "claude-switch".to_string(),
                display_name: format!("Claude Switch ({})", config.backend.as_str()),
                description: "使用 Claude 接口，调用其他后端".to_string(),
                supports_chat: true,
                supports_completion: true,
                is_default: false,
            });

            // 注册到 registry 的 model_to_provider 映射（关键！否则 route() 找不到）
            self.registry
                .register_model_mapping(&model_id, "claude-switch");

            *self.claude_switch.blocking_write() = Some(new_provider);
            Ok(())
        } else {
            Err(AIError::InvalidRequest(
                "Claude Switch 未初始化，请先调用 configure_claude_switch".to_string(),
            ))
        }
    }

    /// 切换 Claude Switch 后端
    pub fn switch_claude_backend(&self, backend: SwitchBackend) -> Result<(), AIError> {
        // 获取配置列表
        let configs = {
            let switch = self.claude_switch.blocking_read();
            if let Some(ref provider) = *switch {
                // list_backends 现在返回 owned 数据
                provider.list_backends()
            } else {
                return Err(AIError::InvalidRequest(
                    "Claude Switch 未初始化".to_string(),
                ));
            }
        };

        // 查找目标后端配置
        let target_config = configs
            .iter()
            .find(|(b, _)| *b == backend)
            .map(|(_, c)| c.clone());

        if let Some(config) = target_config {
            // 构建新的提供商，先添加非目标后端，最后添加目标后端（设为默认）
            let mut new_provider = ClaudeSwitchProvider::new(config);

            for (b, c) in &configs {
                if *b != backend {
                    new_provider = new_provider.with_backend(c.clone());
                }
            }

            // 添加目标后端
            let final_config = configs
                .iter()
                .find(|(b, _)| *b == backend)
                .map(|(_, c)| c.clone())
                .unwrap();
            new_provider = new_provider.with_backend(final_config);

            *self.claude_switch.blocking_write() = Some(new_provider);
            *self.active_backend.blocking_write() = backend;
            Ok(())
        } else {
            Err(AIError::InvalidRequest(format!(
                "后端 '{}' 不可用",
                backend.as_str()
            )))
        }
    }

    /// 获取当前 Claude Switch 后端
    pub fn get_active_backend(&self) -> SwitchBackend {
        *self.active_backend.blocking_read()
    }

    /// 检查 Claude Switch 是否已初始化（异步版本，用于 async 上下文）
    pub async fn is_claude_switch_initialized(&self) -> bool {
        let switch = self.claude_switch.read().await;
        switch.is_some()
    }

    /// 列出所有可用的 Claude Switch 后端
    pub fn list_claude_switch_backends(&self) -> Vec<(SwitchBackend, bool)> {
        let switch = self.claude_switch.blocking_read();
        if let Some(ref provider) = *switch {
            let active = *self.active_backend.blocking_read();
            provider
                .list_backends()
                .into_iter()
                .map(|(b, _)| (b, b == active))
                .collect()
        } else {
            vec![]
        }
    }

    /// 列出所有可用的 Claude Switch 后端（异步版本）
    pub async fn list_claude_switch_backends_async(&self) -> Vec<(SwitchBackend, bool)> {
        let switch = self.claude_switch.read().await;
        if let Some(ref provider) = *switch {
            let active = *self.active_backend.read().await;
            provider
                .list_backends()
                .into_iter()
                .map(|(b, _)| (b, b == active))
                .collect()
        } else {
            vec![]
        }
    }

    /// 使用 Claude Switch 执行聊天
    pub async fn chat_with_claude_switch(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<ChatResponse, AIError> {
        let switch = self.claude_switch.read().await;

        if let Some(ref provider) = *switch {
            let request = ChatRequest {
                messages,
                model: "claude-sonnet-4-6".to_string(),
                max_tokens: 4096,
                temperature: 0.7,
            };
            provider.chat(request).await
        } else {
            Err(AIError::InvalidRequest("Claude Switch 未配置".to_string()))
        }
    }

    /// 测试 Claude Switch 后端连接
    pub async fn test_claude_switch_backend(
        &self,
        backend: SwitchBackend,
        api_key: &str,
        model: &str,
    ) -> Result<(), AIError> {
        use crate::claude_switch::BackendConfig;

        let config = match backend {
            SwitchBackend::MiniMax => BackendConfig::minimax(api_key.to_string(), model),
            SwitchBackend::OpenAI => {
                BackendConfig::openai(api_key.to_string(), "https://api.openai.com/v1", model)
            }
            SwitchBackend::DeepSeek => BackendConfig::deepseek(api_key.to_string(), model),
            SwitchBackend::Zhipu => BackendConfig::zhipu(api_key.to_string(), model),
            SwitchBackend::Ollama => BackendConfig::ollama("http://localhost:11434", model),
        };

        let provider = ClaudeSwitchProvider::new(config);
        provider.test_connection().await
    }
}

impl Default for AIModelManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 简化的 AI 调用接口
pub async fn complete(
    model_id: &str,
    prompt: String,
    manager: &AIModelManager,
) -> Result<AIResponse, AIError> {
    manager.call(model_id, prompt).await
}

/// 简化的聊天接口
pub async fn chat(
    model_id: &str,
    user_message: String,
    manager: &AIModelManager,
) -> Result<AIResponse, AIError> {
    let config = manager.get_model_config(model_id).unwrap_or_else(|| {
        // 如果没有专门配置，使用默认模型的配置
        manager.get_config().default_model
    });

    manager
        .chat(AIRequestParams::chat(config, user_message))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_config_default() {
        let config = ModelConfig::default();
        assert_eq!(config.model_id, "claude-sonnet-4-5");
        assert_eq!(config.provider, ProviderType::Anthropic);
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.temperature, 0.7);
    }

    #[test]
    fn test_request_params_builder() {
        let config = ModelConfig::default();
        let params = AIRequestParams::completion(config, "Hello".to_string())
            .with_system_prompt("You are helpful".to_string())
            .with_stream();

        assert_eq!(params.user_message, "Hello");
        assert_eq!(params.system_prompt, Some("You are helpful".to_string()));
        assert!(params.stream);
    }

    #[test]
    fn test_provider_type_display() {
        assert_eq!(ProviderType::Anthropic.to_string(), "anthropic");
        assert_eq!(ProviderType::OpenAI.to_string(), "openai");
        assert_eq!(ProviderType::Google.to_string(), "google");
        assert_eq!(ProviderType::Ollama.to_string(), "ollama");
    }
}
