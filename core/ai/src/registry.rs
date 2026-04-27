//! AI Provider Registry
//!
//! 管理多个 AI 提供商的中央注册表。
//! 根据模型名称将请求路由到适当的提供商。

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use super::{
    AIError, AIProvider, ChatRequest, ChatResponse, CompletionRequest, CompletionResponse,
    EmbedRequest, EmbedResponse,
};

/// 管理 AI 提供商的注册表
pub struct AIProviderRegistry {
    /// 提供商列表
    providers: RwLock<HashMap<String, Arc<dyn AIProvider>>>,
    /// 模型到提供商的映射
    model_to_provider: RwLock<HashMap<String, String>>,
    /// 默认提供商
    default_provider: RwLock<Option<String>>,
}

impl AIProviderRegistry {
    /// 创建新的空注册表
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            model_to_provider: RwLock::new(HashMap::new()),
            default_provider: RwLock::new(None),
        }
    }

    /// 注册一个提供商
    pub fn register(&self, provider: Arc<dyn AIProvider>) {
        let name = provider.provider_name().to_string();
        // 转换模型引用为拥有的字符串，避免后续借用问题
        let models: Vec<String> = provider
            .supported_models()
            .iter()
            .map(|s| s.to_string())
            .collect();

        // 插入提供商
        {
            let mut providers = self.providers.write();
            providers.insert(name.clone(), provider);
        }

        // 插入模型映射
        {
            let mut model_map = self.model_to_provider.write();
            for model in models {
                model_map.insert(model, name.clone());
            }
        }
    }

    /// 设置默认提供商
    pub fn set_default(&self, name: &str) -> Result<(), AIError> {
        let providers = self.providers.read();
        if !providers.contains_key(name) {
            return Err(AIError::InvalidRequest(format!("提供商 '{}' 未注册", name)));
        }

        let mut default = self.default_provider.write();
        *default = Some(name.to_string());
        Ok(())
    }

    /// 通过名称获取提供商
    pub fn get(&self, name: &str) -> Option<Arc<dyn AIProvider>> {
        let providers = self.providers.read();
        providers.get(name).cloned()
    }

    /// 获取默认提供商
    pub fn default(&self) -> Option<Arc<dyn AIProvider>> {
        let default = self.default_provider.read().clone();
        default.and_then(|name| self.get(&name))
    }

    /// 根据模型名称将请求路由到适当的提供商
    pub fn route(&self, model: &str) -> Result<Arc<dyn AIProvider>, AIError> {
        let provider_name = {
            let model_map = self.model_to_provider.read();
            model_map.get(model).cloned()
        };

        if let Some(name) = provider_name {
            self.get(&name)
                .ok_or_else(|| AIError::Provider(format!("提供商 '{}' 未找到", name)))
        } else {
            // 回退到默认提供商
            self.default()
                .ok_or_else(|| AIError::ModelNotSupported(format!("模型 '{}' 不支持", model)))
        }
    }

    /// 执行补全请求，按模型路由
    pub async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, AIError> {
        let provider = self.route(&request.model)?;
        provider.complete(request).await
    }

    /// 执行聊天请求，按模型路由
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AIError> {
        let provider = self.route(&request.model)?;
        provider.chat(request).await
    }

    /// 执行嵌入请求，按模型路由
    pub async fn embed(&self, request: EmbedRequest) -> Result<EmbedResponse, AIError> {
        let provider = self.route(&request.model)?;
        provider.embed(request).await
    }

    /// 列出所有注册的提供商名称
    pub fn list_providers(&self) -> Vec<String> {
        let providers = self.providers.read();
        providers.keys().cloned().collect()
    }

    /// 注册单个模型到提供商的映射（用于动态添加 Claude Switch 后端模型）
    pub fn register_model_mapping(&self, model_id: &str, provider_name: &str) {
        let mut model_map = self.model_to_provider.write();
        model_map.insert(model_id.to_string(), provider_name.to_string());
    }

    /// 列出所有支持的模型
    pub fn list_models(&self) -> Vec<String> {
        let model_map = self.model_to_provider.read();
        model_map.keys().cloned().collect()
    }

    /// 获取所有提供商
    pub fn get_all_providers(&self) -> Vec<Arc<dyn AIProvider>> {
        let providers = self.providers.read();
        providers.values().cloned().collect()
    }

    /// 检查是否有提供商支持指定 CLI
    pub fn supports_cli(&self, cli: super::CLI) -> bool {
        let providers = self.providers.read();
        providers.values().any(|p| p.supports_cli(cli))
    }

    /// 获取支持指定 CLI 的提供商
    pub fn get_provider_for_cli(&self, cli: super::CLI) -> Option<Arc<dyn AIProvider>> {
        let providers = self.providers.read();
        providers.values().find(|p| p.supports_cli(cli)).cloned()
    }
}

impl Default for AIProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 用于创建已配置注册表的构建器
pub struct RegistryBuilder {
    registry: AIProviderRegistry,
}

impl RegistryBuilder {
    pub fn new() -> Self {
        Self {
            registry: AIProviderRegistry::new(),
        }
    }

    pub fn with_provider(self, provider: Arc<dyn AIProvider>) -> Self {
        self.registry.register(provider);
        self
    }

    pub fn with_default(self, name: &str) -> Result<Self, AIError> {
        self.registry.set_default(name)?;
        Ok(self)
    }

    pub fn build(self) -> AIProviderRegistry {
        self.registry
    }
}

impl Default for RegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TokenUsage;

    struct MockProvider {
        name: String,
        models: Vec<String>,
    }

    impl MockProvider {
        fn new(name: &str, models: Vec<&str>) -> Self {
            Self {
                name: name.to_string(),
                models: models.iter().map(|s| s.to_string()).collect(),
            }
        }
    }

    #[async_trait::async_trait]
    impl AIProvider for MockProvider {
        fn provider_name(&self) -> &str {
            &self.name
        }

        fn supported_models(&self) -> Vec<&str> {
            self.models.iter().map(|s| s.as_str()).collect()
        }

        async fn complete(
            &self,
            _request: CompletionRequest,
        ) -> Result<CompletionResponse, AIError> {
            Ok(CompletionResponse {
                text: "mock response".to_string(),
                model: self.models[0].clone(),
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 20,
                },
                stop_reason: "stop".to_string(),
            })
        }

        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, AIError> {
            todo!()
        }

        async fn embed(&self, _request: EmbedRequest) -> Result<EmbedResponse, AIError> {
            todo!()
        }

        fn default_model(&self) -> &str {
            &self.models[0]
        }
    }

    #[tokio::test]
    async fn test_registry_routing() {
        let registry = AIProviderRegistry::new();

        let provider1 = Arc::new(MockProvider::new("provider1", vec!["model-a", "model-b"]));
        let provider2 = Arc::new(MockProvider::new("provider2", vec!["model-c"]));

        registry.register(provider1);
        registry.register(provider2);

        let result = registry.route("model-a").unwrap();
        assert_eq!(result.provider_name(), "provider1");

        let result = registry.route("model-c").unwrap();
        assert_eq!(result.provider_name(), "provider2");
    }
}
