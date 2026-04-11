//! Codex Provider Implementation
//!
//! OpenAI Codex provider for code generation tasks.

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

use super::{
    AIError, AIProvider, ChatMessage, ChatRequest, ChatResponse, CLI, CLIContext,
    CLIResponse, CompletionRequest, CompletionResponse, EmbedRequest, EmbedResponse,
    TokenUsage,
};

/// Codex Provider
#[derive(Debug, Clone)]
pub struct CodexProvider {
    /// API 密钥
    api_key: String,
    /// 默认模型
    default_model: String,
}

impl CodexProvider {
    /// 创建新的 Codex 提供商
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            default_model: "codex".to_string(),
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
impl AIProvider for CodexProvider {
    /// 获取提供商名称
    fn provider_name(&self) -> &str {
        "codex"
    }

    /// 获取支持的模型列表
    fn supported_models(&self) -> Vec<&str> {
        vec!["codex", "codex-plus", "gpt-4-codex"]
    }

    /// 获取默认模型
    fn default_model(&self) -> &str {
        &self.default_model
    }

    /// Codex 主要用于代码生成，使用聊天接口
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
        // 模拟 Codex 响应
        let code_example = r#"```python
def quicksort(arr):
    if len(arr) <= 1:
        return arr
    pivot = arr[len(arr) // 2]
    left = [x for x in arr if x < pivot]
    middle = [x for x in arr if x == pivot]
    right = [x for x in arr if x > pivot]
    return quicksort(left) + middle + quicksort(right)
```"#;

        Ok(ChatResponse {
            message: ChatMessage {
                role: "assistant".to_string(),
                content: format!(
                    "这是用 Python 实现的快速排序算法:\n\n{}\n\n\
                     时间复杂度: O(n log n) 平均, O(n²) 最坏\n\
                     空间复杂度: O(n)\n\
                     这是一个高效的原地排序算法，采用分治策略。",
                    code_example
                ),
            },
            model: request.model,
            usage: TokenUsage {
                input_tokens: 50,
                output_tokens: 150,
            },
            stop_reason: "stop".to_string(),
        })
    }

    /// Codex 不支持嵌入
    async fn embed(&self, _request: EmbedRequest) -> Result<EmbedResponse, AIError> {
        Err(AIError::InvalidRequest(
            "Codex 不支持嵌入生成".to_string(),
        ))
    }

    /// 获取支持的 CLI
    fn supported_clis(&self) -> Vec<CLI> {
        vec![CLI::OpenCode, CLI::Codex]
    }

    /// 使用 Codex CLI 执行
    async fn execute_with_cli(
        &self,
        prompt: &str,
        cli: CLI,
        context: &CLIContext,
    ) -> Result<CLIResponse, AIError> {
        let start = Instant::now();

        // 模拟 CLI 执行
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

    #[tokio::test]
    async fn test_codex_chat() {
        let provider = CodexProvider::new("test-key".to_string());
        let request = ChatRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Write a sorting function".to_string(),
            }],
            model: "codex".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
        };

        let result = provider.chat(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.message.content.contains("sorting"));
    }

    #[tokio::test]
    async fn test_codex_execute_with_cli() {
        let provider = CodexProvider::new("test-key".to_string());
        let context = CLIContext::default();

        let result = provider
            .execute_with_cli("sort this array", CLI::OpenCode, &context)
            .await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.exit_code, 0);
    }
}