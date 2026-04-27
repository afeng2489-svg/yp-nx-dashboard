//! Anthropic (Claude) Provider 实现

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{
    AIError, AIProvider, ChatMessage, ChatRequest, ChatResponse, CompletionRequest,
    CompletionResponse, EmbedRequest, EmbedResponse, TokenUsage,
};

/// Anthropic API 基础 URL
const ANTHROPIC_API_BASE: &str = "https://api.anthropic.com/v1";

/// Anthropic AI 提供商结构体
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    /// HTTP 客户端
    client: Client,
    /// API 密钥
    api_key: String,
    /// 默认模型
    default_model: String,
}

impl AnthropicProvider {
    /// 使用 API 密钥创建新的 Anthropic 提供商
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            default_model: "claude-sonnet-4-5".to_string(),
        }
    }

    /// 使用指定的默认模型创建 Anthropic 提供商
    pub fn with_model(api_key: String, model: &str) -> Self {
        Self {
            client: Client::new(),
            api_key,
            default_model: model.to_string(),
        }
    }

    /// 发送 HTTP 请求到 Anthropic API
    async fn request(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, AIError> {
        let response = self
            .client
            .post(format!("{}{}", ANTHROPIC_API_BASE, path))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AIError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => AIError::Authentication("无效的 Anthropic API 密钥".to_string()),
                429 => AIError::RateLimit("请求频率超限".to_string()),
                _ => AIError::Provider(format!("Anthropic 错误 {}: {}", status, error_text)),
            });
        }

        response
            .json()
            .await
            .map_err(|e| AIError::Parse(e.to_string()))
    }
}

#[async_trait]
impl AIProvider for AnthropicProvider {
    /// 获取提供商名称
    fn provider_name(&self) -> &str {
        "anthropic"
    }

    /// 获取支持的模型列表
    fn supported_models(&self) -> Vec<&str> {
        vec![
            "claude-opus-4-5",
            "claude-opus-4-5-20251101",
            "claude-sonnet-4-5",
            "claude-sonnet-4-5-20251101",
            "claude-haiku-4-5",
            "claude-haiku-4-5-20251101",
        ]
    }

    /// 获取默认模型
    fn default_model(&self) -> &str {
        &self.default_model
    }

    /// 执行文本补全
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, AIError> {
        #[derive(Serialize)]
        struct MessagesRequest {
            model: String,
            max_tokens: usize,
            temperature: f32,
            prompt: String,
            stop_sequences: Vec<String>,
        }

        let body = MessagesRequest {
            model: request.model.clone(),
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            prompt: request.prompt,
            stop_sequences: request.stop_sequences,
        };

        let response = self
            .request("/messages", serde_json::to_value(body).unwrap())
            .await?;

        #[derive(Deserialize)]
        struct AnthropicResponse {
            content: Vec<ContentBlock>,
            usage: Usage,
            stop_reason: String,
            model: String,
        }

        #[derive(Deserialize)]
        struct ContentBlock {
            text: String,
        }

        #[derive(Deserialize)]
        struct Usage {
            input_tokens: usize,
            output_tokens: usize,
        }

        let resp: AnthropicResponse =
            serde_json::from_value(response).map_err(|e| AIError::Parse(e.to_string()))?;

        Ok(CompletionResponse {
            text: resp
                .content
                .first()
                .map(|c| c.text.clone())
                .unwrap_or_default(),
            model: resp.model,
            usage: TokenUsage {
                input_tokens: resp.usage.input_tokens,
                output_tokens: resp.usage.output_tokens,
            },
            stop_reason: resp.stop_reason,
        })
    }

    /// 执行聊天补全
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AIError> {
        #[derive(Serialize)]
        struct AnthropicChatRequest {
            model: String,
            max_tokens: usize,
            temperature: f32,
            messages: Vec<AnthropicMessage>,
        }

        #[derive(Serialize)]
        struct AnthropicMessage {
            role: String,
            content: String,
        }

        let body = AnthropicChatRequest {
            model: request.model.clone(),
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            messages: request
                .messages
                .into_iter()
                .map(|m| AnthropicMessage {
                    role: m.role,
                    content: m.content,
                })
                .collect(),
        };

        let response = self
            .request("/messages", serde_json::to_value(body).unwrap())
            .await?;

        #[derive(Deserialize)]
        struct AnthropicResponse {
            content: Vec<ContentBlock>,
            usage: Usage,
            stop_reason: String,
            model: String,
        }

        #[derive(Deserialize)]
        struct ContentBlock {
            text: String,
        }

        #[derive(Deserialize)]
        struct Usage {
            input_tokens: usize,
            output_tokens: usize,
        }

        let resp: AnthropicResponse =
            serde_json::from_value(response).map_err(|e| AIError::Parse(e.to_string()))?;

        Ok(ChatResponse {
            message: ChatMessage {
                role: "assistant".to_string(),
                content: resp
                    .content
                    .first()
                    .map(|c| c.text.clone())
                    .unwrap_or_default(),
            },
            model: resp.model,
            usage: TokenUsage {
                input_tokens: resp.usage.input_tokens,
                output_tokens: resp.usage.output_tokens,
            },
            stop_reason: resp.stop_reason,
        })
    }

    /// Anthropic 不支持嵌入生成
    async fn embed(&self, _request: EmbedRequest) -> Result<EmbedResponse, AIError> {
        Err(AIError::InvalidRequest(
            "Anthropic 不支持嵌入生成".to_string(),
        ))
    }
}
