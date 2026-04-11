//! API 服务器配置

use serde::{Deserialize, Serialize};

/// API 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// 监听主机
    pub host: String,
    /// 监听端口
    pub port: u16,
    /// API 密钥（用于认证）
    pub api_key: Option<String>,
    /// 启用 CORS
    pub cors_enabled: bool,
    /// 允许的来源
    pub allowed_origins: Vec<String>,
    /// 最大并发执行数
    pub max_concurrent_executions: usize,
    /// 默认超时时间（秒）
    pub default_timeout_secs: u64,
    /// 数据库路径
    pub db_path: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            api_key: None,
            cors_enabled: true,
            allowed_origins: vec!["http://localhost:3000".to_string()],
            max_concurrent_executions: 10,
            default_timeout_secs: 300,
            db_path: "nexus.db".to_string(),
        }
    }
}

impl ApiConfig {
    /// 从环境变量加载配置
    pub fn load() -> Self {
        Self {
            host: std::env::var("NEXUS_API_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("NEXUS_API_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            api_key: std::env::var("NEXUS_API_KEY").ok(),
            cors_enabled: std::env::var("NEXUS_CORS_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            allowed_origins: std::env::var("NEXUS_ALLOWED_ORIGINS")
                .map(|s| s.split(',').map(|v| v.trim().to_string()).collect())
                .unwrap_or_else(|_| vec!["http://localhost:3000".to_string()]),
            max_concurrent_executions: std::env::var("NEXUS_MAX_CONCURRENT")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            default_timeout_secs: std::env::var("NEXUS_DEFAULT_TIMEOUT")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
            db_path: std::env::var("NEXUS_DB_PATH")
                .map(|p| {
                    if p.starts_with('/') {
                        p  // Absolute path
                    } else {
                        // Convert relative path to absolute based on current working directory
                        std::env::current_dir()
                            .map(|cwd| cwd.join(&p).to_string_lossy().to_string())
                            .unwrap_or(p)
                    }
                })
                .unwrap_or_else(|_| {
                    // Default to absolute path
                    std::env::current_dir()
                        .map(|cwd| cwd.join("nexus.db").to_string_lossy().to_string())
                        .unwrap_or_else(|_| "nexus.db".to_string())
                }),
        }
    }
}