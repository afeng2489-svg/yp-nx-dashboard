//! CLI Registry
//!
//! 管理 CLI 的注册、检测和选择。

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use super::{AIError, CLIContext, CLIResponse, CLI};

/// CLI 能力描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLICapability {
    /// CLI 类型
    pub cli: CLI,
    /// 是否可用
    pub available: bool,
    /// 版本信息
    pub version: Option<String>,
    /// 支持的功能
    pub features: Vec<String>,
}

/// CLI 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLIConfig {
    /// CLI 类型
    pub cli: CLI,
    /// 是否启用
    pub enabled: bool,
    /// 路径（如果需要自定义）
    pub path: Option<String>,
    /// 额外参数
    pub extra_params: HashMap<String, String>,
}

impl CLIConfig {
    /// 创建默认配置
    pub fn default_for(cli: CLI) -> Self {
        Self {
            cli,
            enabled: true,
            path: None,
            extra_params: HashMap::new(),
        }
    }
}

/// CLI 选择策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum CLISelectionStrategy {
    /// 自动选择（基于提示词分析）
    #[default]
    Auto,
    /// 语义选择（分析提示词选择最佳 CLI）
    Semantic,
    /// 手动选择
    Manual,
    /// 回退到默认
    Fallback,
}

/// CLI 注册表
pub struct CLIRegistry {
    /// CLI 配置
    configs: RwLock<HashMap<CLI, CLIConfig>>,
    /// CLI 能力缓存
    capabilities: RwLock<HashMap<CLI, CLICapability>>,
    /// 当前选择的策略
    selection_strategy: RwLock<CLISelectionStrategy>,
    /// 默认 CLI
    default_cli: RwLock<Option<CLI>>,
}

impl CLIRegistry {
    /// 创建新的 CLI 注册表
    pub fn new() -> Self {
        let registry = Self {
            configs: RwLock::new(HashMap::new()),
            capabilities: RwLock::new(HashMap::new()),
            selection_strategy: RwLock::new(CLISelectionStrategy::default()),
            default_cli: RwLock::new(Some(CLI::Claude)),
        };

        // 初始化默认配置
        registry.initialize_defaults();
        registry
    }

    /// 初始化默认 CLI 配置
    fn initialize_defaults(&self) {
        let mut configs = self.configs.write();
        for cli in [
            CLI::Claude,
            CLI::Gemini,
            CLI::Codex,
            CLI::Qwen,
            CLI::OpenCode,
        ] {
            configs.insert(cli, CLIConfig::default_for(cli));
        }
    }

    /// 注册 CLI 配置
    pub fn register_config(&self, config: CLIConfig) {
        self.configs.write().insert(config.cli, config);
    }

    /// 获取 CLI 配置
    pub fn get_config(&self, cli: CLI) -> Option<CLIConfig> {
        self.configs.read().get(&cli).cloned()
    }

    /// 获取所有 CLI 配置
    pub fn all_configs(&self) -> Vec<CLIConfig> {
        self.configs.read().values().cloned().collect()
    }

    /// 更新 CLI 能力
    pub fn update_capability(&self, capability: CLICapability) {
        self.capabilities.write().insert(capability.cli, capability);
    }

    /// 获取 CLI 能力
    pub fn get_capability(&self, cli: CLI) -> Option<CLICapability> {
        self.capabilities.read().get(&cli).cloned()
    }

    /// 获取所有已检测的能力
    pub fn all_capabilities(&self) -> Vec<CLICapability> {
        self.capabilities.read().values().cloned().collect()
    }

    /// 设置选择策略
    pub fn set_selection_strategy(&self, strategy: CLISelectionStrategy) {
        *self.selection_strategy.write() = strategy;
    }

    /// 获取选择策略
    pub fn get_selection_strategy(&self) -> CLISelectionStrategy {
        *self.selection_strategy.read()
    }

    /// 设置默认 CLI
    pub fn set_default_cli(&self, cli: CLI) {
        *self.default_cli.write() = Some(cli);
    }

    /// 获取默认 CLI
    pub fn get_default_cli(&self) -> Option<CLI> {
        *self.default_cli.read()
    }

    /// 获取所有可用的 CLI
    pub fn available_clis(&self) -> Vec<CLI> {
        self.configs
            .read()
            .iter()
            .filter(|(_, config)| config.enabled)
            .map(|(cli, _)| *cli)
            .collect()
    }

    /// 检查 CLI 是否启用
    pub fn is_enabled(&self, cli: CLI) -> bool {
        self.configs
            .read()
            .get(&cli)
            .map(|c| c.enabled)
            .unwrap_or(false)
    }

    /// 启用/禁用 CLI
    pub fn set_enabled(&self, cli: CLI, enabled: bool) {
        if let Some(config) = self.configs.write().get_mut(&cli) {
            config.enabled = enabled;
        }
    }

    /// 基于提示词分析选择最佳 CLI
    pub fn select_cli_for_prompt(&self, prompt: &str) -> CLI {
        let prompt_lower = prompt.to_lowercase();

        // 简单的关键字匹配策略
        // Claude - 通用对话、复杂推理、代码审查
        if prompt_lower.contains("review")
            || prompt_lower.contains("explain")
            || prompt_lower.contains("debug")
            || prompt_lower.contains("refactor")
        {
            return CLI::Claude;
        }

        // Gemini - 多模态、长上下文、创意任务
        if prompt_lower.contains("image")
            || prompt_lower.contains("video")
            || prompt_lower.contains("creative")
            || prompt_lower.contains("long context")
        {
            return CLI::Gemini;
        }

        // Codex - 代码生成、调试、重构
        if prompt_lower.contains("write code")
            || prompt_lower.contains("implement")
            || prompt_lower.contains("function")
            || prompt_lower.contains("algorithm")
        {
            return CLI::Codex;
        }

        // Qwen - 中文支持、数学、逻辑
        let has_cjk = prompt
            .chars()
            .any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c));
        if has_cjk
            || prompt_lower.contains("中文")
            || prompt_lower.contains("math")
            || prompt_lower.contains("逻辑")
        {
            return CLI::Qwen;
        }

        // OpenCode - 开源项目、热门框架
        if prompt_lower.contains("github")
            || prompt_lower.contains("open source")
            || prompt_lower.contains("popular framework")
        {
            return CLI::OpenCode;
        }

        // 回退到默认
        self.get_default_cli().unwrap_or(CLI::Claude)
    }

    /// 根据策略选择 CLI
    pub fn select_cli(&self, prompt: &str, manual_cli: Option<CLI>) -> Result<CLI, AIError> {
        let strategy = self.get_selection_strategy();

        match strategy {
            CLISelectionStrategy::Manual => {
                manual_cli.ok_or_else(|| AIError::InvalidRequest("需要指定 CLI".to_string()))
            }
            CLISelectionStrategy::Semantic | CLISelectionStrategy::Auto => {
                Ok(self.select_cli_for_prompt(prompt))
            }
            CLISelectionStrategy::Fallback => Ok(self.get_default_cli().unwrap_or(CLI::Claude)),
        }
    }
}

impl Default for CLIRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// CLI 执行器 trait
#[async_trait::async_trait]
pub trait CLIExecutor: Send + Sync {
    /// 执行 CLI 命令
    async fn execute(
        &self,
        cli: CLI,
        prompt: &str,
        context: &CLIContext,
    ) -> Result<CLIResponse, AIError>;

    /// 检测 CLI 是否可用
    async fn detect(&self, cli: CLI) -> Result<CLICapability, AIError>;
}

/// 语义 CLI 选择器
pub struct SemanticCLISelector {
    registry: Arc<CLIRegistry>,
}

impl SemanticCLISelector {
    /// 创建新的语义选择器
    pub fn new(registry: Arc<CLIRegistry>) -> Self {
        Self { registry }
    }

    /// 分析提示词并返回最佳 CLI
    pub fn analyze_and_select(&self, prompt: &str) -> CLI {
        self.registry.select_cli_for_prompt(prompt)
    }

    /// 获取选择理由
    pub fn get_selection_reason(&self, cli: CLI, prompt: &str) -> String {
        let prompt_lower = prompt.to_lowercase();

        match cli {
            CLI::Claude => {
                if prompt_lower.contains("review") {
                    "适合代码审查".to_string()
                } else if prompt_lower.contains("explain") {
                    "适合代码解释".to_string()
                } else if prompt_lower.contains("debug") {
                    "适合调试问题".to_string()
                } else {
                    "通用对话和复杂推理".to_string()
                }
            }
            CLI::Gemini => {
                if prompt_lower.contains("image") || prompt_lower.contains("video") {
                    "多模态处理能力".to_string()
                } else {
                    "长上下文和创意任务".to_string()
                }
            }
            CLI::Codex => "代码生成专长".to_string(),
            CLI::Qwen => "中文支持和数学推理".to_string(),
            CLI::OpenCode => "开源项目支持".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_display_name() {
        assert_eq!(CLI::Claude.display_name(), "Claude");
        assert_eq!(CLI::Gemini.display_name(), "Gemini");
        assert_eq!(CLI::Codex.display_name(), "Codex");
    }

    #[test]
    fn test_registry_default_cli() {
        let registry = CLIRegistry::new();
        assert_eq!(registry.get_default_cli(), Some(CLI::Claude));
    }

    #[test]
    fn test_select_cli_for_prompt() {
        let registry = CLIRegistry::new();

        // 测试代码审查场景
        let cli = registry.select_cli_for_prompt("Please review this code");
        assert_eq!(cli, CLI::Claude);

        // 测试代码生成场景
        let cli = registry.select_cli_for_prompt("Write a function to sort an array");
        assert_eq!(cli, CLI::Codex);

        // 测试中文场景
        let cli = registry.select_cli_for_prompt("请解释这段代码");
        assert_eq!(cli, CLI::Qwen);
    }

    #[test]
    fn test_cli_config() {
        let config = CLIConfig::default_for(CLI::Gemini);
        assert_eq!(config.cli, CLI::Gemini);
        assert!(config.enabled);
        assert!(config.path.is_none());
    }
}
