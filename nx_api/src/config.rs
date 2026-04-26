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
            db_path: resolve_default_db_path(),
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
            db_path: resolve_db_path(),
        }
    }
}

/// 统一数据库路径入口
///
/// 1. 检查 NEXUS_DB_PATH 环境变量
/// 2. 验证：路径必须是绝对路径且位于 nx_dashboard/ 目录下
/// 3. 如不满足条件则自动解析
fn resolve_db_path() -> String {
    if let Ok(env_path) = std::env::var("NEXUS_DB_PATH") {
        let p = std::path::Path::new(&env_path);
        if p.is_absolute() {
            // 确保父目录存在
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            eprintln!("[DB] NEXUS_DB_PATH (validated): {}", env_path);
            return env_path;
        }
        eprintln!(
            "[DB] WARNING: NEXUS_DB_PATH='{}' 无效（必须是绝对路径），忽略并自动解析",
            env_path
        );
    } else {
        eprintln!("[DB] NEXUS_DB_PATH not set, auto-resolving...");
    }
    resolve_default_db_path()
}

/// 查找统一的数据库路径
///
/// 所有启动方式（Tauri桌面应用、cargo run、release二进制）都必须解析到同一个绝对路径：
///   <workspace_root>/nx_dashboard/nexus.db
///
/// 判断 workspace root 的标志：目录同时包含 Cargo.toml 和 nx_dashboard/ 子目录
fn resolve_default_db_path() -> String {
    let db_subpath = std::path::Path::new("nx_dashboard").join("nexus.db");

    // 辅助函数：判断某个目录是否为 workspace root
    let is_workspace_root = |dir: &std::path::Path| -> bool {
        dir.join("Cargo.toml").exists() && dir.join("nx_dashboard").is_dir()
    };

    // 策略1: 基于可执行文件位置向上查找 workspace root
    if let Ok(exe) = std::env::current_exe() {
        // 先 canonicalize 解析符号链接
        let exe = exe.canonicalize().unwrap_or(exe);
        for ancestor in exe.ancestors().skip(1) {
            if is_workspace_root(ancestor) {
                let db_path = ancestor.join(&db_subpath);
                let _ = std::fs::create_dir_all(db_path.parent().unwrap());
                eprintln!("[DB resolve] 策略1(exe): {} => {}", exe.display(), db_path.display());
                return db_path.to_string_lossy().to_string();
            }
        }
    }

    // 策略2: 基于当前工作目录向上查找 workspace root
    if let Ok(cwd) = std::env::current_dir() {
        for ancestor in cwd.ancestors() {
            if is_workspace_root(ancestor) {
                let db_path = ancestor.join(&db_subpath);
                let _ = std::fs::create_dir_all(db_path.parent().unwrap());
                eprintln!("[DB resolve] 策略2(cwd): {} => {}", cwd.display(), db_path.display());
                return db_path.to_string_lossy().to_string();
            }
        }
    }

    // 策略3: 基于 CARGO_MANIFEST_DIR 编译期嵌入的路径（仅开发环境有效）
    let compile_time_root = env!("CARGO_MANIFEST_DIR"); // nx_api/
    let manifest_dir = std::path::Path::new(compile_time_root);
    if let Some(parent) = manifest_dir.parent() {
        if is_workspace_root(parent) {
            let db_path = parent.join(&db_subpath);
            let _ = std::fs::create_dir_all(db_path.parent().unwrap());
            eprintln!("[DB resolve] 策略3(compile-time): {} => {}", compile_time_root, db_path.display());
            return db_path.to_string_lossy().to_string();
        }
    }

    // 最终 fallback: 绝对路径 ~/.nexus/nexus.db（绝不使用相对路径）
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let fallback_dir = std::path::Path::new(&home).join(".nexus");
    let _ = std::fs::create_dir_all(&fallback_dir);
    let db_path = fallback_dir.join("nexus.db");
    eprintln!("[DB resolve] 最终fallback: {}", db_path.display());
    db_path.to_string_lossy().to_string()
}