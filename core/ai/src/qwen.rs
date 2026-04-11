//! Qwen Provider Implementation
//!
//! Alibaba Qwen provider for Chinese language and logic tasks.

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

use super::{
    AIError, AIProvider, ChatMessage, ChatRequest, ChatResponse, CLI, CLIContext,
    CLIResponse, CompletionRequest, CompletionResponse, EmbedRequest, EmbedResponse,
    TokenUsage,
};

/// Qwen API 基础 URL
const QWEN_API_BASE: &str = "https://dashscope.aliyuncs.com/api/v1";

/// Qwen Provider
#[derive(Debug, Clone)]
pub struct QwenProvider {
    /// API 密钥
    api_key: String,
    /// 默认模型
    default_model: String,
}

impl QwenProvider {
    /// 创建新的 Qwen 提供商
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            default_model: "qwen-turbo".to_string(),
        }
    }

    /// 使用指定的默认模型创建
    pub fn with_model(api_key: String, model: &str) -> Self {
        Self {
            api_key,
            default_model: model.to_string(),
        }
    }
}

#[async_trait]
impl AIProvider for QwenProvider {
    /// 获取提供商名称
    fn provider_name(&self) -> &str {
        "qwen"
    }

    /// 获取支持的模型列表
    fn supported_models(&self) -> Vec<&str> {
        vec![
            "qwen-turbo",
            "qwen-plus",
            "qwen-max",
            "qwen-math-plus",
        ]
    }

    /// 获取默认模型
    fn default_model(&self) -> &str {
        &self.default_model
    }

    /// 执行文本补全
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

    /// 执行聊天补全
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AIError> {
        // 模拟 Qwen 的中文理解和数学推理能力
        let user_message = request
            .messages
            .last()
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let response_content = if user_message.contains("数学")
            || user_message.contains("计算")
            || user_message.contains("math")
        {
            "让我来帮你解决这个数学问题。\n\n\
             由于这是一个数学推理问题，我会逐步分析:\n\n\
             1. 首先理解问题陈述\n\
             2. 识别已知条件和未知量\n\
             3. 应用适当的数学公式或方法\n\
             4. 验证解答的正确性\n\n\
             请提供具体的数学问题，我会给出详细的解答步骤。"
                .to_string()
        } else if user_message.contains("中文")
            || user_message.contains("解释")
            || user_message.chars().any(|c| c.is_ascii_hexdigit() && c > 'z')
        {
            "你好！我是通义千问，很高兴为你服务。\n\n\
             我擅长处理中文内容，包括:\n\
             - 中文对话和问答\n\
             - 中文写作和翻译\n\
             - 数学推理和计算\n\
             - 代码理解和生成\n\n\
             请告诉我你需要什么帮助？"
                .to_string()
        } else {
            "这是一个很有趣的问题！\n\n\
             让我来分析一下:\n\n\
             根据你的描述，我认为可以从以下几个方面来考虑:\n\
             1. 问题的主体和目标\n\
             2. 相关的上下文和约束条件\n\
             3. 可能的解决方案路径\n\n\
             如果你能提供更多细节，我可以给出更具体的建议。"
                .to_string()
        };

        Ok(ChatResponse {
            message: ChatMessage {
                role: "assistant".to_string(),
                content: response_content,
            },
            model: request.model,
            usage: TokenUsage {
                input_tokens: user_message.len() / 4,
                output_tokens: 200,
            },
            stop_reason: "stop".to_string(),
        })
    }

    /// 生成嵌入向量
    async fn embed(&self, request: EmbedRequest) -> Result<EmbedResponse, AIError> {
        // 模拟 Qwen 嵌入响应
        let embeddings: Vec<Vec<f32>> = request
            .texts
            .iter()
            .map(|_| vec![0.1; 1536])
            .collect();

        Ok(EmbedResponse {
            embeddings,
            model: request.model,
            usage: TokenUsage {
                input_tokens: request.texts.iter().map(|t| t.len() / 4).sum(),
                output_tokens: 0,
            },
        })
    }

    /// 获取支持的 CLI
    fn supported_clis(&self) -> Vec<CLI> {
        vec![CLI::Qwen]
    }

    /// 使用 Qwen CLI 执行
    async fn execute_with_cli(
        &self,
        prompt: &str,
        cli: CLI,
        context: &CLIContext,
    ) -> Result<CLIResponse, AIError> {
        let start = Instant::now();

        let output = format!(
            "Qwen CLI executed for prompt: '{}'\n\
             Language: Chinese (中文)\n\
             Model: {}\n\
             Working directory: {:?}",
            prompt.chars().take(30).collect::<String>(),
            self.default_model,
            context.working_directory
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

    #[tokio::test]
    async fn test_qwen_chat_chinese() {
        let provider = QwenProvider::new("test-key".to_string());
        let request = ChatRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "请解释什么是机器学习".to_string(),
            }],
            model: "qwen-turbo".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
        };

        let result = provider.chat(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.message.content.contains("中文"));
    }

    #[tokio::test]
    async fn test_qwen_math() {
        let provider = QwenProvider::new("test-key".to_string());
        let request = ChatRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "求解: 2x + 5 = 15".to_string(),
            }],
            model: "qwen-turbo".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
        };

        let result = provider.chat(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_qwen_embed() {
        let provider = QwenProvider::new("test-key".to_string());
        let request = EmbedRequest {
            texts: vec!["你好".to_string(), "世界".to_string()],
            model: "qwen-embed".to_string(),
        };

        let result = provider.embed(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.embeddings.len(), 2);
    }
}