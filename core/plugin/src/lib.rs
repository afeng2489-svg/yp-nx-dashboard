pub mod plugin_trait;
pub mod registry;
pub mod agent;

pub use plugin_trait::{Plugin, PluginMetadata, PluginContext};
pub use registry::{PluginRegistry, set_global_skill_registry, set_global_ai_registry, get_global_skill_registry, get_global_ai_registry};
pub use agent::AgentPlugin;
