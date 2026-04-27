//! 配置管理

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 提供商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// API 密钥（用于云提供商）
    #[serde(default)]
    pub api_key: Option<String>,

    /// 基础 URL（用于自托管）
    #[serde(default)]
    pub base_url: Option<String>,

    /// 可用模型
    #[serde(default)]
    pub models: Option<Vec<String>>,

    /// 默认模型
    #[serde(default)]
    pub default_model: Option<String>,
}

/// NexusFlow 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// AI 提供商
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,

    /// 默认提供商名称
    #[serde(default = "default_provider")]
    pub default_provider: Option<String>,

    /// 工作区目录
    #[serde(default = "default_workspace")]
    pub workspace_dir: PathBuf,

    /// 数据库路径
    #[serde(default = "default_db")]
    pub db_path: Option<PathBuf>,

    /// 日志级别
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// 沙箱设置
    #[serde(default)]
    pub sandbox: SandboxConfig,
}

fn default_provider() -> Option<String> {
    Some("anthropic".to_string())
}

fn default_workspace() -> PathBuf {
    PathBuf::from("./workspace")
}

fn default_db() -> Option<PathBuf> {
    Some(PathBuf::from("nexus.db"))
}

fn default_log_level() -> String {
    "info".to_string()
}

/// 沙箱配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// 启用沙箱
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// 内存限制（字节）
    #[serde(default = "default_memory_limit")]
    pub memory_limit: u64,

    /// CPU 时间限制（秒）
    #[serde(default = "default_cpu_limit")]
    pub cpu_limit: u64,

    /// 超时时间（秒）
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_true() -> bool {
    true
}
fn default_memory_limit() -> u64 {
    256 * 1024 * 1024
}
fn default_cpu_limit() -> u64 {
    10
}
fn default_timeout() -> u64 {
    30
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            memory_limit: default_memory_limit(),
            cpu_limit: default_cpu_limit(),
            timeout: default_timeout(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            providers: HashMap::new(),
            default_provider: default_provider(),
            workspace_dir: default_workspace(),
            db_path: default_db(),
            log_level: default_log_level(),
            sandbox: SandboxConfig::default(),
        }
    }
}

/// 从文件加载配置
pub fn load_config(path: &PathBuf) -> anyhow::Result<Config> {
    if !path.exists() {
        tracing::warn!("配置文件 {:?} 不存在，使用默认配置", path);
        return Ok(Config::default());
    }

    let content = std::fs::read_to_string(path)?;

    // 优先尝试 YAML，然后尝试 JSON
    let config: Config = serde_yaml::from_str(&content)
        .or_else(|_| serde_json::from_str(&content))
        .map_err(|e| anyhow::anyhow!("解析配置失败: {}", e))?;

    // 展开 API 密钥中的环境变量
    let config = expand_env_vars(config);

    Ok(config)
}

/// 展开配置中的环境变量
fn expand_env_vars(mut config: Config) -> Config {
    for provider in config.providers.values_mut() {
        if let Some(ref api_key) = provider.api_key {
            if api_key.starts_with("${") && api_key.ends_with("}") {
                let var_name = &api_key[2..api_key.len() - 1];
                if let Ok(value) = std::env::var(var_name) {
                    provider.api_key = Some(value);
                }
            }
        }
        if let Some(ref base_url) = provider.base_url {
            if base_url.starts_with("${") && base_url.ends_with("}") {
                let var_name = &base_url[2..base_url.len() - 1];
                if let Ok(value) = std::env::var(var_name) {
                    provider.base_url = Some(value);
                }
            }
        }
    }
    config
}

/// 创建默认配置文件
pub fn create_default_config(path: &PathBuf) -> anyhow::Result<()> {
    let config = Config::default();
    let yaml = serde_yaml::to_string(&config)?;
    std::fs::write(path, yaml)?;
    Ok(())
}
