//! NexusFlow API 服务器入口

use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use nx_api::{create_router, ApiConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .init();

    tracing::info!("启动 NexusFlow API 服务器...");
    println!("[STARTUP] NexusFlow API 服务器已启动 (debug logs enabled)"); // NEXUS-DEBUG

    // 加载配置
    let config = ApiConfig::load();

    tracing::info!("监听地址: {}:{}", config.host, config.port);

    // 创建路由器
    let (app, app_state) = create_router(config.clone());

    // 启动调度器 workers
    app_state.scheduler_state.start_workers(4);

    // 绑定地址
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = TcpListener::bind(addr).await?;

    tracing::info!("服务器启动成功，监听于 {}", addr);

    // 启动服务器
    axum::serve(listener, app).await?;

    Ok(())
}