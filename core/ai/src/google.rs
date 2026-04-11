//! Google (Gemini) Provider 实现

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{AIError, AIProvider, ChatMessage, ChatRequest, ChatResponse, CompletionRequest, CompletionResponse, EmbedRequest, EmbedResponse, TokenUsage};

/// Google Generative Language API 基础 URL
const GOOGLE_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Google AI 提供商结构体
#[derive(Debug, Clone)]
pub struct GoogleProvider {
    /// HTTP 客户端
    client: Client,
    /// API 密钥
    api_key: String,
    /// 默认模型
    default_model: String,
}

impl GoogleProvider {
    /// 使用 API 密钥创建新的 Google 提供商
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            default_model: "gemini-pro".to_string(),
        }
    }

    /// 使用指定的默认模型创建 Google 提供商
    pub fn with_model(api_key: String, model: &str) -> Self {
        Self {
            client: Client::new(),
            api_key,
            default_model: model.to_string(),
        }
    }

    /// 发送 HTTP 请求到 Google API
    async fn request(&self, path: &str, body: serde_json::Value) -> Result<serde_json::Value, AIError> {
        let url = format!("{}{}?key={}", GOOGLE_API_BASE, path, self.api_key);
        let response = self.client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AIError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 | 403 => AIError::Authentication("无效的 Google API 密钥".to_string()),
                429 => AIError::RateLimit("请求频率超限".to_string()),
                _ => AIError::Provider(format!("Google 错误 {}: {}", status, error_text)),
            });
        }

        response.json().await.map_err(|e| AIError::Parse(e.to_string()))
    }
}

#[async_trait]
impl AIProvider for GoogleProvider {
    /// 获取提供商名称
    fn provider_name(&self) -> &str {
        "google"
    }

    /// 获取支持的模型列表
    fn supported_models(&self) -> Vec<&str> {
        vec![
            "gemini-pro",
            "gemini-pro-vision",
            "gemini-1.5-pro",
            "gemini-1.5-flash",
        ]
    }

    /// 获取默认模型
    fn default_model(&self) -> &str {
        &self.default_model
    }

    /// 执行文本补全
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, AIError> {
        #[derive(Serialize)]
        struct GenerateContentRequest {
            contents: Vec<Content>,
            generation_config: GenerationConfig,
        }

        #[derive(Serialize)]
        struct Content {
            parts: Vec<Part>,
        }

        #[derive(Serialize)]
        struct Part {
            text: String,
        }

        #[derive(Serialize, Deserialize)]
        struct GenerationConfig {
            max_output_tokens: usize,
            temperature: f32,
        }

        let body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part { text: request.prompt }],
            }],
            generation_config: GenerationConfig {
                max_output_tokens: request.max_tokens,
                temperature: request.temperature,
            },
        };

        let model = request.model.trim_start_matches("gemini-");
        let path = format!("/models/{}:generateContent", model);
        let response = self.request(&path, serde_json::to_value(body).unwrap()).await?;

        #[derive(Deserialize)]
        struct GeminiResponse {
            candidates: Vec<Candidate>,
            usage_metadata: Option<UsageMetadata>,
        }

        #[derive(Deserialize)]
        struct Candidate {
            content: ContentResponse,
            finish_reason: String,
        }

        #[derive(Deserialize)]
        struct ContentResponse {
            parts: Vec<PartResponse>,
        }

        #[derive(Deserialize)]
        struct PartResponse {
            text: Option<String>,
        }

        #[derive(Deserialize)]
        struct UsageMetadata {
            prompt_token_count: Option<usize>,
            candidates_token_count: Option<usize>,
            total_token_count: Option<usize>,
        }

        let resp: GeminiResponse = serde_json::from_value(response)
            .map_err(|e| AIError::Parse(e.to_string()))?;

        let candidate = resp.candidates.first().ok_or_else(|| AIError::Provider("响应中没有候选".to_string()))?;
        let text = candidate.content.parts.first().and_then(|p| p.text.clone()).unwrap_or_default();
        let prompt_tokens = resp.usage_metadata.as_ref().map(|u| u.prompt_token_count.unwrap_or(0)).unwrap_or(0);
        let output_tokens = resp.usage_metadata.as_ref().map(|u| u.candidates_token_count.unwrap_or(0)).unwrap_or(0);

        Ok(CompletionResponse {
            text,
            model: request.model,
            usage: TokenUsage {
                input_tokens: prompt_tokens,
                output_tokens,
            },
            stop_reason: candidate.finish_reason.clone(),
        })
    }

    /// 执行聊天补全
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AIError> {
        #[derive(Serialize)]
        struct GenerateContentRequest {
            contents: Vec<Content>,
            generation_config: GenerationConfig,
        }

        #[derive(Serialize)]
        struct Content {
            role: String,
            parts: Vec<Part>,
        }

        #[derive(Serialize)]
        struct Part {
            text: String,
        }

        #[derive(Serialize)]
        struct GenerationConfig {
            max_output_tokens: usize,
            temperature: f32,
        }

        let contents: Vec<Content> = request.messages.into_iter().map(|m| Content {
            role: m.role,
            parts: vec![Part { text: m.content }],
        }).collect();

        let body = GenerateContentRequest {
            contents,
            generation_config: GenerationConfig {
                max_output_tokens: request.max_tokens,
                temperature: request.temperature,
            },
        };

        let model = request.model.trim_start_matches("gemini-");
        let path = format!("/models/{}:generateContent", model);
        let response = self.request(&path, serde_json::to_value(body).unwrap()).await?;

        #[derive(Deserialize)]
        struct GeminiResponse {
            candidates: Vec<Candidate>,
            usage_metadata: Option<UsageMetadata>,
        }

        #[derive(Deserialize)]
        struct Candidate {
            content: ContentResponse,
            finish_reason: String,
        }

        #[derive(Deserialize)]
        struct ContentResponse {
            parts: Vec<PartResponse>,
        }

        #[derive(Deserialize)]
        struct PartResponse {
            text: Option<String>,
        }

        #[derive(Deserialize)]
        struct UsageMetadata {
            prompt_token_count: Option<usize>,
            candidates_token_count: Option<usize>,
        }

        let resp: GeminiResponse = serde_json::from_value(response)
            .map_err(|e| AIError::Parse(e.to_string()))?;

        let candidate = resp.candidates.first().ok_or_else(|| AIError::Provider("响应中没有候选".to_string()))?;
        let text = candidate.content.parts.first().and_then(|p| p.text.clone()).unwrap_or_default();
        let prompt_tokens = resp.usage_metadata.as_ref().map(|u| u.prompt_token_count.unwrap_or(0)).unwrap_or(0);
        let output_tokens = resp.usage_metadata.as_ref().map(|u| u.candidates_token_count.unwrap_or(0)).unwrap_or(0);

        Ok(ChatResponse {
            message: ChatMessage {
                role: "assistant".to_string(),
                content: text,
            },
            model: request.model,
            usage: TokenUsage {
                input_tokens: prompt_tokens,
                output_tokens,
            },
            stop_reason: candidate.finish_reason.clone(),
        })
    }

    /// 生成嵌入向量
    async fn embed(&self, request: EmbedRequest) -> Result<EmbedResponse, AIError> {
        #[derive(Serialize)]
        struct EmbedRequestBody {
            content: String,
        }

        let texts = request.texts;
        let mut embeddings = Vec::with_capacity(texts.len());

        for text in texts {
            let body = EmbedRequestBody { content: text };
            let response = self.request("/models/embedding-001:embedContent", serde_json::to_value(body).unwrap()).await?;

            #[derive(Deserialize)]
            struct EmbedResponse {
                embedding: EmbeddingValues,
            }

            #[derive(Deserialize)]
            struct EmbeddingValues {
                values: Vec<f32>,
            }

            let resp: EmbedResponse = serde_json::from_value(response)
                .map_err(|e| AIError::Parse(e.to_string()))?;
            embeddings.push(resp.embedding.values);
        }

        Ok(EmbedResponse {
            embeddings,
            model: request.model,
            usage: TokenUsage {
                input_tokens: 0,
                output_tokens: 0,
            },
        })
    }
}
