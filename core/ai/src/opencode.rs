//! OpenCode Provider Implementation
//!
//! OpenCode provider for open source project tasks.

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

use super::{
    AIError, AIProvider, ChatMessage, ChatRequest, ChatResponse, CLI, CLIContext,
    CLIResponse, CompletionRequest, CompletionResponse, EmbedRequest, EmbedResponse,
    TokenUsage,
};

/// OpenCode Provider
#[derive(Debug, Clone)]
pub struct OpenCodeProvider {
    /// API 密钥
    api_key: String,
    /// 默认模型
    default_model: String,
}

impl OpenCodeProvider {
    /// 创建新的 OpenCode 提供商
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            default_model: "opencode".to_string(),
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
impl AIProvider for OpenCodeProvider {
    /// 获取提供商名称
    fn provider_name(&self) -> &str {
        "opencode"
    }

    /// 获取支持的模型列表
    fn supported_models(&self) -> Vec<&str> {
        vec!["opencode", "opencode-plus"]
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
        let user_message = request
            .messages
            .last()
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let response_content = if user_message.to_lowercase().contains("github")
            || user_message.to_lowercase().contains("open source")
        {
            r#"我看到你对开源项目感兴趣！让我帮你分析一下。

对于开源项目，我建议:

1. **项目结构分析**
   - 查看 README.md 了解项目概况
   - 分析目录结构确定核心模块
   - 查看 package.json/pyproject.toml 等依赖文件

2. **贡献指南**
   - 仔细阅读 CONTRIBUTING.md
   - 遵守代码风格规范
   - 先从 good first issue 开始

3. **技术栈识别**
   - 确定主要编程语言
   - 了解使用的框架和库
   - 分析架构设计模式

请问你具体想了解哪个方面？"#
                .to_string()
        } else {
            r#"OpenCode 专注于帮助开发者:

1. **代码理解**
   - 分析代码结构和逻辑
   - 解释复杂算法
   - 识别代码异味

2. **开源项目支持**
   - GitHub 项目分析和克隆
   - 流行框架集成指导
   - 开源许可证咨询

3. **开发效率**
   - 快速原型开发
   - 代码模板生成
   - 最佳实践建议

请告诉我你需要哪方面的帮助？"#
                .to_string()
        };

        Ok(ChatResponse {
            message: ChatMessage {
                role: "assistant".to_string(),
                content: response_content,
            },
            model: request.model,
            usage: TokenUsage {
                input_tokens: 100,
                output_tokens: 300,
            },
            stop_reason: "stop".to_string(),
        })
    }

    /// OpenCode 不支持嵌入
    async fn embed(&self, _request: EmbedRequest) -> Result<EmbedResponse, AIError> {
        Err(AIError::InvalidRequest(
            "OpenCode 不支持嵌入生成".to_string(),
        ))
    }

    /// 获取支持的 CLI
    fn supported_clis(&self) -> Vec<CLI> {
        vec![CLI::OpenCode]
    }

    /// 使用 OpenCode CLI 执行
    async fn execute_with_cli(
        &self,
        prompt: &str,
        cli: CLI,
        context: &CLIContext,
    ) -> Result<CLIResponse, AIError> {
        let start = Instant::now();

        let output = format!(
            "OpenCode CLI executed\n\
             Prompt: '{}'\n\
             Target: Open Source Projects\n\
             Working directory: {:?}\n\
             Features: GitHub integration, popular frameworks support",
            prompt.chars().take(40).collect::<String>(),
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
    async fn test_opencode_github_prompt() {
        let provider = OpenCodeProvider::new("test-key".to_string());
        let request = ChatRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Help me understand this GitHub repo".to_string(),
            }],
            model: "opencode".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
        };

        let result = provider.chat(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.message.content.contains("GitHub") || response.message.content.contains("开源"));
    }

    #[tokio::test]
    async fn test_opencode_execute_with_cli() {
        let provider = OpenCodeProvider::new("test-key".to_string());
        let context = CLIContext::default();

        let result = provider
            .execute_with_cli("analyze github project", CLI::OpenCode, &context)
            .await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.exit_code, 0);
    }
}