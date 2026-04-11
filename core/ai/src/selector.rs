//! Global Model Selector
//!
//! 中央选择器,管理当前选定的 AI 模型。所有 AI 调用默认使用此选择器指定的模型。

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{AIError, AIProviderRegistry, ChatRequest, ChatResponse, CompletionRequest, CompletionResponse};

/// 24 小时刷新间隔
const MODEL_REFRESH_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// 全局模型选择器
///
/// 所有 AI 调用应该通过此选择器进行路由，以确保使用统一的模型配置。
pub struct GlobalModelSelector {
    /// 当前选定的模型 ID
    selected_model: RwLock<String>,
    /// 模型 ID 到完整模型配置的映射
    model_configs: RwLock<Vec<ModelInfo>>,
    /// 提供商注册表引用
    registry: Arc<AIProviderRegistry>,
    /// 上次刷新时间
    last_refresh: RwLock<Instant>,
    /// 刷新间隔
    refresh_interval: Duration,
}

/// 模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// 模型 ID (如 "claude-opus-4-5")
    pub model_id: String,
    /// 提供商名称 (如 "anthropic")
    pub provider: String,
    /// 模型显示名称
    pub display_name: String,
    /// 模型描述
    pub description: String,
    /// 是否支持聊天
    pub supports_chat: bool,
    /// 是否支持补全
    pub supports_completion: bool,
    /// 是否为默认模型
    pub is_default: bool,
}

impl GlobalModelSelector {
    /// 创建新的全局模型选择器
    pub fn new(registry: Arc<AIProviderRegistry>) -> Self {
        Self {
            selected_model: RwLock::new("claude-sonnet-4-5".to_string()),
            model_configs: RwLock::new(Vec::new()),
            registry,
            last_refresh: RwLock::new(Instant::now()),
            refresh_interval: MODEL_REFRESH_INTERVAL,
        }
    }

    /// 检查是否需要刷新模型列表
    pub fn needs_refresh(&self) -> bool {
        let last = *self.last_refresh.read();
        last.elapsed() >= self.refresh_interval
    }

    /// 标记模型列表已刷新
    pub fn mark_refreshed(&self) {
        let mut last = self.last_refresh.write();
        *last = Instant::now();
    }

    /// 获取距离下次刷新的剩余时间（秒）
    pub fn time_until_refresh(&self) -> u64 {
        let last = *self.last_refresh.read();
        let elapsed = last.elapsed();
        if elapsed >= self.refresh_interval {
            0
        } else {
            (self.refresh_interval - elapsed).as_secs()
        }
    }

    /// 获取上次刷新的时间戳
    pub fn last_refresh_time(&self) -> Instant {
        *self.last_refresh.read()
    }

    /// 清除所有已注册的模型
    pub fn clear_models(&self) {
        self.model_configs.write().clear();
    }

    /// 刷新模型列表（重新注册）
    /// 返回刷新前后模型数量变化
    pub fn refresh_models<F>(&self, register_fn: F) -> (usize, usize)
    where
        F: FnOnce(&Self),
    {
        let before = self.model_configs.read().len();
        self.clear_models();
        register_fn(self);
        self.mark_refreshed();
        let after = self.model_configs.read().len();
        (before, after)
    }

    /// 获取当前选定的模型 ID
    pub fn get_selected_model(&self) -> String {
        self.selected_model.read().clone()
    }

    /// 设置当前选定的模型
    pub fn set_selected_model(&self, model_id: &str) -> Result<(), AIError> {
        // 验证模型是否存在
        let configs = self.model_configs.read();
        if !configs.iter().any(|c| c.model_id == model_id) {
            return Err(AIError::InvalidRequest(format!(
                "模型 '{}' 不存在或未注册",
                model_id
            )));
        }
        drop(configs);

        let mut selected = self.selected_model.write();
        *selected = model_id.to_string();
        Ok(())
    }

    /// 注册可用模型
    pub fn register_model(&self, model_info: ModelInfo) {
        let mut configs = self.model_configs.write();
        // 如果已存在,更新而不是重复添加
        if let Some(existing) = configs.iter_mut().find(|c| c.model_id == model_info.model_id) {
            *existing = model_info;
        } else {
            configs.push(model_info);
        }
    }

    /// 注册多个模型
    pub fn register_models(&self, models: Vec<ModelInfo>) {
        for model in models {
            self.register_model(model);
        }
    }

    /// 列出所有可用模型
    pub fn list_models(&self) -> Vec<ModelInfo> {
        self.model_configs.read().clone()
    }

    /// 获取当前选定模型的完整信息
    pub fn get_selected_model_info(&self) -> Option<ModelInfo> {
        let selected = self.get_selected_model();
        self.model_configs.read()
            .iter()
            .find(|c| c.model_id == selected)
            .cloned()
    }

    /// 执行聊天请求,使用当前选定的模型
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AIError> {
        let model_id = self.get_selected_model();
        let request = ChatRequest {
            model: model_id,
            ..request
        };
        self.registry.chat(request).await
    }

    /// 执行补全请求,使用当前选定的模型
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, AIError> {
        let model_id = self.get_selected_model();
        let request = CompletionRequest {
            model: model_id,
            ..request
        };
        self.registry.complete(request).await
    }

    /// 使用指定的模型执行聊天请求(覆盖全局选择)
    pub async fn chat_with_model(&self, model_id: &str, request: ChatRequest) -> Result<ChatResponse, AIError> {
        let request = ChatRequest {
            model: model_id.to_string(),
            ..request
        };
        self.registry.chat(request).await
    }

    /// 使用指定的模型执行补全请求(覆盖全局选择)
    pub async fn complete_with_model(&self, model_id: &str, request: CompletionRequest) -> Result<CompletionResponse, AIError> {
        let request = CompletionRequest {
            model: model_id.to_string(),
            ..request
        };
        self.registry.complete(request).await
    }

    /// 检查指定模型是否可用
    pub fn is_model_available(&self, model_id: &str) -> bool {
        self.model_configs.read()
            .iter()
            .any(|c| c.model_id == model_id)
    }

    /// 获取提供商注册表
    pub fn registry(&self) -> Arc<AIProviderRegistry> {
        self.registry.clone()
    }
}

impl Default for GlobalModelSelector {
    fn default() -> Self {
        Self::new(Arc::new(AIProviderRegistry::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selector_default() {
        let selector = GlobalModelSelector::default();
        assert_eq!(selector.get_selected_model(), "claude-sonnet-4-5");
    }

    #[test]
    fn test_register_model() {
        let selector = GlobalModelSelector::default();
        selector.register_model(ModelInfo {
            model_id: "gpt-4".to_string(),
            provider: "openai".to_string(),
            display_name: "GPT-4".to_string(),
            description: "OpenAI's most capable model".to_string(),
            supports_chat: true,
            supports_completion: true,
            is_default: false,
        });

        let models = selector.list_models();
        assert!(models.iter().any(|m| m.model_id == "gpt-4"));
    }

    #[test]
    fn test_set_selected_model() {
        let selector = GlobalModelSelector::default();
        selector.register_model(ModelInfo {
            model_id: "gpt-4".to_string(),
            provider: "openai".to_string(),
            display_name: "GPT-4".to_string(),
            description: "".to_string(),
            supports_chat: true,
            supports_completion: true,
            is_default: false,
        });

        selector.set_selected_model("gpt-4").unwrap();
        assert_eq!(selector.get_selected_model(), "gpt-4");
    }

    #[test]
    fn test_set_invalid_model() {
        let selector = GlobalModelSelector::default();
        let result = selector.set_selected_model("non-existent-model");
        assert!(result.is_err());
    }
}