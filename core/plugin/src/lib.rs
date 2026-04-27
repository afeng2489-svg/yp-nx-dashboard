pub mod agent;
pub mod plugin_trait;
pub mod registry;

pub use agent::AgentPlugin;
pub use plugin_trait::{Plugin, PluginContext, PluginMetadata};
pub use registry::{
    get_global_ai_registry, get_global_skill_registry, set_global_ai_registry,
    set_global_skill_registry, PluginRegistry,
};
