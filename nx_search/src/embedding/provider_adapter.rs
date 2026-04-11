//! AI Provider 到 EmbeddingProvider 的适配器
//!
//! 桥接 `nexus_ai::AIProviderRegistry` 和 `EmbeddingProvider` trait

use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;

use nexus_ai::{AIProviderRegistry, AIError, EmbedRequest};

use super::{EmbeddingProvider, EmbeddingResult, EmbedError};

/// 默认 embedding 模型
const DEFAULT_EMBEDDING_MODEL: &str = "text-embedding-3-small";

/// 将 AIProviderRegistry 适配为 EmbeddingProvider
pub struct AIEmbeddingAdapter {
    registry: Arc<AIProviderRegistry>,
    default_model: String,
}

impl AIEmbeddingAdapter {
    /// 创建新的适配器
    pub fn new(registry: Arc<AIProviderRegistry>) -> Self {
        Self {
            registry,
            default_model: DEFAULT_EMBEDDING_MODEL.to_string(),
        }
    }

    /// 创建新的适配器，自定义默认模型
    pub fn with_model(registry: Arc<AIProviderRegistry>, model: impl Into<String>) -> Self {
        Self {
            registry,
            default_model: model.into(),
        }
    }

    /// 将 AIError 转换为 EmbedError
    fn map_error(err: AIError) -> EmbedError {
        match err {
            AIError::Authentication(msg) => EmbedError::Api(msg),
            AIError::Network(msg) => EmbedError::Network(msg),
            AIError::RateLimit(msg) => EmbedError::Api(format!("Rate limit: {}", msg)),
            AIError::ModelNotSupported(msg) => EmbedError::Api(format!("Model not supported: {}", msg)),
            AIError::InvalidRequest(msg) => EmbedError::Api(format!("Invalid request: {}", msg)),
            AIError::Provider(msg) => EmbedError::Api(format!("Provider error: {}", msg)),
            AIError::Timeout(msg) => EmbedError::Network(format!("Timeout: {}", msg)),
            AIError::Parse(msg) => EmbedError::Parse(msg),
        }
    }
}

impl EmbeddingProvider for AIEmbeddingAdapter {
    fn name(&self) -> &str {
        "ai_registry_adapter"
    }

    fn embed(&self, text: &str) -> Pin<Box<dyn Future<Output = Result<EmbeddingResult, EmbedError>> + Send + '_>> {
        let text = text.to_string();
        let registry = self.registry.clone();
        let model = self.default_model.clone();

        Box::pin(async move {
            let request = EmbedRequest {
                texts: vec![text.clone()],
                model: model.clone(),
            };

            let response = registry
                .embed(request)
                .await
                .map_err(Self::map_error)?;

            let vector = response
                .embeddings
                .into_iter()
                .next()
                .ok_or_else(|| EmbedError::Parse("No embedding returned".to_string()))?;

            Ok(EmbeddingResult {
                vector,
                model: response.model,
                token_count: response.usage.input_tokens,
            })
        })
    }

    fn embed_batch(
        &self,
        texts: &[String],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<EmbeddingResult>, EmbedError>> + Send + '_>> {
        let texts = texts.to_vec();
        let registry = self.registry.clone();
        let model = self.default_model.clone();

        Box::pin(async move {
            let request = EmbedRequest {
                texts: texts.clone(),
                model: model.clone(),
            };

            let response = registry
                .embed(request)
                .await
                .map_err(Self::map_error)?;

            let results = response
                .embeddings
                .into_iter()
                .map(|vector| EmbeddingResult {
                    vector,
                    model: response.model.clone(),
                    token_count: response.usage.input_tokens,
                })
                .collect();

            Ok(results)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use nexus_ai::{AIProvider, CompletionRequest, CompletionResponse, ChatRequest, ChatResponse, EmbedResponse, TokenUsage, AIError, EmbedRequest};

    struct MockAIProvider {
        name: &'static str,
        embed_vector: Vec<f32>,
    }

    impl MockAIProvider {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                embed_vector: vec![0.1; 1536],
            }
        }
    }

    #[async_trait]
    impl AIProvider for MockAIProvider {
        fn provider_name(&self) -> &str {
            self.name
        }

        fn supported_models(&self) -> Vec<&str> {
            vec!["text-embedding-3-small", "embedding-model"]
        }

        async fn complete(&self, _: CompletionRequest) -> Result<CompletionResponse, AIError> {
            todo!()
        }

        async fn chat(&self, _: ChatRequest) -> Result<ChatResponse, AIError> {
            todo!()
        }

        async fn embed(&self, request: EmbedRequest) -> Result<EmbedResponse, AIError> {
            Ok(EmbedResponse {
                embeddings: request
                    .texts
                    .iter()
                    .map(|_| {
                        let mut v = self.embed_vector.clone();
                        // 轻微变化，使每个文本的 embedding 不同
                        v[0] += 0.01;
                        v
                    })
                    .collect(),
                model: "text-embedding-3-small".to_string(),
                usage: TokenUsage {
                    input_tokens: request.texts.iter().map(|t| t.len() / 4).sum(),
                    output_tokens: 0,
                },
            })
        }

        fn default_model(&self) -> &str {
            "text-embedding-3-small"
        }
    }

    #[tokio::test]
    async fn test_adapter_embed_single() {
        let registry = Arc::new(AIProviderRegistry::new());
        registry.register(Arc::new(MockAIProvider::new("openai")));

        let adapter = AIEmbeddingAdapter::new(registry);
        let result = adapter.embed("Hello world").await.unwrap();

        assert_eq!(result.vector.len(), 1536);
        assert_eq!(result.model, "text-embedding-3-small");
    }

    #[tokio::test]
    async fn test_adapter_embed_batch() {
        let registry = Arc::new(AIProviderRegistry::new());
        registry.register(Arc::new(MockAIProvider::new("openai")));

        let adapter = AIEmbeddingAdapter::new(registry);
        let texts = vec!["Hello".to_string(), "World".to_string()];
        let results = adapter.embed_batch(&texts).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].vector.len(), 1536);
        assert_eq!(results[1].vector.len(), 1536);
    }
}
