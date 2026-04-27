use crate::plugin_trait::{Plugin, PluginContext, PluginError};
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

/// Global registries for plugin context
static GLOBAL_SKILL_REGISTRY: OnceCell<Arc<nexus_skills::SkillRegistry>> = OnceCell::new();
static GLOBAL_AI_REGISTRY: OnceCell<Arc<nexus_ai::AIProviderRegistry>> = OnceCell::new();

/// Set the global skill registry
pub fn set_global_skill_registry(registry: Arc<nexus_skills::SkillRegistry>) {
    GLOBAL_SKILL_REGISTRY.set(registry).ok();
}

/// Set the global AI registry
pub fn set_global_ai_registry(registry: Arc<nexus_ai::AIProviderRegistry>) {
    GLOBAL_AI_REGISTRY.set(registry).ok();
}

/// Get the global skill registry
pub fn get_global_skill_registry() -> Option<Arc<nexus_skills::SkillRegistry>> {
    GLOBAL_SKILL_REGISTRY.get().cloned()
}

/// Get the global AI registry
pub fn get_global_ai_registry() -> Option<Arc<nexus_ai::AIProviderRegistry>> {
    GLOBAL_AI_REGISTRY.get().cloned()
}

/// Plugin lifecycle state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginState {
    Loaded,
    Initialized,
    Running,
    ShuttingDown,
    ShutDown,
}

/// Information about a loaded plugin
struct LoadedPlugin {
    plugin: Arc<dyn Plugin>,
    state: PluginState,
}

impl LoadedPlugin {
    fn new(plugin: Arc<dyn Plugin>) -> Self {
        Self {
            plugin,
            state: PluginState::Loaded,
        }
    }
}

/// Registry for managing plugins
pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, LoadedPlugin>>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    /// Create a new empty plugin registry
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Load a plugin into the registry
    pub async fn load_plugin(&self, plugin: Arc<dyn Plugin>) -> Result<(), PluginError> {
        let id = plugin.metadata().id.clone();

        // Check if already loaded
        {
            let plugins = self.plugins.read();
            if plugins.contains_key(&id) {
                return Err(PluginError::AlreadyLoaded(id));
            }
        }

        // Initialize the plugin
        let context = self.create_context()?;
        plugin.initialize(&context).await?;

        // Store the plugin
        let mut plugins = self.plugins.write();
        plugins.insert(id.clone(), LoadedPlugin::new(plugin));

        info!(plugin_id = %id, "Plugin loaded and initialized");

        Ok(())
    }

    /// Unload a plugin from the registry
    pub async fn unload(&self, id: &str) -> Result<(), PluginError> {
        let plugin = {
            let mut plugins = self.plugins.write();
            let loaded = plugins
                .get_mut(id)
                .ok_or_else(|| PluginError::NotFound(id.to_string()))?;

            // Transition to shutting down
            loaded.state = PluginState::ShuttingDown;

            // Get the plugin Arc to call shutdown outside the lock
            loaded.plugin.clone()
        };

        // Shutdown outside the lock
        if let Err(e) = plugin.shutdown().await {
            error!(plugin_id = %id, error = %e, "Plugin shutdown returned error");
            // Continue with unload anyway
        }

        // Remove from registry
        let mut plugins = self.plugins.write();
        plugins.remove(id);

        info!(plugin_id = %id, "Plugin unloaded");

        Ok(())
    }

    /// Get a plugin by ID
    pub fn get(&self, id: &str) -> Option<Arc<dyn Plugin>> {
        let plugins = self.plugins.read();
        plugins.get(id).map(|loaded| loaded.plugin.clone())
    }

    /// Get all loaded plugin IDs
    pub fn ids(&self) -> Vec<String> {
        let plugins = self.plugins.read();
        plugins.keys().cloned().collect()
    }

    /// Get all plugins
    pub fn plugins(&self) -> Vec<Arc<dyn Plugin>> {
        let plugins = self.plugins.read();
        plugins
            .values()
            .map(|loaded| loaded.plugin.clone())
            .collect()
    }

    /// Get the state of a plugin
    pub fn state(&self, id: &str) -> Option<PluginState> {
        let plugins = self.plugins.read();
        plugins.get(id).map(|loaded| loaded.state.clone())
    }

    /// Check if a plugin is loaded
    pub fn is_loaded(&self, id: &str) -> bool {
        let plugins = self.plugins.read();
        plugins.contains_key(id)
    }

    /// Get count of loaded plugins
    pub fn len(&self) -> usize {
        let plugins = self.plugins.read();
        plugins.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        let plugins = self.plugins.read();
        plugins.is_empty()
    }

    /// Create a plugin context for initialization
    fn create_context(&self) -> Result<PluginContext, PluginError> {
        // Get skill registry from global or create a new one
        let skill_registry = GLOBAL_SKILL_REGISTRY
            .get()
            .cloned()
            .unwrap_or_else(|| Arc::new(nexus_skills::SkillRegistry::new()));

        // Get AI registry from global or create a new one
        let ai_registry = GLOBAL_AI_REGISTRY
            .get()
            .cloned()
            .unwrap_or_else(|| Arc::new(nexus_ai::AIProviderRegistry::new()));

        // Create a default config provider
        let config: Arc<dyn crate::plugin_trait::ConfigProvider> =
            Arc::new(std::collections::HashMap::new());

        Ok(PluginContext {
            skill_registry,
            ai_registry,
            config,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_trait::{PluginContext, PluginMetadata};
    use async_trait::async_trait;
    use std::any::Any;

    struct TestPlugin {
        metadata: PluginMetadata,
        initialized: std::sync::atomic::AtomicBool,
        shutdown: std::sync::atomic::AtomicBool,
    }

    impl TestPlugin {
        fn new(id: &str) -> Self {
            Self {
                metadata: PluginMetadata::new(id, format!("Test Plugin {}", id), "1.0.0"),
                initialized: std::sync::atomic::AtomicBool::new(false),
                shutdown: std::sync::atomic::AtomicBool::new(false),
            }
        }
    }

    #[async_trait]
    impl Plugin for TestPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }

        async fn initialize(&self, _context: &PluginContext) -> Result<(), PluginError> {
            self.initialized
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), PluginError> {
            self.shutdown
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[tokio::test]
    async fn test_load_and_unload_plugin() {
        let registry = PluginRegistry::new();

        // Load a plugin
        let plugin = Arc::new(TestPlugin::new("test-plugin"));
        registry.load_plugin(plugin.clone()).await.unwrap();

        // Verify it's loaded
        assert!(registry.is_loaded("test-plugin"));
        assert_eq!(registry.len(), 1);

        // Get the plugin
        let retrieved = registry.get("test-plugin").unwrap();
        assert_eq!(retrieved.metadata().id, "test-plugin");

        // Unload the plugin
        registry.unload("test-plugin").await.unwrap();

        // Verify it's unloaded
        assert!(!registry.is_loaded("test-plugin"));
        assert!(registry.is_empty());
    }

    #[tokio::test]
    async fn test_load_duplicate_plugin() {
        let registry = PluginRegistry::new();

        let plugin1 = Arc::new(TestPlugin::new("duplicate"));
        let plugin2 = Arc::new(TestPlugin::new("duplicate"));

        registry.load_plugin(plugin1).await.unwrap();

        let result = registry.load_plugin(plugin2).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PluginError::AlreadyLoaded(_)));
    }

    #[tokio::test]
    async fn test_get_nonexistent_plugin() {
        let registry = PluginRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }
}
