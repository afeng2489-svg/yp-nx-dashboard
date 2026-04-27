use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::any::Any;

/// Metadata about a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
}

impl PluginMetadata {
    pub fn new(id: impl Into<String>, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            description: None,
            author: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }
}

/// Context provided to plugins during initialization
pub struct PluginContext {
    pub skill_registry: std::sync::Arc<nexus_skills::SkillRegistry>,
    pub ai_registry: std::sync::Arc<nexus_ai::AIProviderRegistry>,
    pub config: std::sync::Arc<dyn ConfigProvider>,
}

/// Configuration provider trait for plugins
pub trait ConfigProvider: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    fn get_all(&self) -> std::collections::HashMap<String, String>;
}

/// Plugin initialization error
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Initialization failed: {0}")]
    InitFailed(String),

    #[error("Shutdown failed: {0}")]
    ShutdownFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Dependency error: {0}")]
    DependencyError(String),

    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin already loaded: {0}")]
    AlreadyLoaded(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Core plugin trait that all plugins must implement
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Returns the plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize the plugin with the given context
    async fn initialize(&self, context: &PluginContext) -> Result<(), PluginError>;

    /// Shutdown the plugin gracefully
    async fn shutdown(&self) -> Result<(), PluginError>;

    /// Returns self as a dyn Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Extension trait for plugin ID convenience
pub trait PluginExt: Plugin {
    fn id(&self) -> &str {
        &self.metadata().id
    }

    fn name(&self) -> &str {
        &self.metadata().name
    }

    fn version(&self) -> &str {
        &self.metadata().version
    }
}

impl<T: Plugin> PluginExt for T {}

/// Unit type for plugins with no dependencies
pub struct NoDependencies;

impl ConfigProvider for std::collections::HashMap<String, String> {
    fn get(&self, key: &str) -> Option<String> {
        self.get(key).cloned()
    }

    fn get_all(&self) -> std::collections::HashMap<String, String> {
        self.clone()
    }
}

/// Simple in-memory config provider
pub struct InMemoryConfig {
    values: parking_lot::RwLock<std::collections::HashMap<String, String>>,
}

impl InMemoryConfig {
    pub fn new() -> Self {
        Self {
            values: parking_lot::RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        self.values.write().insert(key.into(), value.into());
    }

    pub fn remove(&self, key: &str) {
        self.values.write().remove(key);
    }
}

impl Default for InMemoryConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigProvider for InMemoryConfig {
    fn get(&self, key: &str) -> Option<String> {
        self.values.read().get(key).cloned()
    }

    fn get_all(&self) -> std::collections::HashMap<String, String> {
        self.values.read().clone()
    }
}
