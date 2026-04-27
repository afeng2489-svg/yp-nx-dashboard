//! OpenAI (GPT-4) Provider 实现

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{
    AIError, AIProvider, ChatMessage, ChatRequest, ChatResponse, CompletionRequest,
    CompletionResponse, EmbedRequest, EmbedResponse, TokenUsage,
};

/// OpenAI API 基础 URL
const OPENAI_API_BASE: &str = "https://api.openai.com/v1";

/// OpenAI AI 提供商结构体
#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    /// HTTP 客户端
    client: Client,
    /// API 密钥
    api_key: String,
    /// 默认模型
    default_model: String,
}

impl OpenAIProvider {
    /// 使用 API 密钥创建新的 OpenAI 提供商
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            default_model: "gpt-4-turbo".to_string(),
        }
    }

    /// 使用指定的默认模型创建 OpenAI 提供商
    pub fn with_model(api_key: String, model: &str) -> Self {
        Self {
            client: Client::new(),
            api_key,
            default_model: model.to_string(),
        }
    }

    /// 发送 HTTP 请求到 OpenAI API
    async fn request(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, AIError> {
        let response = self
            .client
            .post(&format!("{}{}", OPENAI_API_BASE, path))
            .header("authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AIError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => AIError::Authentication("无效的 OpenAI API 密钥".to_string()),
                429 => AIError::RateLimit("请求频率超限".to_string()),
                _ => AIError::Provider(format!("OpenAI 错误 {}: {}", status, error_text)),
            });
        }

        response
            .json()
            .await
            .map_err(|e| AIError::Parse(e.to_string()))
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    /// 获取提供商名称
    fn provider_name(&self) -> &str {
        "openai"
    }

    /// 获取支持的模型列表
    fn supported_models(&self) -> Vec<&str> {
        vec![
            "gpt-4-turbo",
            "gpt-4o",
            "gpt-4o-mini",
            "gpt-4",
            "gpt-3.5-turbo",
        ]
    }

    /// 获取默认模型
    fn default_model(&self) -> &str {
        &self.default_model
    }

    /// 执行文本补全
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, AIError> {
        #[derive(Serialize)]
        struct CompletionRequestBody {
            model: String,
            prompt: String,
            max_tokens: usize,
            temperature: f32,
            stop: Vec<String>,
        }

        let body = CompletionRequestBody {
            model: request.model.clone(),
            prompt: request.prompt,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stop: request.stop_sequences,
        };

        let response = self
            .request("/completions", serde_json::to_value(body).unwrap())
            .await?;

        #[derive(Deserialize)]
        struct OpenAIResponse {
            choices: Vec<Choice>,
            usage: Usage,
            model: String,
        }

        #[derive(Deserialize)]
        struct Choice {
            text: String,
            finish_reason: String,
        }

        #[derive(Deserialize)]
        struct Usage {
            prompt_tokens: usize,
            completion_tokens: usize,
            total_tokens: usize,
        }

        let resp: OpenAIResponse =
            serde_json::from_value(response).map_err(|e| AIError::Parse(e.to_string()))?;

        Ok(CompletionResponse {
            text: resp
                .choices
                .first()
                .map(|c| c.text.clone())
                .unwrap_or_default(),
            model: resp.model,
            usage: TokenUsage {
                input_tokens: resp.usage.prompt_tokens,
                output_tokens: resp.usage.completion_tokens,
            },
            stop_reason: resp
                .choices
                .first()
                .map(|c| c.finish_reason.clone())
                .unwrap_or_default(),
        })
    }

    /// 执行聊天补全
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AIError> {
        #[derive(Serialize)]
        struct ChatRequestBody {
            model: String,
            messages: Vec<ChatMessage>,
            max_tokens: usize,
            temperature: f32,
        }

        let body = ChatRequestBody {
            model: request.model.clone(),
            messages: request.messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };

        let response = self
            .request("/chat/completions", serde_json::to_value(body).unwrap())
            .await?;

        #[derive(Deserialize)]
        struct OpenAIResponse {
            choices: Vec<Choice>,
            usage: Usage,
            model: String,
        }

        #[derive(Deserialize)]
        struct Choice {
            message: AssistantMessage,
            finish_reason: String,
        }

        #[derive(Deserialize)]
        struct AssistantMessage {
            role: String,
            content: String,
        }

        #[derive(Deserialize)]
        struct Usage {
            prompt_tokens: usize,
            completion_tokens: usize,
            total_tokens: usize,
        }

        let resp: OpenAIResponse =
            serde_json::from_value(response).map_err(|e| AIError::Parse(e.to_string()))?;

        let choice = resp
            .choices
            .first()
            .ok_or_else(|| AIError::Provider("响应中没有选项".to_string()))?;

        Ok(ChatResponse {
            message: ChatMessage {
                role: choice.message.role.clone(),
                content: choice.message.content.clone(),
            },
            model: resp.model,
            usage: TokenUsage {
                input_tokens: resp.usage.prompt_tokens,
                output_tokens: resp.usage.completion_tokens,
            },
            stop_reason: choice.finish_reason.clone(),
        })
    }

    /// 生成嵌入向量
    async fn embed(&self, request: EmbedRequest) -> Result<EmbedResponse, AIError> {
        #[derive(Serialize)]
        struct EmbedRequestBody {
            model: String,
            input: Vec<String>,
        }

        let body = EmbedRequestBody {
            model: request.model.clone(),
            input: request.texts,
        };

        let response = self
            .request("/embeddings", serde_json::to_value(body).unwrap())
            .await?;

        #[derive(Deserialize)]
        struct OpenAIResponse {
            data: Vec<EmbeddingData>,
            usage: Usage,
            model: String,
        }

        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
        }

        #[derive(Deserialize)]
        struct Usage {
            prompt_tokens: usize,
            total_tokens: usize,
        }

        let resp: OpenAIResponse =
            serde_json::from_value(response).map_err(|e| AIError::Parse(e.to_string()))?;

        Ok(EmbedResponse {
            embeddings: resp.data.into_iter().map(|d| d.embedding).collect(),
            model: resp.model,
            usage: TokenUsage {
                input_tokens: resp.usage.prompt_tokens,
                output_tokens: resp.usage.total_tokens,
            },
        })
    }
}
