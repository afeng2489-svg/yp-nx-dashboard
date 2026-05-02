//! Claude 终端服务
//!
//! 管理团队角色的 PTY 终端会话，每个会话运行一个持久的 `claude` 交互进程。
//! 前端通过 xterm.js 渲染原始终端输出，可直接在应用内看到 Claude 的完整执行过程。

use parking_lot::RwLock;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

/// 终端会话元信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct TerminalSessionInfo {
    pub session_id: String,
    pub team_id: String,
    pub role_id: Option<String>,
    pub working_dir: Option<String>,
}

/// PTY 输入命令（统一通过单一 channel 传给 PTY 线程）
enum PtyInput {
    /// 原始键盘字节
    Data(Vec<u8>),
    /// 终端尺寸变化
    Resize { rows: u16, cols: u16 },
}

/// Claude 终端会话（持久的 claude 交互进程）
pub struct ClaudeTerminalSession {
    pub info: TerminalSessionInfo,
    /// 原始终端输出 -> 前端 xterm.js（广播）
    pub output_tx: broadcast::Sender<Vec<u8>>,
    /// PTY 输入命令（键盘数据 + resize）
    input_tx: mpsc::UnboundedSender<PtyInput>,
}

impl ClaudeTerminalSession {
    /// 创建新的终端会话，启动 `claude --dangerously-skip-permissions` 交互进程
    pub fn new(info: TerminalSessionInfo, cols: u16, rows: u16) -> Self {
        let (output_tx, _) = broadcast::channel(2048);
        let (input_tx, input_rx) = mpsc::unbounded_channel::<PtyInput>();

        let output_tx_clone = output_tx.clone();
        let working_dir = info.working_dir.clone();

        // 在专用线程中运行 PTY（portable-pty 使用阻塞 I/O）
        std::thread::spawn(move || {
            run_pty_session(output_tx_clone, input_rx, working_dir, cols, rows);
        });

        Self {
            info,
            output_tx,
            input_tx,
        }
    }

    /// 订阅终端输出（每个 WebSocket 客户端订阅一个 receiver）
    pub fn subscribe_output(&self) -> broadcast::Receiver<Vec<u8>> {
        self.output_tx.subscribe()
    }

    /// 发送原始键盘数据到 PTY（从 xterm.js 传入）
    pub fn send_input(&self, data: Vec<u8>) {
        let _ = self.input_tx.send(PtyInput::Data(data));
    }

    /// 发送一个 Enter 键（CR，用于确认 workspace trust dialog 等）
    pub fn send_enter(&self) {
        let _ = self.input_tx.send(PtyInput::Data(vec![b'\r']));
    }

    /// 向 PTY 发送一段任务文本并自动提交
    ///
    /// claude code 的 TUI 输入框：
    /// - `\n` 是输入框内换行（不提交）
    /// - `\r` (CR / Enter) 才是提交
    /// - 大段文字会被识别为粘贴，需要 Enter 才会 submit
    ///
    /// 流程：写入文字 → 等 claude 回显 → 发 Enter 提交
    pub fn dispatch_task(&self, task: &str) {
        let _ = self.input_tx.send(PtyInput::Data(task.as_bytes().to_vec()));
        // 等 claude 把输入文字回显到屏幕（粘贴大段文字时尤其需要）
        std::thread::sleep(std::time::Duration::from_millis(800));
        // 发 Enter (CR) 提交输入
        let _ = self.input_tx.send(PtyInput::Data(vec![b'\r']));
    }

    /// 调整 PTY 终端尺寸
    pub fn resize(&self, rows: u16, cols: u16) {
        let _ = self.input_tx.send(PtyInput::Resize { rows, cols });
    }

    /// 关闭会话（发送 Ctrl+C + exit）
    pub fn close(&self) {
        let _ = self.input_tx.send(PtyInput::Data(vec![0x03]));
        let _ = self.input_tx.send(PtyInput::Data(b"exit\n".to_vec()));
    }
}

/// PTY 核心运行函数（在阻塞线程内执行）
fn run_pty_session(
    output_tx: broadcast::Sender<Vec<u8>>,
    mut input_rx: mpsc::UnboundedReceiver<PtyInput>,
    working_dir: Option<String>,
    cols: u16,
    rows: u16,
) {
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};

    let pty_system = native_pty_system();
    let pair = match pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("[PTY] openpty 失败: {}", e);
            let msg = format!("\r\n[错误] 无法创建 PTY: {}\r\n", e);
            let _ = output_tx.send(msg.into_bytes());
            return;
        }
    };

    let cli_path = match crate::services::claude_cli::get_claude_cli_path() {
        Some(p) => p,
        None => {
            tracing::error!("[PTY] 未找到 Claude Code CLI");
            let msg = "\r\n[错误] 未找到 Claude Code CLI，请先安装 claude (npm install -g @anthropic-ai/claude-code)\r\n".to_string();
            let _ = output_tx.send(msg.into_bytes());
            return;
        }
    };
    tracing::info!("[PTY] 使用 Claude CLI: {}", cli_path);

    let mut cmd = CommandBuilder::new(&cli_path);
    cmd.args(["--dangerously-skip-permissions"]);
    if let Some(ref dir) = working_dir {
        cmd.cwd(dir);
    }

    let _child = match pair.slave.spawn_command(cmd) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("[PTY] 启动 claude 失败: {} (path: {})", e, cli_path);
            let msg = format!("\r\n[错误] 无法启动 claude: {}\r\n", e);
            let _ = output_tx.send(msg.into_bytes());
            return;
        }
    };

    // PTY 读取线程：将终端输出广播给前端
    let mut reader = match pair.master.try_clone_reader() {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("[PTY] try_clone_reader 失败: {}", e);
            return;
        }
    };
    let output_tx_reader = output_tx.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    tracing::info!("[PTY] 读取 EOF，会话结束");
                    break;
                }
                Ok(n) => {
                    let data = buf[..n].to_vec();
                    let _ = output_tx_reader.send(data);
                }
                Err(e) => {
                    tracing::debug!("[PTY] 读取错误: {}", e);
                    break;
                }
            }
        }
    });

    // PTY 写入（键盘输入）
    let mut writer = match pair.master.take_writer() {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("[PTY] take_writer 失败: {}", e);
            return;
        }
    };

    // master 保留用于 resize
    let master = pair.master;

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("[PTY] Failed to build tokio runtime: {}", e);
            return;
        }
    };

    rt.block_on(async move {
        while let Some(cmd) = input_rx.recv().await {
            match cmd {
                PtyInput::Data(data) => {
                    if writer.write_all(&data).is_err() {
                        tracing::debug!("[PTY] 写入失败，会话可能已结束");
                        break;
                    }
                    let _ = writer.flush();
                }
                PtyInput::Resize { rows, cols } => {
                    let size = PtySize {
                        rows,
                        cols,
                        pixel_width: 0,
                        pixel_height: 0,
                    };
                    if let Err(e) = master.resize(size) {
                        tracing::warn!("[PTY] resize 失败: {}", e);
                    }
                }
            }
        }
    });
}

/// Claude 终端管理器（全局单例，管理所有团队的终端会话）
#[derive(Clone)]
pub struct ClaudeTerminalManager {
    sessions: Arc<RwLock<HashMap<String, Arc<ClaudeTerminalSession>>>>,
    /// Secondary index: (team_id, role_id) → session_id
    sessions_by_role: Arc<RwLock<HashMap<(String, String), String>>>,
}

impl ClaudeTerminalManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            sessions_by_role: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建新的终端会话，返回 session_id
    pub fn create_session(
        &self,
        team_id: &str,
        role_id: Option<&str>,
        working_dir: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> String {
        let session_id = Uuid::new_v4().to_string();
        let info = TerminalSessionInfo {
            session_id: session_id.clone(),
            team_id: team_id.to_string(),
            role_id: role_id.map(|s| s.to_string()),
            working_dir: working_dir.map(|s| s.to_string()),
        };
        let session = Arc::new(ClaudeTerminalSession::new(info, cols, rows));
        self.sessions.write().insert(session_id.clone(), session);

        // Maintain secondary index by (team_id, role_id)
        if let Some(rid) = role_id {
            self.sessions_by_role
                .write()
                .insert((team_id.to_string(), rid.to_string()), session_id.clone());
        }

        tracing::info!(
            "[ClaudeTerminal] 创建会话: {}, team: {}, size: {}x{}",
            session_id,
            team_id,
            cols,
            rows
        );
        session_id
    }

    /// 获取会话
    pub fn get_session(&self, session_id: &str) -> Option<Arc<ClaudeTerminalSession>> {
        self.sessions.read().get(session_id).cloned()
    }

    /// 向会话发送任务（后端逻辑处理完 prompt 后调用）
    pub fn dispatch_task(&self, session_id: &str, task: &str) -> bool {
        if let Some(session) = self.get_session(session_id) {
            session.dispatch_task(task);
            true
        } else {
            false
        }
    }

    /// 向会话发送原始输入（xterm.js 键盘事件）
    pub fn send_input(&self, session_id: &str, data: Vec<u8>) -> bool {
        if let Some(session) = self.get_session(session_id) {
            session.send_input(data);
            true
        } else {
            false
        }
    }

    /// 调整终端尺寸
    pub fn resize_session(&self, session_id: &str, rows: u16, cols: u16) {
        if let Some(session) = self.get_session(session_id) {
            session.resize(rows, cols);
        }
    }

    /// 关闭并移除会话
    pub fn close_session(&self, session_id: &str) {
        if let Some(session) = self.sessions.write().remove(session_id) {
            // Clean up secondary index
            let key = (
                session.info.team_id.clone(),
                session.info.role_id.clone().unwrap_or_default(),
            );
            self.sessions_by_role.write().remove(&key);
            session.close();
            tracing::info!("[ClaudeTerminal] 关闭会话: {}", session_id);
        }
    }

    /// 通过 (team_id, role_id) 查找会话
    pub fn get_session_by_role(
        &self,
        team_id: &str,
        role_id: &str,
    ) -> Option<Arc<ClaudeTerminalSession>> {
        let sessions_by_role = self.sessions_by_role.read();
        let session_id = sessions_by_role.get(&(team_id.to_string(), role_id.to_string()))?;
        self.sessions.read().get(session_id).cloned()
    }

    /// 获取或创建角色对应的 PTY 会话
    pub fn get_or_create_session(
        &self,
        team_id: &str,
        role_id: &str,
        working_dir: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> anyhow::Result<Arc<ClaudeTerminalSession>> {
        // Try to find existing session
        if let Some(session) = self.get_session_by_role(team_id, role_id) {
            return Ok(session);
        }
        // Create new one
        let session_id = self.create_session(team_id, Some(role_id), working_dir, cols, rows);
        self.sessions
            .read()
            .get(&session_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("session just created but not found: {}", session_id))
    }

    /// 列出团队所有会话
    pub fn list_sessions_for_team(&self, team_id: &str) -> Vec<Arc<ClaudeTerminalSession>> {
        self.sessions
            .read()
            .values()
            .filter(|s| s.info.team_id == team_id)
            .cloned()
            .collect()
    }

    /// 列出所有会话信息
    pub fn list_sessions(&self) -> Vec<TerminalSessionInfo> {
        self.sessions
            .read()
            .values()
            .map(|s| s.info.clone())
            .collect()
    }
}

impl Default for ClaudeTerminalManager {
    fn default() -> Self {
        Self::new()
    }
}
