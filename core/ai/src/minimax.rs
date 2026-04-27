//! MiniMax AI Provider 实现
//!
//! MiniMax API 文档: https://www.minimaxi.com/document

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::{
    AIError, AIProvider, ChatMessage, ChatRequest, ChatResponse, CompletionRequest,
    CompletionResponse, EmbedRequest, EmbedResponse, TokenUsage,
};

/// MiniMax API 基础 URL
const MINIMAX_API_BASE: &str = "https://api.minimax.chat/v1";

/// MiniMax AI 提供商结构体
#[derive(Debug, Clone)]
pub struct MiniMaxProvider {
    /// HTTP 客户端
    client: Client,
    /// API 密钥
    api_key: String,
    /// 基础 URL
    base_url: String,
    /// 默认模型
    default_model: String,
}

impl MiniMaxProvider {
    /// 使用 API 密钥创建新的 MiniMax 提供商
    pub fn new(api_key: String) -> Self {
        Self::with_model(api_key, "MiniMax-M2.7".to_string())
    }

    /// 使用指定的默认模型创建 MiniMax 提供商
    pub fn with_model(api_key: String, model: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(300))
                .build()
                .unwrap_or_else(|_| Client::new()),
            api_key,
            base_url: MINIMAX_API_BASE.to_string(),
            default_model: model,
        }
    }

    /// 使用自定义端点创建（用于 OpenAI 兼容的第三方 API）
    pub fn with_custom_endpoint(api_key: String, base_url: &str, model: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(300))
                .build()
                .unwrap_or_else(|_| Client::new()),
            api_key,
            base_url: base_url.to_string(),
            default_model: model.to_string(),
        }
    }

    /// 发送 HTTP 请求到 MiniMax API
    async fn request(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, AIError> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AIError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => AIError::Authentication("无效的 MiniMax API 密钥".to_string()),
                429 => AIError::RateLimit("请求频率超限".to_string()),
                _ => AIError::Provider(format!("MiniMax 错误 {}: {}", status, error_text)),
            });
        }

        response
            .json()
            .await
            .map_err(|e| AIError::Parse(e.to_string()))
    }
}

#[async_trait]
impl AIProvider for MiniMaxProvider {
    /// 获取提供商名称
    fn provider_name(&self) -> &str {
        "minimax"
    }

    /// 获取支持的模型列表
    fn supported_models(&self) -> Vec<&str> {
        vec![
            "MiniMax-M2.7",
            "MiniMax-M2",
            "abab6-chat",
            "abab6-gs",
            "doubao-seed",
            "bailing-chat",
        ]
    }

    /// 检查是否支持指定模型
    fn supports_model(&self, model: &str) -> bool {
        self.supported_models().contains(&model)
    }

    /// 检查是否支持 CLI
    fn supports_cli(&self, _cli: super::CLI) -> bool {
        false // MiniMax 不支持 CLI
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    /// 执行聊天补全
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AIError> {
        #[derive(Serialize)]
        struct MiniMaxChatRequest {
            model: String,
            messages: Vec<MiniMaxChatMessage>,
            temperature: Option<f32>,
            max_tokens: Option<usize>,
            stream: bool,
        }

        #[derive(Serialize)]
        struct MiniMaxChatMessage {
            role: String,
            content: String,
        }

        let minimax_messages: Vec<MiniMaxChatMessage> = request
            .messages
            .into_iter()
            .map(|m| MiniMaxChatMessage {
                role: m.role,
                content: m.content,
            })
            .collect();

        let body = MiniMaxChatRequest {
            model: request.model.clone(),
            messages: minimax_messages,
            temperature: Some(request.temperature),
            max_tokens: Some(request.max_tokens),
            stream: false,
        };

        let response = self
            .request(
                "/chat/completions",
                serde_json::to_value(body).map_err(|e| AIError::Parse(e.to_string()))?,
            )
            .await?;

        #[derive(Deserialize)]
        struct MiniMaxChatChoice {
            message: MiniMaxChatMessageOut,
        }

        #[derive(Deserialize)]
        struct MiniMaxChatResponse {
            choices: Vec<MiniMaxChatChoice>,
            usage: MiniMaxUsage,
            model: String,
        }

        #[derive(Deserialize)]
        struct MiniMaxChatMessageOut {
            role: String,
            content: String,
        }

        #[derive(Deserialize)]
        struct MiniMaxUsage {
            prompt_tokens: usize,
            completion_tokens: usize,
            #[allow(dead_code)]
            total_tokens: usize,
        }

        let response: MiniMaxChatResponse = serde_json::from_value(response)
            .map_err(|e| AIError::Parse(format!("解析 MiniMax 响应失败: {}", e)))?;

        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AIError::Provider("MiniMax 返回空响应".to_string()))?;

        Ok(ChatResponse {
            message: ChatMessage {
                role: choice.message.role,
                content: choice.message.content,
            },
            model: response.model,
            usage: TokenUsage {
                input_tokens: response.usage.prompt_tokens,
                output_tokens: response.usage.completion_tokens,
            },
            stop_reason: "stop".to_string(),
        })
    }

    /// 执行文本补全
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, AIError> {
        #[derive(Serialize)]
        struct MiniMaxCompletionRequest {
            model: String,
            prompt: String,
            temperature: Option<f32>,
            max_tokens: Option<usize>,
            stream: bool,
        }

        let body = MiniMaxCompletionRequest {
            model: request.model.clone(),
            prompt: request.prompt,
            temperature: Some(request.temperature),
            max_tokens: Some(request.max_tokens),
            stream: false,
        };

        let response = self
            .request(
                "/completions",
                serde_json::to_value(body).map_err(|e| AIError::Parse(e.to_string()))?,
            )
            .await?;

        #[derive(Deserialize)]
        struct MiniMaxCompletionResponse {
            choices: Vec<MiniMaxCompletionChoice>,
            usage: MiniMaxCompletionUsage,
            model: String,
        }

        #[derive(Deserialize)]
        struct MiniMaxCompletionChoice {
            text: String,
        }

        #[derive(Deserialize)]
        struct MiniMaxCompletionUsage {
            prompt_tokens: usize,
            completion_tokens: usize,
            #[allow(dead_code)]
            total_tokens: usize,
        }

        let response: MiniMaxCompletionResponse = serde_json::from_value(response)
            .map_err(|e| AIError::Parse(format!("解析 MiniMax 响应失败: {}", e)))?;

        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AIError::Provider("MiniMax 返回空响应".to_string()))?;

        Ok(CompletionResponse {
            text: choice.text,
            model: response.model,
            usage: TokenUsage {
                input_tokens: response.usage.prompt_tokens,
                output_tokens: response.usage.completion_tokens,
            },
            stop_reason: "stop".to_string(),
        })
    }

    /// 执行嵌入请求
    async fn embed(&self, _request: EmbedRequest) -> Result<EmbedResponse, AIError> {
        Err(AIError::Provider("MiniMax 不支持嵌入 API".to_string()))
    }
}
