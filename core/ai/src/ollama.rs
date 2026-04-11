//! Ollama (本地模型) Provider 实现

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::{AIError, AIProvider, ChatMessage, ChatRequest, ChatResponse, CompletionRequest, CompletionResponse, EmbedRequest, EmbedResponse, TokenUsage};

/// Ollama API 基础 URL
const OLLAMA_API_BASE: &str = "http://localhost:11434/api";

/// Ollama 提供商结构体 (用于本地模型)
#[derive(Debug, Clone)]
pub struct OllamaProvider {
    /// HTTP 客户端
    client: Client,
    /// 基础 URL
    base_url: String,
    /// 默认模型
    default_model: String,
}

impl OllamaProvider {
    /// 创建新的 Ollama 提供商
    pub fn new() -> Self {
        Self::with_base_url("http://localhost:11434")
    }

    /// 使用指定的基础 URL 创建 Ollama 提供商
    pub fn with_base_url(base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(300))
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url: base_url.to_string(),
            default_model: "llama3".to_string(),
        }
    }

    /// 使用指定的默认模型创建 Ollama 提供商
    pub fn with_model(model: &str) -> Self {
        let mut provider = Self::new();
        provider.default_model = model.to_string();
        provider
    }

    /// 发送 HTTP 请求到 Ollama API
    async fn request<T: for<'de> Deserialize<'de>>(&self, path: &str, body: Option<serde_json::Value>) -> Result<T, AIError> {
        let url = format!("{}{}", self.base_url, path);
        let response = match body {
            Some(b) => {
                self.client
                    .post(&url)
                    .header("content-type", "application/json")
                    .json(&b)
                    .send()
                    .await
            }
            None => {
                self.client.get(&url).send().await
            }
        }.map_err(|e| AIError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AIError::Provider(format!("Ollama 错误 {}: {}", status, error_text)));
        }

        response.json().await.map_err(|e| AIError::Parse(e.to_string()))
    }

    /// 从 Ollama 获取可用的模型列表
    pub async fn list_models(&self) -> Result<Vec<OllamaModel>, AIError> {
        #[derive(Deserialize)]
        struct ListResponse {
            models: Vec<OllamaModel>,
        }

        let response: ListResponse = self.request("/tags", None).await?;
        Ok(response.models)
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Ollama 模型信息
#[derive(Deserialize, Debug)]
pub struct OllamaModel {
    pub name: String,
    pub model: String,
    pub size: u64,
    pub modified_at: String,
}

#[async_trait]
impl AIProvider for OllamaProvider {
    /// 获取提供商名称
    fn provider_name(&self) -> &str {
        "ollama"
    }

    /// 获取支持的模型列表
    fn supported_models(&self) -> Vec<&str> {
        vec![
            "llama3",
            "llama3.1",
            "llama3.2",
            "codellama",
            "mistral",
            "mixtral",
            "neural-chat",
            "phi3",
            "qwen2",
            "wizardcoder",
        ]
    }

    /// 获取默认模型
    fn default_model(&self) -> &str {
        &self.default_model
    }

    /// 执行文本补全
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, AIError> {
        #[derive(Serialize)]
        struct GenerateRequest {
            model: String,
            prompt: String,
            stream: bool,
            options: GenerateOptions,
        }

        #[derive(Serialize)]
        struct GenerateOptions {
            temperature: f32,
            num_predict: usize,
        }

        let body = GenerateRequest {
            model: request.model.clone(),
            prompt: request.prompt,
            stream: false,
            options: GenerateOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
            },
        };

        #[derive(Deserialize)]
        struct GenerateResponse {
            response: String,
            context: Option<Vec<i32>>,
            total_duration: Option<u64>,
            eval_count: Option<usize>,
            prompt_eval_count: Option<usize>,
        }

        let response: GenerateResponse = self.request("/generate", Some(serde_json::to_value(body).unwrap())).await?;

        Ok(CompletionResponse {
            text: response.response,
            model: request.model,
            usage: TokenUsage {
                input_tokens: response.prompt_eval_count.unwrap_or(0),
                output_tokens: response.eval_count.unwrap_or(0),
            },
            stop_reason: "stop".to_string(),
        })
    }

    /// 执行聊天补全
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AIError> {
        #[derive(Serialize)]
        struct ChatRequestBody {
            model: String,
            messages: Vec<ChatMessage>,
            stream: bool,
            options: ChatOptions,
        }

        #[derive(Serialize)]
        struct ChatOptions {
            temperature: f32,
            num_predict: usize,
        }

        let body = ChatRequestBody {
            model: request.model.clone(),
            messages: request.messages,
            stream: false,
            options: ChatOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
            },
        };

        #[derive(Deserialize)]
        struct ChatResponseBody {
            message: AssistantMessage,
            total_duration: Option<u64>,
            eval_count: Option<usize>,
            prompt_eval_count: Option<usize>,
        }

        #[derive(Deserialize)]
        struct AssistantMessage {
            role: String,
            content: String,
        }

        let response: ChatResponseBody = self.request("/chat", Some(serde_json::to_value(body).unwrap())).await?;

        Ok(ChatResponse {
            message: ChatMessage {
                role: response.message.role,
                content: response.message.content,
            },
            model: request.model,
            usage: TokenUsage {
                input_tokens: response.prompt_eval_count.unwrap_or(0),
                output_tokens: response.eval_count.unwrap_or(0),
            },
            stop_reason: "stop".to_string(),
        })
    }

    /// 生成嵌入向量 (通过 /api/embeddings 端点)
    async fn embed(&self, request: EmbedRequest) -> Result<EmbedResponse, AIError> {
        #[derive(Serialize)]
        struct EmbeddingsRequest {
            model: String,
            prompt: String,
        }

        #[derive(Deserialize)]
        struct EmbeddingsResponse {
            embedding: Vec<f32>,
        }

        let mut embeddings = Vec::with_capacity(request.texts.len());
        let mut total_input_tokens = 0;

        for text in &request.texts {
            let body = EmbeddingsRequest {
                model: request.model.clone(),
                prompt: text.clone(),
            };

            let response: EmbeddingsResponse = self.request("/embeddings", Some(serde_json::to_value(body).unwrap())).await?;
            embeddings.push(response.embedding);
            total_input_tokens += text.len() / 4; // 粗略估算
        }

        Ok(EmbedResponse {
            embeddings,
            model: request.model,
            usage: TokenUsage {
                input_tokens: total_input_tokens,
                output_tokens: 0,
            },
        })
    }
}
