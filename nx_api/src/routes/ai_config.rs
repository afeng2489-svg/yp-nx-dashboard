//! AI Configuration Routes
//!
//! API routes for multi-CLI AI provider management.

use axum::{
    extract::{State, Query},
    http::StatusCode,
    Json,
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::routes::AppState;
use crate::services::{
    AIProvider as ProviderAIProvider, APIFormat, ConnectionTestResult, MappingType, ModelMapping, ProviderPreset,
    claude_cli::{call_claude_cli, messages_to_prompt},
};
use nexus_ai::{
    AIModelManager, BackendConfig, ChatMessage, ChatResponse, CLI, CLIConfig, CLICapability, CLIContext,
    CLIResponse, CLISelectionStrategy, ModelRefreshStatus, ProviderType, SwitchBackend, TokenUsage,
};

/// 请求：列出 AI 提供商
#[derive(Debug, serde::Deserialize)]
pub struct ListProvidersRequest {
    /// 可选的提供商类型过滤
    #[serde(default)]
    pub provider_type: Option<ProviderType>,
}

/// 提供商信息响应
#[derive(Debug, serde::Serialize)]
pub struct ProviderInfo {
    /// 提供商名称
    pub name: String,
    /// 提供商类型
    pub provider_type: ProviderType,
    /// 支持的模型
    pub models: Vec<String>,
    /// 支持的 CLI
    pub supported_clis: Vec<String>,
    /// 默认模型
    pub default_model: String,
}

/// 提供商列表响应
#[derive(Debug, serde::Serialize)]
pub struct ProviderListResponse {
    pub providers: Vec<ProviderInfo>,
}

/// CLI 信息响应
#[derive(Debug, serde::Serialize)]
pub struct CLIInfo {
    /// CLI 类型
    pub cli: String,
    /// 显示名称
    pub display_name: String,
    /// 是否启用
    pub enabled: bool,
    /// 可用性状态
    pub available: bool,
    /// 能力信息
    pub capability: Option<CLICapability>,
    /// 配置路径（如果有）
    pub path: Option<String>,
}

/// CLI 列表响应
#[derive(Debug, serde::Serialize)]
pub struct CLIListResponse {
    pub clis: Vec<CLIInfo>,
    /// 当前选择的策略
    pub selection_strategy: CLISelectionStrategy,
    /// 默认 CLI
    pub default_cli: Option<String>,
}

/// CLI 执行请求
#[derive(Debug, serde::Deserialize)]
pub struct ExecuteCLIRequest {
    /// 提示词
    pub prompt: String,
    /// CLI 类型（可选，如果不提供则自动选择）
    pub cli: Option<CLI>,
    /// 工作目录
    pub working_directory: Option<String>,
    /// 超时时间（秒）
    pub timeout_secs: Option<u64>,
    /// 额外参数
    #[serde(default)]
    pub extra_params: HashMap<String, String>,
    /// 自动 yes 模式（当 AI 询问确认时自动回答 yes）
    #[serde(default)]
    pub auto_yes: Option<bool>,
}

/// CLI 执行响应
#[derive(Debug, serde::Serialize)]
pub struct ExecuteCLIResponse {
    pub output: String,
    pub error: Option<String>,
    pub exit_code: i32,
    pub execution_time_ms: u64,
    pub cli: String,
}

/// 更新 CLI 配置请求
#[derive(Debug, serde::Deserialize)]
pub struct UpdateCLIConfigRequest {
    pub cli: CLI,
    pub enabled: Option<bool>,
    pub path: Option<String>,
    #[serde(default)]
    pub extra_params: HashMap<String, String>,
}

/// 更新选择策略请求
#[derive(Debug, serde::Deserialize)]
pub struct UpdateStrategyRequest {
    pub strategy: CLISelectionStrategy,
    pub default_cli: Option<CLI>,
}

/// 列出 AI 提供商
pub async fn list_providers(
    State(_state): State<Arc<AppState>>,
    Query(_params): Query<ListProvidersRequest>,
) -> Result<Json<ProviderListResponse>, (StatusCode, String)> {
    // 静态返回当前支持的提供商
    let providers = vec![
        ProviderInfo {
            name: "anthropic".to_string(),
            provider_type: ProviderType::Anthropic,
            models: vec![
                "claude-opus-4-5".to_string(),
                "claude-sonnet-4-5".to_string(),
                "claude-haiku-4-5".to_string(),
            ],
            supported_clis: vec![CLI::Claude.identifier().to_string()],
            default_model: "claude-sonnet-4-5".to_string(),
        },
        ProviderInfo {
            name: "openai".to_string(),
            provider_type: ProviderType::OpenAI,
            models: vec![
                "gpt-4-turbo".to_string(),
                "gpt-4".to_string(),
                "gpt-3.5-turbo".to_string(),
            ],
            supported_clis: vec![],
            default_model: "gpt-4-turbo".to_string(),
        },
        ProviderInfo {
            name: "google".to_string(),
            provider_type: ProviderType::Google,
            models: vec![
                "gemini-pro".to_string(),
                "gemini-1.5-pro".to_string(),
                "gemini-1.5-flash".to_string(),
            ],
            supported_clis: vec![CLI::Gemini.identifier().to_string()],
            default_model: "gemini-pro".to_string(),
        },
        ProviderInfo {
            name: "ollama".to_string(),
            provider_type: ProviderType::Ollama,
            models: vec!["llama2".to_string(), "codellama".to_string()],
            supported_clis: vec![],
            default_model: "llama2".to_string(),
        },
        ProviderInfo {
            name: "codex".to_string(),
            provider_type: ProviderType::Codex,
            models: vec!["codex".to_string(), "codex-plus".to_string()],
            supported_clis: vec![
                CLI::Codex.identifier().to_string(),
                CLI::OpenCode.identifier().to_string(),
            ],
            default_model: "codex".to_string(),
        },
        ProviderInfo {
            name: "qwen".to_string(),
            provider_type: ProviderType::Qwen,
            models: vec![
                "qwen-turbo".to_string(),
                "qwen-plus".to_string(),
                "qwen-max".to_string(),
            ],
            supported_clis: vec![CLI::Qwen.identifier().to_string()],
            default_model: "qwen-turbo".to_string(),
        },
        ProviderInfo {
            name: "opencode".to_string(),
            provider_type: ProviderType::OpenCode,
            models: vec!["opencode".to_string()],
            supported_clis: vec![CLI::OpenCode.identifier().to_string()],
            default_model: "opencode".to_string(),
        },
        ProviderInfo {
            name: "minimax".to_string(),
            provider_type: ProviderType::MiniMax,
            models: vec![
                "MiniMax-M2.7".to_string(),
                "abab6-chat".to_string(),
                "abab6-gs".to_string(),
                "doubao-seed".to_string(),
            ],
            supported_clis: vec![],
            default_model: "MiniMax-M2.7".to_string(),
        },
    ];

    Ok(Json(ProviderListResponse { providers }))
}

/// 列出可用的 CLI
pub async fn list_clis(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<CLIListResponse>, (StatusCode, String)> {
    let cli_infos = vec![
        CLIInfo {
            cli: CLI::Claude.identifier().to_string(),
            display_name: CLI::Claude.display_name().to_string(),
            enabled: true,
            available: true,
            capability: Some(CLICapability {
                cli: CLI::Claude,
                available: true,
                version: Some("3.5".to_string()),
                features: vec![
                    "Code Review".to_string(),
                    "Debugging".to_string(),
                    "Explanation".to_string(),
                    "Refactoring".to_string(),
                ],
            }),
            path: None,
        },
        CLIInfo {
            cli: CLI::Gemini.identifier().to_string(),
            display_name: CLI::Gemini.display_name().to_string(),
            enabled: true,
            available: true,
            capability: Some(CLICapability {
                cli: CLI::Gemini,
                available: true,
                version: Some("1.5".to_string()),
                features: vec![
                    "Multimodal".to_string(),
                    "Long Context".to_string(),
                    "Creative Tasks".to_string(),
                ],
            }),
            path: None,
        },
        CLIInfo {
            cli: CLI::Codex.identifier().to_string(),
            display_name: CLI::Codex.display_name().to_string(),
            enabled: true,
            available: true,
            capability: Some(CLICapability {
                cli: CLI::Codex,
                available: true,
                version: Some("1.0".to_string()),
                features: vec![
                    "Code Generation".to_string(),
                    "Algorithm Implementation".to_string(),
                    "Function Writing".to_string(),
                ],
            }),
            path: None,
        },
        CLIInfo {
            cli: CLI::Qwen.identifier().to_string(),
            display_name: CLI::Qwen.display_name().to_string(),
            enabled: true,
            available: true,
            capability: Some(CLICapability {
                cli: CLI::Qwen,
                available: true,
                version: Some("2.5".to_string()),
                features: vec![
                    "Chinese Language".to_string(),
                    "Math Reasoning".to_string(),
                    "Logic".to_string(),
                ],
            }),
            path: None,
        },
        CLIInfo {
            cli: CLI::OpenCode.identifier().to_string(),
            display_name: CLI::OpenCode.display_name().to_string(),
            enabled: true,
            available: true,
            capability: Some(CLICapability {
                cli: CLI::OpenCode,
                available: true,
                version: Some("1.0".to_string()),
                features: vec![
                    "Open Source Projects".to_string(),
                    "GitHub Integration".to_string(),
                    "Popular Frameworks".to_string(),
                ],
            }),
            path: None,
        },
    ];

    Ok(Json(CLIListResponse {
        clis: cli_infos,
        selection_strategy: CLISelectionStrategy::Auto,
        default_cli: Some(CLI::Claude.identifier().to_string()),
    }))
}

/// 执行 CLI
pub async fn execute_cli(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ExecuteCLIRequest>,
) -> Result<Json<ExecuteCLIResponse>, (StatusCode, String)> {
    let start = std::time::Instant::now();
    let cli = request.cli.unwrap_or(CLI::Claude);

    // 先提取 working_directory，避免 move 后再借用
    let working_dir = request.working_directory.as_deref();

    let context = CLIContext {
        working_directory: None, // CLI 调用时用 working_dir 参数
        env_vars: HashMap::new(),
        timeout_secs: request.timeout_secs,
        extra_params: request.extra_params,
    };

    // 实际调用 AIModelManager 执行 AI 请求
    let manager = &state.ai_model_manager;

    // 构建聊天消息 - 添加自动 yes 指令
    let auto_yes_prefix = "You are operating in auto-yes mode. If you ask any question requiring confirmation (yes/no, continue?, proceed?, etc.), always assume the answer is YES and proceed automatically. Never ask for confirmation.";
    let full_prompt = if request.auto_yes.unwrap_or(false) {
        format!("{}\n\nUser request: {}", auto_yes_prefix, request.prompt)
    } else {
        request.prompt.clone()
    };

    // 直接调用 Claude CLI（Claude Switch 切换后自动使用新模型）
    // working_directory 用于 --project 参数切换项目目录
    match call_claude_cli(&full_prompt, working_dir).await {
        Ok(output) => {
            let execution_time_ms = start.elapsed().as_millis() as u64;
            Ok(Json(ExecuteCLIResponse {
                output,
                error: None,
                exit_code: 0,
                execution_time_ms,
                cli: cli.identifier().to_string(),
            }))
        }
        Err(e) => {
            let execution_time_ms = start.elapsed().as_millis() as u64;
            Ok(Json(ExecuteCLIResponse {
                output: String::new(),
                error: Some(e),
                exit_code: 1,
                execution_time_ms,
                cli: cli.identifier().to_string(),
            }))
        }
    }
}

/// 更新 CLI 配置
pub async fn update_cli_config(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<UpdateCLIConfigRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    tracing::info!("更新 CLI 配置: {:?}", request.cli);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("CLI {} 配置已更新", request.cli.identifier())
    })))
}

/// 更新选择策略
pub async fn update_selection_strategy(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<UpdateStrategyRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    tracing::info!("更新选择策略: {:?}", request.strategy);

    Ok(Json(serde_json::json!({
        "success": true,
        "strategy": request.strategy,
        "default_cli": request.default_cli.map(|c| c.identifier().to_string())
    })))
}

/// 获取选择建议
#[derive(Debug, serde::Serialize)]
pub struct SelectionSuggestion {
    pub recommended_cli: String,
    pub reason: String,
    pub alternatives: Vec<String>,
}

/// 获取 CLI 选择建议
pub async fn get_selection_suggestion(
    State(_state): State<Arc<AppState>>,
    Json(prompt): Json<String>,
) -> Result<Json<SelectionSuggestion>, (StatusCode, String)> {
    let prompt_lower = prompt.to_lowercase();

    // 基于提示词内容返回建议
    let (recommended, reason, alternatives) = if prompt_lower.contains("review")
        || prompt_lower.contains("explain")
        || prompt_lower.contains("debug")
    {
        (
            CLI::Claude.identifier().to_string(),
            "Claude 适合代码审查、解释和调试任务".to_string(),
            vec![
                CLI::Gemini.identifier().to_string(),
                CLI::Codex.identifier().to_string(),
            ],
        )
    } else if prompt_lower.contains("write code")
        || prompt_lower.contains("implement")
        || prompt_lower.contains("function")
    {
        (
            CLI::Codex.identifier().to_string(),
            "Codex 专精于代码生成和算法实现".to_string(),
            vec![
                CLI::OpenCode.identifier().to_string(),
                CLI::Claude.identifier().to_string(),
            ],
        )
    } else if prompt_lower.contains("中文")
        || prompt_lower.contains("数学")
        || prompt_lower.contains("逻辑")
    {
        (
            CLI::Qwen.identifier().to_string(),
            "Qwen 擅长中文理解和数学推理".to_string(),
            vec![
                CLI::Claude.identifier().to_string(),
                CLI::Gemini.identifier().to_string(),
            ],
        )
    } else if prompt_lower.contains("github")
        || prompt_lower.contains("open source")
        || prompt_lower.contains("framework")
    {
        (
            CLI::OpenCode.identifier().to_string(),
            "OpenCode 专为开源项目和流行框架优化".to_string(),
            vec![
                CLI::Codex.identifier().to_string(),
                CLI::Claude.identifier().to_string(),
            ],
        )
    } else if prompt_lower.contains("image")
        || prompt_lower.contains("video")
        || prompt_lower.contains("multimodal")
    {
        (
            CLI::Gemini.identifier().to_string(),
            "Gemini 支持多模态内容和长上下文".to_string(),
            vec![
                CLI::Claude.identifier().to_string(),
                CLI::Qwen.identifier().to_string(),
            ],
        )
    } else {
        (
            CLI::Claude.identifier().to_string(),
            "Claude 是通用任务的首选".to_string(),
            vec![
                CLI::Gemini.identifier().to_string(),
                CLI::Codex.identifier().to_string(),
                CLI::Qwen.identifier().to_string(),
            ],
        )
    };

    Ok(Json(SelectionSuggestion {
        recommended_cli: recommended,
        reason,
        alternatives,
    }))
}

// ============== Model Selection Endpoints ==============

/// 模型信息响应(用于 API)
#[derive(Debug, serde::Serialize)]
pub struct ModelInfoResponse {
    pub model_id: String,
    pub provider: String,
    pub display_name: String,
    pub description: String,
    pub supports_chat: bool,
    pub supports_completion: bool,
    pub is_default: bool,
}

/// 当前选定的模型响应
#[derive(Debug, serde::Serialize)]
pub struct SelectedModelResponse {
    pub model_id: String,
    pub provider: String,
    pub display_name: String,
}

/// 设置选定模型请求
#[derive(Debug, serde::Deserialize)]
pub struct SetSelectedModelRequest {
    pub model_id: String,
}

/// 设置默认模型请求
#[derive(Debug, serde::Deserialize)]
pub struct SetDefaultModelRequest {
    pub model_id: String,
    pub provider: String,
}

/// 模型配置请求
#[derive(Debug, serde::Deserialize)]
pub struct ModelConfigRequest {
    pub model_id: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
}

/// 获取当前选定的模型
pub async fn get_selected_model(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SelectedModelResponse>, (StatusCode, String)> {
    let manager = &state.ai_model_manager;
    let model_info = manager.get_selected_model_info();

    match model_info {
        Some(info) => Ok(Json(SelectedModelResponse {
            model_id: info.model_id,
            provider: info.provider,
            display_name: info.display_name,
        })),
        None => {
            let selected = manager.get_selected_model();
            Ok(Json(SelectedModelResponse {
                model_id: selected.clone(),
                provider: "unknown".to_string(),
                display_name: selected,
            }))
        }
    }
}

/// 设置当前选定的模型
pub async fn set_selected_model(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SetSelectedModelRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let manager = &state.ai_model_manager;

    manager.set_selected_model(&request.model_id)
        .map_err(|e| {
            tracing::error!("Failed to set selected model: {}", e);
            (StatusCode::BAD_REQUEST, format!("Failed to set model: {}", e))
        })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "model_id": request.model_id,
        "message": "Model selected successfully"
    })))
}

/// 列出所有可用模型
pub async fn list_models(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ModelInfoResponse>>, (StatusCode, String)> {
    let manager = &state.ai_model_manager;
    let models = manager.list_available_models();

    let response: Vec<ModelInfoResponse> = models.into_iter().map(|m| {
        ModelInfoResponse {
            model_id: m.model_id,
            provider: m.provider,
            display_name: m.display_name,
            description: m.description,
            supports_chat: m.supports_chat,
            supports_completion: m.supports_completion,
            is_default: m.is_default,
        }
    }).collect();

    Ok(Json(response))
}

/// 使用选定模型执行聊天(测试端点)
#[derive(Debug, serde::Deserialize)]
pub struct ChatWithModelRequest {
    pub messages: Vec<nexus_ai::ChatMessage>,
}

/// 使用选定模型执行聊天
pub async fn chat_with_selected(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<ChatWithModelRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    // 将消息转换为 prompt
    let prompt = messages_to_prompt(&request.messages);

    // 调用 Claude CLI（Claude Switch 切换后自动使用新模型）
    call_claude_cli(&prompt, None)
        .await
        .map(|content| {
            Json(ChatResponse {
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content,
                },
                model: "claude".to_string(), // Claude CLI 使用的模型由配置决定
                usage: TokenUsage {
                    input_tokens: 0,  // Claude CLI 不返回 token 使用量
                    output_tokens: 0,
                },
                stop_reason: "stop".to_string(),
            })
        })
        .map_err(|e| {
            tracing::error!("Chat with Claude CLI failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Chat failed: {}", e))
        })
}

/// 设置默认模型
pub async fn set_default_model(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SetDefaultModelRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let manager = &state.ai_model_manager;

    // 更新默认模型配置
    let config = manager.get_config();
    let mut new_config = config;

    // 找到对应的 ProviderType
    let provider_type = match request.provider.as_str() {
        "anthropic" => nexus_ai::ProviderType::Anthropic,
        "openai" => nexus_ai::ProviderType::OpenAI,
        "google" => nexus_ai::ProviderType::Google,
        "ollama" => nexus_ai::ProviderType::Ollama,
        "codex" => nexus_ai::ProviderType::Codex,
        "qwen" => nexus_ai::ProviderType::Qwen,
        "opencode" => nexus_ai::ProviderType::OpenCode,
        _ => return Err((StatusCode::BAD_REQUEST, format!("Unknown provider: {}", request.provider))),
    };

    // 更新默认模型
    new_config.default_model = nexus_ai::ModelConfig {
        model_id: request.model_id.clone(),
        provider: provider_type,
        ..Default::default()
    };

    manager.update_config(new_config);

    Ok(Json(serde_json::json!({
        "success": true,
        "model_id": request.model_id,
        "provider": request.provider,
        "message": "Default model updated"
    })))
}

/// 获取提供商的可用模型
pub async fn get_provider_models(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(provider): axum::extract::Path<String>,
) -> Result<Json<Vec<ModelInfoResponse>>, (StatusCode, String)> {
    let manager = &state.ai_model_manager;
    let all_models = manager.list_available_models();

    let provider_models: Vec<ModelInfoResponse> = all_models
        .into_iter()
        .filter(|m| m.provider == provider)
        .map(|m| ModelInfoResponse {
            model_id: m.model_id,
            provider: m.provider,
            display_name: m.display_name,
            description: m.description,
            supports_chat: m.supports_chat,
            supports_completion: m.supports_completion,
            is_default: m.is_default,
        })
        .collect();

    Ok(Json(provider_models))
}

/// 更新模型配置
pub async fn update_model_config(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ModelConfigRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let manager = &state.ai_model_manager;

    // 获取现有配置
    let mut config = manager.get_config();

    // 更新对应模型的配置
    if config.default_model.model_id == request.model_id {
        if let Some(max_tokens) = request.max_tokens {
            config.default_model.max_tokens = max_tokens;
        }
        if let Some(temperature) = request.temperature {
            config.default_model.temperature = temperature;
        }
        manager.update_config(config);
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "model_id": request.model_id,
        "message": "Model config updated"
    })))
}

/// 获取模型刷新状态
pub async fn get_refresh_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ModelRefreshStatus>, (StatusCode, String)> {
    let manager = &state.ai_model_manager;
    let status = manager.get_refresh_status();
    Ok(Json(status))
}

/// 手动刷新模型列表
#[derive(Debug, serde::Serialize)]
pub struct RefreshModelsResponse {
    pub success: bool,
    pub models_before: usize,
    pub models_after: usize,
    pub message: String,
}

pub async fn refresh_models(
    State(state): State<Arc<AppState>>,
) -> Result<Json<RefreshModelsResponse>, (StatusCode, String)> {
    let manager = &state.ai_model_manager;
    let (before, after) = manager.refresh_models();
    Ok(Json(RefreshModelsResponse {
        success: true,
        models_before: before,
        models_after: after,
        message: format!("模型列表已刷新: {} -> {} 个模型", before, after),
    }))
}

// ============== API Key Management Endpoints ==============

/// API 密钥信息（不包含实际密钥）
#[derive(Debug, serde::Serialize)]
pub struct ApiKeyInfo {
    pub provider: String,
    pub has_key: bool,
    pub updated_at: Option<String>,
}

/// 保存 API 密钥请求
#[derive(Debug, serde::Deserialize)]
pub struct SaveApiKeyRequest {
    pub provider: String,
    pub api_key: String,
}

/// 列出已配置的 API 密钥提供商
pub async fn list_api_keys(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ApiKeyInfo>>, (StatusCode, String)> {
    use crate::services::ApiKeyRepository;

    let providers = ["anthropic", "openai", "google", "ollama", "codex", "qwen", "opencode", "minimax"];
    let mut result = Vec::new();

    for provider in providers {
        let has_key = state.api_key_repository
            .exists(provider)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let updated_at = if has_key {
            // 获取密钥以触发错误（如果需要），但不返回
            state.api_key_repository
                .get(provider)
                .ok()
                .map(|_| "configured".to_string())
        } else {
            None
        };

        result.push(ApiKeyInfo {
            provider: provider.to_string(),
            has_key,
            updated_at,
        });
    }

    Ok(Json(result))
}

/// 保存 API 密钥
pub async fn save_api_key(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SaveApiKeyRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use crate::services::ApiKeyRepository;

    // 验证 provider
    let valid_providers = ["anthropic", "openai", "google", "ollama", "codex", "qwen", "opencode", "minimax"];
    if !valid_providers.contains(&request.provider.as_str()) {
        return Err((StatusCode::BAD_REQUEST, format!("Invalid provider: {}", request.provider)));
    }

    // 保存密钥
    state.api_key_repository
        .save(&request.provider, &request.api_key)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 重新加载 AI 配置
    reload_ai_config(&state).await;

    tracing::info!("API 密钥已保存: {}", request.provider);

    Ok(Json(serde_json::json!({
        "success": true,
        "provider": request.provider,
        "message": format!("API 密钥已保存，将在下次请求时生效")
    })))
}

/// 删除 API 密钥
pub async fn delete_api_key(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(provider): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use crate::services::ApiKeyRepository;

    let deleted = state.api_key_repository
        .delete(&provider)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !deleted {
        return Err((StatusCode::NOT_FOUND, format!("No API key found for provider: {}", provider)));
    }

    // 重新加载 AI 配置
    reload_ai_config(&state).await;

    tracing::info!("API 密钥已删除: {}", provider);

    Ok(Json(serde_json::json!({
        "success": true,
        "provider": provider,
        "message": format!("API 密钥已删除")
    })))
}

/// 重新加载 AI 配置
async fn reload_ai_config(state: &AppState) {
    use crate::services::ApiKeyRepository;
    use nexus_ai::{AIModelManager, AIManagerConfig, APIConfig, ProviderType, ModelConfig};
    use std::collections::HashMap;

    let mut api_config = HashMap::new();
    let mut enabled_providers = Vec::new();

    // 从数据库加载 API 密钥
    let providers = [
        ("anthropic", ProviderType::Anthropic),
        ("openai", ProviderType::OpenAI),
        ("google", ProviderType::Google),
        ("ollama", ProviderType::Ollama),
        ("codex", ProviderType::Codex),
        ("qwen", ProviderType::Qwen),
        ("opencode", ProviderType::OpenCode),
        ("minimax", ProviderType::MiniMax),
    ];

    for (name, provider_type) in providers {
        if let Ok(Some(key)) = state.api_key_repository.get(name) {
            if !key.is_empty() {
                api_config.insert(provider_type, APIConfig {
                    api_key: key,
                    base_url: String::new(),
                    organization_id: String::new(),
                    timeout_secs: 120,
                });
                enabled_providers.push(provider_type);
            }
        }
    }

    // 如果没有配置任何密钥，使用默认配置
    if api_config.is_empty() {
        // 尝试从环境变量加载
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            if !key.is_empty() {
                api_config.insert(ProviderType::Anthropic, APIConfig {
                    api_key: key,
                    base_url: String::new(),
                    organization_id: String::new(),
                    timeout_secs: 120,
                });
                enabled_providers.push(ProviderType::Anthropic);
            }
        }

        // 尝试从环境变量加载 MiniMax
        if let Ok(key) = std::env::var("MINIMAX_API_KEY") {
            if !key.is_empty() {
                api_config.insert(ProviderType::MiniMax, APIConfig {
                    api_key: key,
                    base_url: String::new(),
                    organization_id: String::new(),
                    timeout_secs: 120,
                });
                enabled_providers.push(ProviderType::MiniMax);
            }
        }
    }

    let default_model = ModelConfig::default();
    let config = AIManagerConfig {
        default_model,
        api_config,
        enabled_providers,
    };

    // 更新 AIModelManager
    state.ai_model_manager.update_config(config);

    tracing::info!("AI 配置已重新加载");
}

// ============== AI Provider V2 Endpoints ==============

/// Provider V2 列表响应
#[derive(Debug, serde::Serialize)]
pub struct ProviderListV2Response {
    pub providers: Vec<ProviderAIProvider>,
}

/// Provider V2 列表
pub async fn list_providers_v2(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ProviderListV2Response>, (StatusCode, String)> {
    let providers = state.provider_service
        .list_providers()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ProviderListV2Response { providers }))
}

/// Provider V2 详情
pub async fn get_provider(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ProviderAIProvider>, (StatusCode, String)> {
    let provider = state.provider_service
        .get_provider(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Provider not found: {}", id)))?;

    Ok(Json(provider))
}

/// 创建 Provider V2 请求
#[derive(Debug, serde::Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    pub provider_key: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub api_format: APIFormat,
    pub auth_field: String,
    pub base_url: String,
    pub api_key: String,
    pub enabled: Option<bool>,
    pub config_json: Option<String>,
}

/// 创建 Provider V2
pub async fn create_provider(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateProviderRequest>,
) -> Result<Json<ProviderAIProvider>, (StatusCode, String)> {
    let provider_id = uuid::Uuid::new_v4().to_string();

    let provider = ProviderAIProvider {
        id: provider_id.clone(),
        provider_key: request.provider_key,
        name: request.name,
        description: request.description,
        website: request.website,
        api_key: None, // API key stored separately
        base_url: request.base_url,
        api_format: request.api_format,
        auth_field: request.auth_field,
        enabled: request.enabled.unwrap_or(true),
        config_json: request.config_json,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Create provider first
    state.provider_service
        .create_provider(&provider)
        .await
        .map_err(|e| match e {
            crate::services::ProviderServiceError::InvalidOperation(msg) => {
                (StatusCode::BAD_REQUEST, msg)
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;

    // Then save API key (if provided)
    if !request.api_key.is_empty() {
        state.provider_service
            .save_api_key(&provider_id, &request.api_key)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(Json(provider))
}

/// 更新 Provider V2 请求
#[derive(Debug, serde::Deserialize)]
pub struct UpdateProviderRequest {
    pub name: Option<String>,
    pub provider_key: Option<String>,
    pub description: Option<String>,
    pub website: Option<String>,
    pub api_format: Option<APIFormat>,
    pub auth_field: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub enabled: Option<bool>,
    pub config_json: Option<String>,
}

/// 更新 Provider V2
pub async fn update_provider(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(request): Json<UpdateProviderRequest>,
) -> Result<Json<ProviderAIProvider>, (StatusCode, String)> {
    let existing = state.provider_service
        .get_provider(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Provider not found: {}", id)))?;

    // Update API key if provided
    if let Some(ref api_key) = request.api_key {
        if !api_key.is_empty() {
            state.provider_service
                .save_api_key(&id, api_key)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }

    let updated = ProviderAIProvider {
        id: existing.id,
        provider_key: request.provider_key.unwrap_or(existing.provider_key),
        name: request.name.unwrap_or(existing.name),
        description: request.description.or(existing.description),
        website: request.website.or(existing.website),
        api_key: None, // Never expose API key
        base_url: request.base_url.unwrap_or(existing.base_url),
        api_format: request.api_format.unwrap_or(existing.api_format),
        auth_field: request.auth_field.unwrap_or(existing.auth_field),
        enabled: request.enabled.unwrap_or(existing.enabled),
        config_json: request.config_json.or(existing.config_json),
        created_at: existing.created_at,
        updated_at: chrono::Utc::now(),
    };

    let result = state.provider_service
        .update_provider(&updated)
        .await
        .map_err(|e| match e {
            crate::services::ProviderServiceError::NotFound(_) => {
                (StatusCode::NOT_FOUND, e.to_string())
            }
            crate::services::ProviderServiceError::InvalidOperation(msg) => {
                (StatusCode::BAD_REQUEST, msg)
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;

    Ok(Json(result))
}

/// 删除 Provider V2
pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let deleted = state.provider_service
        .delete_provider(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !deleted {
        return Err((StatusCode::NOT_FOUND, format!("Provider not found: {}", id)));
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Provider {} deleted", id)
    })))
}

/// 获取 Provider 的模型映射
pub async fn get_provider_mappings(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(provider_id): axum::extract::Path<String>,
) -> Result<Json<Vec<ModelMapping>>, (StatusCode, String)> {
    let mappings = state.provider_service
        .get_model_mappings(&provider_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(mappings))
}

/// 添加模型映射请求
#[derive(Debug, serde::Deserialize)]
pub struct AddModelMappingRequest {
    pub mapping_type: MappingType,
    pub model_id: String,
    pub display_name: Option<String>,
    pub config_json: Option<String>,
}

/// 添加模型映射
pub async fn add_model_mapping(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(provider_id): axum::extract::Path<String>,
    Json(request): Json<AddModelMappingRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mapping = ModelMapping {
        id: uuid::Uuid::new_v4().to_string(),
        provider_id: provider_id,
        mapping_type: request.mapping_type,
        model_id: request.model_id,
        display_name: request.display_name,
        config_json: request.config_json,
    };

    state.provider_service
        .add_model_mapping(&mapping)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Model mapping added"
    })))
}

/// 删除模型映射
pub async fn remove_model_mapping(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((provider_id, mapping_id)): axum::extract::Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    state.provider_service
        .delete_model_mapping(&mapping_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Model mapping removed"
    })))
}

/// 测试提供商连接
pub async fn test_provider_connection(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(provider_id): axum::extract::Path<String>,
) -> Result<Json<ConnectionTestResult>, (StatusCode, String)> {
    let result = state.provider_service
        .test_provider_connection(&provider_id)
        .await
        .map_err(|e| match e {
            crate::services::ProviderServiceError::NotFound(_) => {
                (StatusCode::NOT_FOUND, e.to_string())
            }
            crate::services::ProviderServiceError::ConnectionFailed(msg) => {
                (StatusCode::BAD_REQUEST, msg)
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;

    Ok(Json(result))
}

/// 启用提供商请求
#[derive(Debug, serde::Deserialize)]
pub struct EnableProviderRequest {
    /// 可选的模型 ID，如果不提供则使用默认模型
    pub model: Option<String>,
}

/// 启用提供商 - 将其设置为默认 AI 提供商
pub async fn enable_provider(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(provider_id): axum::extract::Path<String>,
    Json(request): Json<EnableProviderRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let manager = &state.ai_model_manager;

    // 获取提供商信息
    let provider = state.provider_service
        .get_provider(&provider_id)
        .await
        .map_err(|e| -> (StatusCode, Json<serde_json::Value>) {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() })))
        })?
        .ok_or_else(|| -> (StatusCode, Json<serde_json::Value>) {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": format!("Provider {} not found", provider_id) })))
        })?;

    // 获取 API key
    let api_key = state.provider_service
        .get_api_key(&provider_id)
        .await
        .map_err(|e| -> (StatusCode, Json<serde_json::Value>) {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() })))
        })?
        .ok_or_else(|| -> (StatusCode, Json<serde_json::Value>) {
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": format!(
                    "❌ API Key 未配置\n\n请先为 {} 配置 API Key：\nAI设置 → 选择 {} → 添加 API Key",
                    provider.name, provider.name
                )
            })))
        })?;

    // 获取要使用的模型
    let model = if let Some(ref model_id) = request.model {
        model_id.clone()
    } else {
        // 如果没有提供模型，从模型映射中获取
        let mappings = state.provider_service
            .get_model_mappings(&provider_id)
            .await
            .map_err(|e| -> (StatusCode, Json<serde_json::Value>) {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Failed to get model mappings: {}", e) })))
            })?;

        // 优先使用 "main" 类型的映射
        let model_id = mappings.iter()
            .find(|m| m.mapping_type == MappingType::Main)
            .or_else(|| mappings.first())
            .map(|m| m.model_id.clone());

        match model_id {
            Some(id) => id,
            None => {
                return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
                    "error": format!(
                        "❌ 模型映射未配置\n\n请先为 {} 添加模型映射：\nAI设置 → 选择 {} → 底部「模型映射」→ 添加模型 → 选择类型为 main → 输入模型ID\n\n提示：模型ID可以在提供商的官网或文档中找到，例如 MiniMax 的模型ID是 MiniMax-M2.7",
                        provider.name,
                        provider.name
                    )
                }))));
            }
        }
    };

    // 将 provider_key 转换为 SwitchBackend
    let backend_type = SwitchBackend::from_str(&provider.provider_key)
        .ok_or_else(|| -> (StatusCode, Json<serde_json::Value>) {
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": format!("Unsupported provider type: {}", provider.provider_key) })))
        })?;

    // 创建后端配置
    let config = match backend_type {
        SwitchBackend::MiniMax => BackendConfig::minimax(api_key, &model),
        SwitchBackend::OpenAI => BackendConfig::openai(
            api_key,
            &provider.base_url,
            &model,
        ),
        SwitchBackend::DeepSeek => BackendConfig::deepseek(api_key, &model),
        SwitchBackend::Zhipu => BackendConfig::zhipu(api_key, &model),
        SwitchBackend::Ollama => BackendConfig::ollama(
            provider.base_url.trim_end_matches('/'),
            &model,
        ),
    };

    // 检查 Claude Switch 是否已初始化
    let is_initialized = manager.is_claude_switch_initialized().await;
    let model_id = format!("claude-switch-{}", backend_type.as_str());

    if is_initialized {
        // 添加并切换到该后端（在 spawn_blocking 中执行，避免阻塞 async runtime）
        let manager1 = manager.clone();
        let config_clone = config.clone();
        let result = tokio::task::spawn_blocking(move || {
            manager1.add_claude_switch_backend(config_clone)
        })
        .await;

        if let Err(e) = result {
            tracing::error!("[EnableProvider] Join error: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Task join error: {}", e) }))));
        }
        if let Err(e) = result.unwrap() {
            tracing::error!("[EnableProvider] Add backend error: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))));
        }

        let manager2 = manager.clone();
        let result = tokio::task::spawn_blocking(move || {
            manager2.switch_claude_backend(backend_type)
        })
        .await;

        if let Err(e) = result {
            tracing::error!("[EnableProvider] Join error: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Task join error: {}", e) }))));
        }
        if let Err(e) = result.unwrap() {
            tracing::error!("[EnableProvider] Switch backend error: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))));
        }
    } else {
        // Claude Switch 未初始化，使用配置好的后端列表初始化
        let manager = manager.clone();
        let result = tokio::task::spawn_blocking(move || {
            manager.configure_claude_switch(vec![config])
        })
        .await;

        if let Err(e) = result {
            tracing::error!("[EnableProvider] Join error: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Task join error: {}", e) }))));
        }
        if let Err(e) = result.unwrap() {
            tracing::error!("[EnableProvider] Configure switch error: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))));
        }
    }

    // 关键：设置选中的模型为刚启用的后端
    if let Err(e) = manager.set_selected_model(&model_id) {
        tracing::warn!("[EnableProvider] Failed to set selected model {}: {}", model_id, e);
    }

    tracing::info!("[EnableProvider] Enabled provider {} with model {} (selected: {})", provider.name, model, model_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Provider {} enabled with model {}", provider.name, model),
        "provider_id": provider_id,
        "model": model
    })))
}

/// 关闭模型 - 切换回默认 Claude
pub async fn disable_provider(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let manager = &state.ai_model_manager;

    // 切换回默认的 Claude Sonnet 模型
    let default_model = "claude-sonnet-4-5";

    manager.set_selected_model(default_model)
        .map_err(|e| -> (StatusCode, Json<serde_json::Value>) {
            tracing::error!("Failed to disable provider (switch to default): {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Failed to disable: {}", e) })))
        })?;

    tracing::info!("[DisableProvider] Switched back to default model: {}", default_model);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "已切换回默认 Claude 模型",
        "model": default_model
    })))
}

/// 获取预设提供商列表
#[derive(Debug, serde::Serialize)]
pub struct PresetListResponse {
    pub presets: Vec<ProviderPreset>,
}

pub async fn get_provider_presets(
    _state: State<Arc<AppState>>,
) -> Result<Json<PresetListResponse>, (StatusCode, String)> {
    let presets = crate::services::ProviderService::get_presets();
    Ok(Json(PresetListResponse { presets }))
}

/// 从预设创建提供商请求
#[derive(Debug, serde::Deserialize)]
pub struct CreateFromPresetRequest {
    pub preset_key: String,
    pub api_key: String,
}

/// 从预设创建提供商
pub async fn create_from_preset(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateFromPresetRequest>,
) -> Result<Json<ProviderAIProvider>, (StatusCode, String)> {
    let provider = state.provider_service
        .create_from_preset(&request.preset_key, &request.api_key)
        .await
        .map_err(|e| match e {
            crate::services::ProviderServiceError::InvalidOperation(msg) => {
                (StatusCode::BAD_REQUEST, msg)
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;

    Ok(Json(provider))
}

// ============== Claude Switch Endpoints ==============

/// Claude Switch 后端信息
#[derive(Debug, serde::Serialize)]
pub struct ClaudeSwitchBackendInfo {
    pub backend: String,
    pub model: String,
    pub base_url: String,
    pub is_active: bool,
}

/// Claude Switch 配置请求
#[derive(Debug, serde::Deserialize)]
pub struct ConfigureClaudeSwitchRequest {
    pub backends: Vec<ClaudeSwitchBackendConfig>,
}

/// Claude Switch 后端配置
#[derive(Debug, serde::Deserialize)]
pub struct ClaudeSwitchBackendConfig {
    pub backend: String, // "minimax", "openai", "deepseek", "zhipu", "ollama"
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: String,
}

/// 添加 Claude Switch 后端请求
#[derive(Debug, serde::Deserialize)]
pub struct AddClaudeSwitchBackendRequest {
    pub backend: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: String,
}

/// 切换 Claude Switch 后端请求
#[derive(Debug, serde::Deserialize)]
pub struct SwitchClaudeSwitchBackendRequest {
    pub backend: String,
}

/// 配置 Claude Switch
pub async fn configure_claude_switch(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ConfigureClaudeSwitchRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let manager = &state.ai_model_manager;

    // 转换后端配置
    let backends: Result<Vec<BackendConfig>, String> = request.backends
        .iter()
        .map(|b| {
            let backend_type = SwitchBackend::from_str(&b.backend)
                .ok_or_else(|| format!("Unknown backend: {}", b.backend))?;

            let config = match backend_type {
                SwitchBackend::MiniMax => BackendConfig::minimax(b.api_key.clone(), &b.model),
                SwitchBackend::OpenAI => BackendConfig::openai(
                    b.api_key.clone(),
                    b.base_url.as_deref().unwrap_or("https://api.openai.com/v1"),
                    &b.model,
                ),
                SwitchBackend::DeepSeek => BackendConfig::deepseek(b.api_key.clone(), &b.model),
                SwitchBackend::Zhipu => BackendConfig::zhipu(b.api_key.clone(), &b.model),
                SwitchBackend::Ollama => BackendConfig::ollama(
                    b.base_url.as_deref().unwrap_or("http://localhost:11434"),
                    &b.model,
                ),
            };
            Ok(config)
        })
        .collect();

    let backends = backends.map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    manager.configure_claude_switch(backends)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Claude Switch configured successfully"
    })))
}

/// 添加 Claude Switch 后端
pub async fn add_claude_switch_backend(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AddClaudeSwitchBackendRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let manager = &state.ai_model_manager;

    let backend_type = SwitchBackend::from_str(&request.backend)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, format!("Unknown backend: {}", request.backend)))?;

    let config = match backend_type {
        SwitchBackend::MiniMax => BackendConfig::minimax(request.api_key.clone(), &request.model),
        SwitchBackend::OpenAI => BackendConfig::openai(
            request.api_key.clone(),
            request.base_url.as_deref().unwrap_or("https://api.openai.com/v1"),
            &request.model,
        ),
        SwitchBackend::DeepSeek => BackendConfig::deepseek(request.api_key.clone(), &request.model),
        SwitchBackend::Zhipu => BackendConfig::zhipu(request.api_key.clone(), &request.model),
        SwitchBackend::Ollama => BackendConfig::ollama(
            request.base_url.as_deref().unwrap_or("http://localhost:11434"),
            &request.model,
        ),
    };

    // 检查 Claude Switch 是否已初始化
    let is_initialized = manager.is_claude_switch_initialized().await;

    if is_initialized {
        // Claude Switch 已初始化，添加新后端（在 spawn_blocking 中执行）
        let manager1 = manager.clone();
        let config_clone = config.clone();
        let result = tokio::task::spawn_blocking(move || {
            manager1.add_claude_switch_backend(config_clone)
        })
        .await;

        match result {
            Ok(Ok(())) => {},
            Ok(Err(e)) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
            Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Join error: {}", e))),
        }
    } else {
        // Claude Switch 未初始化，使用配置好的后端列表初始化
        let manager = manager.clone();
        let result = tokio::task::spawn_blocking(move || {
            manager.configure_claude_switch(vec![config])
        })
        .await;

        match result {
            Ok(Ok(())) => {},
            Ok(Err(e)) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
            Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Join error: {}", e))),
        }
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Backend {} added successfully", request.backend)
    })))
}

/// 切换 Claude Switch 后端
pub async fn switch_claude_switch_backend(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SwitchClaudeSwitchBackendRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let manager = &state.ai_model_manager;

    let backend = SwitchBackend::from_str(&request.backend)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, format!("Unknown backend: {}", request.backend)))?;

    let manager1 = manager.clone();
    let result = tokio::task::spawn_blocking(move || {
        manager1.switch_claude_backend(backend)
    })
    .await;

    match result {
        Ok(Ok(())) => {},
        Ok(Err(e)) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Join error: {}", e))),
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Switched to backend {}", request.backend)
    })))
}

/// 获取 Claude Switch 后端列表
pub async fn list_claude_switch_backends(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ClaudeSwitchBackendInfo>>, (StatusCode, String)> {
    let manager = &state.ai_model_manager;

    let backends = manager.list_claude_switch_backends_async().await;

    let result: Vec<ClaudeSwitchBackendInfo> = backends
        .into_iter()
        .map(|(backend, is_active)| {
            ClaudeSwitchBackendInfo {
                backend: backend.as_str().to_string(),
                model: backend.as_str().to_string(),
                base_url: String::new(),
                is_active,
            }
        })
        .collect();

    Ok(Json(result))
}

/// 获取当前激活的 Claude Switch 后端
pub async fn get_active_claude_switch_backend(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ClaudeSwitchBackendInfo>, (StatusCode, String)> {
    let manager = &state.ai_model_manager;

    let active = manager.get_active_backend();
    let backends = manager.list_claude_switch_backends_async().await;

    let info = backends
        .into_iter()
        .find(|(_, is_active)| *is_active)
        .map(|(backend, _)| ClaudeSwitchBackendInfo {
            backend: backend.as_str().to_string(),
            model: backend.as_str().to_string(),
            base_url: String::new(),
            is_active: true,
        })
        .unwrap_or_else(|| ClaudeSwitchBackendInfo {
            backend: active.as_str().to_string(),
            model: String::new(),
            base_url: String::new(),
            is_active: true,
        });

    Ok(Json(info))
}

/// 测试 Claude Switch 后端连接请求
#[derive(Debug, serde::Deserialize)]
pub struct TestClaudeSwitchBackendRequest {
    pub backend: String,
    pub api_key: String,
    pub model: String,
}

/// 测试 Claude Switch 后端连接
pub async fn test_claude_switch_backend(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TestClaudeSwitchBackendRequest>,
) -> Result<Json<ConnectionTestResult>, (StatusCode, String)> {
    use nexus_ai::SwitchBackend;

    let backend = SwitchBackend::from_str(&request.backend)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, format!("Unknown backend: {}", request.backend)))?;

    let result = state.ai_model_manager
        .test_claude_switch_backend(backend, &request.api_key, &request.model)
        .await;

    match result {
        Ok(_) => Ok(Json(ConnectionTestResult {
            success: true,
            message: "Connection successful".to_string(),
            models: Some(vec![request.model]),
        })),
        Err(e) => Ok(Json(ConnectionTestResult {
            success: false,
            message: e.to_string(),
            models: None,
        })),
    }
}