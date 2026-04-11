//! 插件服务
//!
//! 管理插件的加载、卸载和查询。

use std::sync::Arc;
use nx_plugin::{PluginRegistry, Plugin, PluginMetadata};
use nx_plugin::plugin_trait::PluginError;

/// 插件服务
pub struct PluginService {
    registry: Arc<PluginRegistry>,
}

impl PluginService {
    /// 创建新的插件服务
    pub fn new() -> Self {
        Self {
            registry: Arc::new(PluginRegistry::new()),
        }
    }

    /// 获取插件注册表
    pub fn registry(&self) -> Arc<PluginRegistry> {
        self.registry.clone()
    }

    /// 列出所有已加载的插件
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.registry
            .plugins()
            .into_iter()
            .map(|p| PluginInfo {
                id: p.metadata().id.clone(),
                name: p.metadata().name.clone(),
                version: p.metadata().version.clone(),
                description: p.metadata().description.clone(),
                author: p.metadata().author.clone(),
            })
            .collect()
    }

    /// 获取插件信息
    pub fn get_plugin(&self, id: &str) -> Option<PluginInfo> {
        self.registry.get(id).map(|p| PluginInfo {
            id: p.metadata().id.clone(),
            name: p.metadata().name.clone(),
            version: p.metadata().version.clone(),
            description: p.metadata().description.clone(),
            author: p.metadata().author.clone(),
        })
    }

    /// 加载插件
    pub async fn load_plugin(&self, plugin: Arc<dyn Plugin>) -> Result<(), PluginServiceError> {
        self.registry
            .load_plugin(plugin)
            .await
            .map_err(PluginServiceError::from)
    }

    /// 卸载插件
    pub async fn unload_plugin(&self, id: &str) -> Result<(), PluginServiceError> {
        self.registry
            .unload(id)
            .await
            .map_err(PluginServiceError::from)
    }

    /// 检查插件是否已加载
    pub fn is_loaded(&self, id: &str) -> bool {
        self.registry.is_loaded(id)
    }

    /// 获取已加载插件数量
    pub fn count(&self) -> usize {
        self.registry.len()
    }
}

impl Default for PluginService {
    fn default() -> Self {
        Self::new()
    }
}

/// 插件信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
}

impl From<PluginMetadata> for PluginInfo {
    fn from(m: PluginMetadata) -> Self {
        Self {
            id: m.id,
            name: m.name,
            version: m.version,
            description: m.description,
            author: m.author,
        }
    }
}

/// 插件服务错误
#[derive(Debug, thiserror::Error)]
pub enum PluginServiceError {
    #[error("插件已加载: {0}")]
    AlreadyLoaded(String),

    #[error("插件不存在: {0}")]
    NotFound(String),

    #[error("加载失败: {0}")]
    LoadFailed(String),

    #[error("卸载失败: {0}")]
    UnloadFailed(String),
}

impl From<PluginError> for PluginServiceError {
    fn from(err: PluginError) -> Self {
        match err {
            PluginError::AlreadyLoaded(id) => PluginServiceError::AlreadyLoaded(id),
            PluginError::NotFound(id) => PluginServiceError::NotFound(id),
            _ => PluginServiceError::LoadFailed(err.to_string()),
        }
    }
}