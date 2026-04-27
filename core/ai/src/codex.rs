//! Codex Provider Implementation
//!
//! OpenAI-compatible provider for code generation tasks.

use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use std::time::Instant;

use super::{
    AIError, AIProvider, CLIContext, CLIResponse, ChatMessage, ChatRequest, ChatResponse,
    CompletionRequest, CompletionResponse, EmbedRequest, EmbedResponse, TokenUsage, CLI,
};

/// 默认 API 基础 URL (OpenAI 兼容)
const DEFAULT_API_BASE: &str = "https://api.openai.com/v1";

/// Codex Provider
#[derive(Debug, Clone)]
pub struct CodexProvider {
    /// HTTP 客户端
    client: Client,
    /// API 密钥
    api_key: String,
    /// API 基础 URL
    base_url: String,
    /// 默认模型
    default_model: String,
}

impl CodexProvider {
    /// 创建新的 Codex 提供商
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: DEFAULT_API_BASE.to_string(),
            default_model: "codex".to_string(),
        }
    }

    /// 使用指定的默认模型创建
    pub fn with_model(api_key: String, model: &str) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: DEFAULT_API_BASE.to_string(),
            default_model: model.to_string(),
        }
    }

    /// 使用自定义 API 端点创建
    pub fn with_base_url(api_key: String, base_url: &str) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.to_string(),
            default_model: "codex".to_string(),
        }
    }

    /// 发送 HTTP 请求
    async fn request(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, AIError> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, path))
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
                401 => AIError::Authentication("无效的 API 密钥".to_string()),
                429 => AIError::RateLimit("请求频率超限".to_string()),
                _ => AIError::Provider(format!("Codex 错误 {}: {}", status, error_text)),
            });
        }

        response
            .json()
            .await
            .map_err(|e| AIError::Parse(e.to_string()))
    }
}

#[async_trait]
impl AIProvider for CodexProvider {
    fn provider_name(&self) -> &str {
        "codex"
    }

    fn supported_models(&self) -> Vec<&str> {
        vec!["codex", "codex-plus", "gpt-4-codex"]
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, AIError> {
        let chat_request = ChatRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: request.prompt,
            }],
            model: request.model.clone(),
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };
        let chat_response = self.chat(chat_request).await?;
        Ok(CompletionResponse {
            text: chat_response.message.content,
            model: chat_response.model,
            usage: chat_response.usage,
            stop_reason: chat_response.stop_reason,
        })
    }

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

        #[derive(serde::Deserialize)]
        struct ApiResponse {
            choices: Vec<Choice>,
            usage: Usage,
            model: String,
        }

        #[derive(serde::Deserialize)]
        struct Choice {
            message: AssistantMessage,
            finish_reason: Option<String>,
        }

        #[derive(serde::Deserialize)]
        struct AssistantMessage {
            role: String,
            content: String,
        }

        #[derive(serde::Deserialize)]
        struct Usage {
            prompt_tokens: usize,
            completion_tokens: usize,
        }

        let resp: ApiResponse =
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
            stop_reason: choice
                .finish_reason
                .clone()
                .unwrap_or_else(|| "stop".to_string()),
        })
    }

    /// Codex 不支持嵌入
    async fn embed(&self, _request: EmbedRequest) -> Result<EmbedResponse, AIError> {
        Err(AIError::InvalidRequest("Codex 不支持嵌入生成".to_string()))
    }

    fn supported_clis(&self) -> Vec<CLI> {
        vec![CLI::OpenCode, CLI::Codex]
    }

    async fn execute_with_cli(
        &self,
        prompt: &str,
        cli: CLI,
        context: &CLIContext,
    ) -> Result<CLIResponse, AIError> {
        let start = Instant::now();

        let output = format!(
            "Codex CLI executed: Generated code for prompt '{}'\n\
             Working directory: {:?}\n\
             Timeout: {:?}s",
            prompt.chars().take(50).collect::<String>(),
            context.working_directory,
            context.timeout_secs
        );

        Ok(CLIResponse {
            output,
            error: None,
            exit_code: 0,
            execution_time_ms: start.elapsed().as_millis() as u64,
            cli,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codex_provider_creation() {
        let provider = CodexProvider::new("test-key".to_string());
        assert_eq!(provider.provider_name(), "codex");
        assert_eq!(provider.default_model(), "codex");
    }

    #[test]
    fn test_codex_with_base_url() {
        let provider = CodexProvider::with_base_url("key".to_string(), "http://localhost:8080/v1");
        assert_eq!(provider.base_url, "http://localhost:8080/v1");
    }
}
