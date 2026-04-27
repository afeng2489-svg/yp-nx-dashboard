//! Claude Switch - 动态模型切换适配器
//!
//! 将 Claude 格式的请求转换为其他 AI 提供商的格式，
//! 实现"用 Claude 接口，调用任意后端"的功能。

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

use super::{
    AIError, AIProvider, ChatMessage, ChatRequest, ChatResponse, CompletionRequest,
    CompletionResponse, EmbedRequest, EmbedResponse, TokenUsage,
};

/// Claude Switch 支持的后端提供商
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwitchBackend {
    MiniMax,
    OpenAI,
    DeepSeek,
    Zhipu,
    Ollama,
}

impl SwitchBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            SwitchBackend::MiniMax => "minimax",
            SwitchBackend::OpenAI => "openai",
            SwitchBackend::DeepSeek => "deepseek",
            SwitchBackend::Zhipu => "zhipu",
            SwitchBackend::Ollama => "ollama",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "minimax" => Some(SwitchBackend::MiniMax),
            "openai" => Some(SwitchBackend::OpenAI),
            "deepseek" => Some(SwitchBackend::DeepSeek),
            "zhipu" => Some(SwitchBackend::Zhipu),
            "ollama" => Some(SwitchBackend::Ollama),
            _ => None,
        }
    }
}

/// 后端配置
#[derive(Debug, Clone)]
pub struct BackendConfig {
    /// 后端类型
    pub backend: SwitchBackend,
    /// API 密钥
    pub api_key: String,
    /// 基础 URL
    pub base_url: String,
    /// 默认模型
    pub model: String,
}

impl BackendConfig {
    pub fn minimax(api_key: String, model: &str) -> Self {
        Self {
            backend: SwitchBackend::MiniMax,
            api_key,
            base_url: "https://api.minimax.chat/v1".to_string(),
            model: model.to_string(),
        }
    }

    pub fn openai(api_key: String, base_url: &str, model: &str) -> Self {
        Self {
            backend: SwitchBackend::OpenAI,
            api_key,
            base_url: base_url.to_string(),
            model: model.to_string(),
        }
    }

    pub fn deepseek(api_key: String, model: &str) -> Self {
        Self {
            backend: SwitchBackend::DeepSeek,
            api_key,
            base_url: "https://api.deepseek.com/v1".to_string(),
            model: model.to_string(),
        }
    }

    pub fn zhipu(api_key: String, model: &str) -> Self {
        Self {
            backend: SwitchBackend::Zhipu,
            api_key,
            base_url: "https://open.bigmodel.cn/api/paas/v1".to_string(),
            model: model.to_string(),
        }
    }

    pub fn ollama(base_url: &str, model: &str) -> Self {
        Self {
            backend: SwitchBackend::Ollama,
            api_key: "".to_string(),
            base_url: base_url.to_string(),
            model: model.to_string(),
        }
    }
}

/// Claude Switch 提供商
///
/// 这是一个适配器，让 Claude 格式的请求可以路由到不同的后端。
#[derive(Debug, Clone)]
pub struct ClaudeSwitchProvider {
    /// HTTP 客户端
    client: Client,
    /// 当前后端配置
    backend: BackendConfig,
    /// 可用的后端列表
    available_backends: HashMap<SwitchBackend, BackendConfig>,
}

impl ClaudeSwitchProvider {
    /// 创建新的 Claude Switch 提供商
    pub fn new(backend: BackendConfig) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(300))
                .build()
                .unwrap_or_else(|_| Client::new()),
            backend: backend.clone(),
            available_backends: HashMap::from([(backend.backend, backend)]),
        }
    }

    /// 添加可用的后端
    pub fn with_backend(mut self, config: BackendConfig) -> Self {
        self.available_backends
            .insert(config.backend, config.clone());
        // 如果没有设置过后端，使用第一个
        if self.backend.api_key.is_empty() && !config.api_key.is_empty() {
            self.backend = config;
        }
        self
    }

    /// 切换后端
    pub fn set_backend(&mut self, backend: SwitchBackend) -> Result<(), AIError> {
        if let Some(config) = self.available_backends.get(&backend) {
            self.backend = config.clone();
            Ok(())
        } else {
            Err(AIError::InvalidRequest(format!(
                "后端 '{}' 未配置或不可用",
                backend.as_str()
            )))
        }
    }

    /// 获取当前后端
    pub fn current_backend(&self) -> &BackendConfig {
        &self.backend
    }

    /// 获取可用后端列表
    pub fn list_backends(&self) -> Vec<(SwitchBackend, BackendConfig)> {
        self.available_backends
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect()
    }

    /// 发送请求到后端
    async fn request(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, AIError> {
        let response = self
            .client
            .post(format!("{}{}", self.backend.base_url, path))
            .header("Authorization", format!("Bearer {}", self.backend.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AIError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => AIError::Authentication("认证失败".to_string()),
                429 => AIError::RateLimit("请求频率超限".to_string()),
                _ => AIError::Provider(format!("后端错误 {}: {}", status, error_text)),
            });
        }

        response
            .json()
            .await
            .map_err(|e| AIError::Parse(e.to_string()))
    }

    /// 测试后端连接
    pub async fn test_connection(&self) -> Result<(), AIError> {
        let test_request = serde_json::json!({
            "model": &self.backend.model,
            "messages": [{"role": "user", "content": "test"}],
            "max_tokens": 1
        });

        self.request("/chat/completions", test_request).await?;
        Ok(())
    }

    /// 将 Claude 格式转换为目标后端格式
    fn convert_request(&self, request: &ChatRequest) -> serde_json::Value {
        match self.backend.backend {
            SwitchBackend::MiniMax | SwitchBackend::OpenAI | SwitchBackend::DeepSeek => {
                // OpenAI 兼容格式
                let messages: Vec<serde_json::Value> = request
                    .messages
                    .iter()
                    .map(|m| {
                        serde_json::json!({
                            "role": m.role,
                            "content": m.content
                        })
                    })
                    .collect();

                serde_json::json!({
                    "model": &self.backend.model,
                    "messages": messages,
                    "temperature": request.temperature,
                    "max_tokens": request.max_tokens
                })
            }
            SwitchBackend::Zhipu => {
                // 智谱格式
                let messages: Vec<serde_json::Value> = request
                    .messages
                    .iter()
                    .map(|m| {
                        serde_json::json!({
                            "role": m.role,
                            "content": m.content
                        })
                    })
                    .collect();

                serde_json::json!({
                    "model": &self.backend.model,
                    "messages": messages,
                    "temperature": request.temperature,
                    "max_tokens": request.max_tokens
                })
            }
            SwitchBackend::Ollama => {
                // Ollama 格式
                let last_message = request
                    .messages
                    .last()
                    .map(|m| m.content.as_str())
                    .unwrap_or("");

                serde_json::json!({
                    "model": &self.backend.model,
                    "prompt": last_message,
                    "stream": false
                })
            }
        }
    }

    /// 将后端响应转换为 Claude 格式
    fn convert_response(&self, response: serde_json::Value) -> Result<ChatResponse, AIError> {
        match self.backend.backend {
            SwitchBackend::MiniMax
            | SwitchBackend::OpenAI
            | SwitchBackend::DeepSeek
            | SwitchBackend::Zhipu => {
                // OpenAI/智谱兼容格式
                #[derive(Deserialize)]
                struct Choice {
                    message: ResponseMessage,
                }

                #[derive(Deserialize)]
                struct ResponseMessage {
                    role: String,
                    content: String,
                }

                #[derive(Deserialize)]
                struct Usage {
                    prompt_tokens: usize,
                    completion_tokens: usize,
                    #[allow(dead_code)]
                    total_tokens: usize,
                }

                #[derive(Deserialize)]
                struct OpenAIResponse {
                    choices: Vec<Choice>,
                    usage: Usage,
                    model: String,
                }

                let resp: OpenAIResponse = serde_json::from_value(response)
                    .map_err(|e| AIError::Parse(format!("解析响应失败: {}", e)))?;

                let choice = resp
                    .choices
                    .into_iter()
                    .next()
                    .ok_or_else(|| AIError::Provider("后端返回空响应".to_string()))?;

                Ok(ChatResponse {
                    message: ChatMessage {
                        role: choice.message.role,
                        content: choice.message.content,
                    },
                    model: resp.model,
                    usage: TokenUsage {
                        input_tokens: resp.usage.prompt_tokens,
                        output_tokens: resp.usage.completion_tokens,
                    },
                    stop_reason: "stop".to_string(),
                })
            }
            SwitchBackend::Ollama => {
                // Ollama 格式
                #[derive(Deserialize)]
                struct OllamaResponse {
                    response: String,
                    model: String,
                }

                let resp: OllamaResponse = serde_json::from_value(response)
                    .map_err(|e| AIError::Parse(format!("解析 Ollama 响应失败: {}", e)))?;

                Ok(ChatResponse {
                    message: ChatMessage {
                        role: "assistant".to_string(),
                        content: resp.response,
                    },
                    model: resp.model,
                    usage: TokenUsage {
                        input_tokens: 0,
                        output_tokens: 0,
                    },
                    stop_reason: "stop".to_string(),
                })
            }
        }
    }
}

#[async_trait]
impl AIProvider for ClaudeSwitchProvider {
    fn provider_name(&self) -> &str {
        "claude-switch"
    }

    fn supported_models(&self) -> Vec<&str> {
        vec!["claude-sonnet-4-6", "claude-opus-4-6", "claude-haiku-4-5"]
    }

    fn supports_model(&self, model: &str) -> bool {
        self.supported_models().contains(&model)
    }

    fn default_model(&self) -> &str {
        &self.backend.model
    }

    fn supports_cli(&self, _cli: super::CLI) -> bool {
        false
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AIError> {
        let body = self.convert_request(&request);
        let response = self.request("/chat/completions", body).await?;
        self.convert_response(response)
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, AIError> {
        // 将补全请求转换为聊天格式
        let chat_request = ChatRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: request.prompt,
            }],
            model: request.model,
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

    async fn embed(&self, _request: EmbedRequest) -> Result<EmbedResponse, AIError> {
        Err(AIError::Provider(
            "Claude Switch 不支持嵌入 API".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_from_str() {
        assert_eq!(
            SwitchBackend::parse("minimax"),
            Some(SwitchBackend::MiniMax)
        );
        assert_eq!(SwitchBackend::parse("openai"), Some(SwitchBackend::OpenAI));
        assert_eq!(
            SwitchBackend::parse("deepseek"),
            Some(SwitchBackend::DeepSeek)
        );
        assert_eq!(SwitchBackend::parse("unknown"), None);
    }

    #[test]
    fn test_backend_config() {
        let config = BackendConfig::minimax("test-key".to_string(), "MiniMax-M2.7");
        assert_eq!(config.backend, SwitchBackend::MiniMax);
        assert_eq!(config.model, "MiniMax-M2.7");

        let config = BackendConfig::deepseek("test-key".to_string(), "deepseek-chat");
        assert_eq!(config.backend, SwitchBackend::DeepSeek);
        assert_eq!(config.base_url, "https://api.deepseek.com/v1");
    }
}
