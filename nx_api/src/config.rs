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
                .unwrap_or_else(|_| resolve_default_db_path()),
        }
    }
}

/// 查找统一的数据库路径
/// 优先级：可执行文件所在项目根/nx_dashboard/nexus.db → 当前目录向上查找 → fallback nexus.db
fn resolve_default_db_path() -> String {
    // 策略1: 基于可执行文件位置 (target/release/nx_api → 项目根)
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors().skip(1) {
            let candidate = ancestor.join("nx_dashboard").join("nexus.db");
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
    }

    // 策略2: 基于当前工作目录向上查找
    if let Ok(cwd) = std::env::current_dir() {
        for ancestor in cwd.ancestors() {
            let candidate = ancestor.join("nx_dashboard").join("nexus.db");
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
    }

    // fallback
    "nexus.db".to_string()
}