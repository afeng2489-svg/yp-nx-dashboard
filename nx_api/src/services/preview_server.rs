//! 预览服务器管理
//!
//! 为项目启动前端开发服务器并提供预览 URL。
//! 包含端口仲裁、Node.js 检测、健康探测、资源限制和空闲回收。

use parking_lot::RwLock;
use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::process::{Child, Command};
use tokio::time::interval;

/// 空闲超时：30 分钟无请求即回收（前端预览页会定期 ping status 保活）
const IDLE_TIMEOUT_SECS: u64 = 1800;
/// SIGTERM 后等待 SIGKILL 的时间
const KILL_GRACE_SECS: u64 = 9;
/// 健康探测间隔
const PROBE_INTERVAL_MS: u64 = 500;
/// 健康探测最大等待时间
const PROBE_TIMEOUT_SECS: u64 = 30;

/// 预览会话
#[derive(Debug)]
pub struct PreviewSession {
    pub session_id: String,
    pub project_id: String,
    pub project_path: String,
    pub port: u16,
    pub preview_url: String,
    pub status: PreviewStatus,
    started_at: Instant,
    last_request_at: Instant,
    child: Option<Child>,
}

/// 预览状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewStatus {
    Starting,
    Running,
    Stopped,
    Failed,
}

impl PreviewSession {
    /// 记录一次请求（刷新空闲计时器）
    fn touch(&mut self) {
        self.last_request_at = Instant::now();
    }

    /// 检查是否已空闲超时
    fn is_idle(&self) -> bool {
        self.last_request_at.elapsed().as_secs() >= IDLE_TIMEOUT_SECS
    }
}

/// 预览服务器管理器
#[derive(Clone)]
pub struct PreviewServerManager {
    sessions: Arc<RwLock<HashMap<String, Arc<RwLock<PreviewSession>>>>>,
}

impl PreviewServerManager {
    pub fn new() -> Self {
        let manager = Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        };
        manager.spawn_idle_reaper();
        manager
    }

    /// 启动预览服务器
    pub async fn start(
        &self,
        project_id: &str,
        project_path: &str,
    ) -> Result<PreviewSessionInfo, PreviewError> {
        let npm_path = detect_node().await?;
        let session_id = uuid::Uuid::new_v4().to_string();

        // npm install（如 node_modules 不存在）
        let node_modules = std::path::Path::new(project_path).join("node_modules");
        if !node_modules.exists() {
            tracing::info!(session_id = %session_id, "安装依赖: npm install");
            let install = Command::new(&npm_path)
                .arg("install")
                .current_dir(project_path)
                .output()
                .await;
            match install {
                Ok(out) if !out.status.success() => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    tracing::error!(session_id = %session_id, "npm install 失败: {}", stderr);
                    return Err(PreviewError::InstallFailed(stderr.trim().to_string()));
                }
                Err(e) => {
                    tracing::error!(session_id = %session_id, error = %e, "npm install 失败");
                    return Err(PreviewError::InstallFailed(e.to_string()));
                }
                _ => {}
            }
        }

        // 端口候选：优先随机空闲端口（避开用户常用的 3000/5173），再退回固定端口。
        // 每个候选都以 --strictPort 启动：端口被占时 vite 会立即退出而不是悄悄漂移到
        // 别的端口，于是我们能秒级换端口重试，而不是死等 30 秒健康探测超时。
        let mut candidates: Vec<u16> = Vec::new();
        if let Ok(p) = random_available_port().await {
            candidates.push(p);
        }
        for p in [5173u16, 3000u16] {
            if port_is_available(p) && !candidates.contains(&p) {
                candidates.push(p);
            }
        }
        if let Ok(p) = random_available_port().await {
            if !candidates.contains(&p) {
                candidates.push(p);
            }
        }
        if candidates.is_empty() {
            return Err(PreviewError::NoPortAvailable);
        }

        let mut last_err: Option<PreviewError> = None;
        for port in candidates {
            let preview_url = format!("http://127.0.0.1:{}", port);

            // 启动 dev server（--strictPort：端口被占即退出，不漂移）
            // --host 0.0.0.0：绑定所有接口（IPv4+IPv6），避免 reqwest 无法连接纯 IPv6 localhost
            let mut child = Command::new(&npm_path)
                .arg("run")
                .arg("dev")
                .arg("--")
                .arg("--port")
                .arg(port.to_string())
                .arg("--strictPort")
                .arg("--host")
                .arg("0.0.0.0")
                .current_dir(project_path)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true)
                .spawn()
                .map_err(|e| PreviewError::SpawnFailed(e.to_string()))?;

            tracing::info!(session_id = %session_id, port = port, "启动预览，等待健康探测...");
            match wait_ready_or_exit(&mut child, port, PROBE_TIMEOUT_SECS).await {
                Ok(_) => {
                    let session = PreviewSession {
                        session_id: session_id.clone(),
                        project_id: project_id.to_string(),
                        project_path: project_path.to_string(),
                        port,
                        preview_url: preview_url.clone(),
                        status: PreviewStatus::Running,
                        started_at: Instant::now(),
                        last_request_at: Instant::now(),
                        child: Some(child),
                    };
                    self.sessions
                        .write()
                        .insert(session_id.clone(), Arc::new(RwLock::new(session)));
                    tracing::info!(session_id = %session_id, url = %preview_url, "预览就绪");
                    return Ok(PreviewSessionInfo {
                        session_id,
                        project_id: project_id.to_string(),
                        port,
                        preview_url,
                        status: PreviewStatus::Running,
                    });
                }
                Err(e) => {
                    // 关键：失败时立即杀掉子进程，避免泄漏 vite 进程继续占用端口，
                    // 否则后续仲裁会越来越糟（端口雪崩）。
                    let _ = child.start_kill();
                    tracing::warn!(session_id = %session_id, port = port, "该端口启动失败，换端口重试: {}", e);
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or(PreviewError::NoPortAvailable))
    }

    /// 停止预览服务器
    pub async fn stop(&self, session_id: &str) -> bool {
        let session = {
            let sessions = self.sessions.read();
            sessions.get(session_id).cloned()
        };

        match session {
            Some(s) => {
                let mut s = s.write();
                if let Some(mut child) = s.child.take() {
                    // 发送 SIGTERM
                    if let Some(pid) = child.id() {
                        let pid_str = pid.to_string();
                        let _ = std::process::Command::new("kill")
                            .arg("-TERM")
                            .arg(&pid_str)
                            .status();
                    }

                    // 9 秒后 SIGKILL
                    let session_id = session_id.to_string();
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(KILL_GRACE_SECS)).await;
                        let _ = child.kill().await;
                        tracing::info!(session_id = %session_id, "进程已强制终止");
                    });
                }
                s.status = PreviewStatus::Stopped;
                tracing::info!(session_id = %session_id, "预览已停止");
                true
            }
            None => false,
        }
    }

    /// 查询预览状态（同时刷新空闲计时：浏览页面的轮询即视为活跃，避免被回收）
    pub fn status(&self, session_id: &str) -> Option<PreviewSessionInfo> {
        let sessions = self.sessions.read();
        sessions.get(session_id).map(|s| {
            let mut s = s.write();
            s.touch();
            PreviewSessionInfo {
                session_id: s.session_id.clone(),
                project_id: s.project_id.clone(),
                port: s.port,
                preview_url: s.preview_url.clone(),
                status: s.status,
            }
        })
    }

    /// 记录一次请求（用于空闲计时器刷新）
    pub fn touch(&self, session_id: &str) {
        let sessions = self.sessions.read();
        if let Some(s) = sessions.get(session_id) {
            s.write().touch();
        }
    }

    /// 启动空闲回收后台任务
    fn spawn_idle_reaper(&self) {
        let sessions = self.sessions.clone();
        tokio::spawn(async move {
            let mut tick = interval(Duration::from_secs(30));
            loop {
                tick.tick().await;
                let to_reap: Vec<String> = {
                    let all = sessions.read();
                    all.iter()
                        .filter(|(_, s)| s.read().is_idle())
                        .map(|(id, _)| id.clone())
                        .collect()
                };
                for id in to_reap {
                    let child = {
                        let sessions = sessions.read();
                        sessions.get(&id).and_then(|s| s.write().child.take())
                    };
                    if let Some(mut child) = child {
                        tracing::info!(session_id = %id, "空闲超时，回收预览服务器");
                        let _ = child.kill().await;
                    }
                    sessions.write().remove(&id);
                }
            }
        });
    }
}

impl Default for PreviewServerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 预览会话信息（外部 API 返回）
#[derive(Debug, Clone, serde::Serialize)]
pub struct PreviewSessionInfo {
    pub session_id: String,
    pub project_id: String,
    pub port: u16,
    pub preview_url: String,
    pub status: PreviewStatus,
}

fn port_is_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

async fn random_available_port() -> Result<u16, PreviewError> {
    // 在 1024..65535 范围内尝试绑定随机端口
    for _ in 0..100 {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|e| PreviewError::PortAlloc(e.to_string()))?;
        let port = listener
            .local_addr()
            .map_err(|e| PreviewError::PortAlloc(e.to_string()))?
            .port();
        drop(listener);

        // 立即重试绑定确认可用
        if port_is_available(port) {
            return Ok(port);
        }
    }
    Err(PreviewError::NoPortAvailable)
}

/// 检测 npm：which npm → nvm exec
async fn detect_node() -> Result<String, PreviewError> {
    let output = Command::new("which")
        .arg("npm")
        .output()
        .await
        .map_err(|e| PreviewError::NodeNotFound(e.to_string()))?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(path);
        }
    }

    // 回退 nvm
    let nvm_output = Command::new("bash")
        .arg("-c")
        .arg("source ~/.nvm/nvm.sh && nvm exec npm --version")
        .output()
        .await
        .map_err(|e| PreviewError::NodeNotFound(e.to_string()))?;

    if nvm_output.status.success() {
        return Ok("npm".to_string());
    }

    Err(PreviewError::NodeNotFound(
        "未找到 npm，请安装 Node.js 或 nvm".to_string(),
    ))
}

/// 健康探测：轮询 TCP 连接直到成功或超时；同时检测 dev 进程是否提前退出
/// （端口被占用时 --strictPort 会让 vite 立即退出，此时无需再等满超时）。
/// 使用 TCP 连接而非 HTTP 请求，避免 reqwest 的 localhost 解析问题。
async fn wait_ready_or_exit(
    child: &mut Child,
    port: u16,
    timeout_secs: u64,
) -> Result<(), PreviewError> {
    let start = Instant::now();

    loop {
        // dev 进程提前退出（多半是端口冲突 / 启动报错）→ 立即失败，触发换端口重试
        if let Ok(Some(status)) = child.try_wait() {
            return Err(PreviewError::SpawnFailed(format!(
                "dev server 在端口 {} 上提前退出 (code={:?})",
                port,
                status.code()
            )));
        }

        if start.elapsed().as_secs() >= timeout_secs {
            return Err(PreviewError::HealthCheck(format!(
                "端口 {} 在 {} 秒内未就绪",
                port, timeout_secs
            )));
        }

        // 简单 TCP 连接检查：端口可连接即认为就绪
        if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(PROBE_INTERVAL_MS)).await;
    }
}

/// 预览服务错误
#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    #[error("Node.js 未找到: {0}")]
    NodeNotFound(String),

    #[error("端口分配失败: {0}")]
    PortAlloc(String),

    #[error("无可用端口")]
    NoPortAvailable,

    #[error("依赖安装失败: {0}")]
    InstallFailed(String),

    #[error("进程启动失败: {0}")]
    SpawnFailed(String),

    #[error("健康检查失败: {0}")]
    HealthCheck(String),

    #[error("会话未找到: {0}")]
    SessionNotFound(String),
}
