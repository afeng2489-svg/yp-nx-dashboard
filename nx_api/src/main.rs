//! NexusFlow API 服务器入口

use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use nx_api::{create_router, ApiConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 解析命令行参数（--db-path 优先级高于环境变量 NEXUS_DB_PATH）
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--db-path" => {
                if i + 1 < args.len() {
                    std::env::set_var("NEXUS_DB_PATH", &args[i + 1]);
                    i += 1;
                } else {
                    eprintln!("[ERROR] --db-path 需要指定路径参数");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                println!("NexusFlow API 服务器");
                println!("用法: nx_api [选项]");
                println!();
                println!("选项:");
                println!("  --db-path <PATH>   指定数据库文件路径");
                println!("  --help, -h         显示此帮助信息");
                println!();
                println!("环境变量:");
                println!("  NEXUS_DB_PATH       数据库路径（--db-path 覆盖）");
                println!("  NEXUS_API_PORT      监听端口（默认 8080）");
                println!("  CLAUDE_CLI_PATH_OVERRIDE  手动指定 Claude CLI 路径");
                println!("  RUST_LOG            日志级别（默认 info）");
                std::process::exit(0);
            }
            unknown => {
                eprintln!("[ERROR] 未知参数: {}（使用 --help 查看帮助）", unknown);
                std::process::exit(1);
            }
        }
        i += 1;
    }
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("启动 NexusFlow API 服务器...");
    tracing::info!("[STARTUP] NexusFlow API 服务器已启动 (debug logs enabled)");

    // 启动早期解析 Claude CLI 路径：
    // 用户配置（最高优先级）→ 智能搜索 → 写入 CLAUDE_CLI_PATH_OVERRIDE 让 engine.rs 也能读到
    nx_api::services::claude_cli::init_at_startup();

    // 加载配置
    let config = ApiConfig::load();

    tracing::info!("监听地址: {}:{}", config.host, config.port);

    // 创建路由器
    let (app, _app_state) = create_router(config.clone())?;

    // 绑定地址
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = TcpListener::bind(addr).await?;

    tracing::info!("服务器启动成功，监听于 {}", addr);

    // 启动服务器
    axum::serve(listener, app).await?;

    Ok(())
}
