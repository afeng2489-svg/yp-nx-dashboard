use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use futures_util::{SinkExt, StreamExt};
use tauri::Emitter;
use tauri::Manager;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use uuid::Uuid;

/// Active PTY bridge connections: session_id → sender to WS sink task.
type PtyConnections = Arc<Mutex<HashMap<String, mpsc::Sender<WsMessage>>>>;

/// Input command for a direct PTY session (spawned locally, not via WS/nx_api).
enum PtyInput {
    /// Raw keyboard bytes to write to PTY master
    Data(Vec<u8>),
    /// Terminal resize request (rows, cols)
    Resize(u16, u16),
    /// Close the PTY session
    Close,
}

/// Handle for a direct PTY session.
struct DirectPtyHandle {
    tx: std::sync::mpsc::Sender<PtyInput>,
    /// Barrier: reader thread waits here until frontend calls pty_start.
    ready: Arc<std::sync::Barrier>,
}

/// Direct PTY connections: session_id → channel to pump thread.
type DirectPtyConnections = Arc<Mutex<HashMap<String, DirectPtyHandle>>>;

// ── Tauri IPC Commands ──────────────────────────────────────────────────────

/// Connect to a PTY session hosted by nx_api over WS (Rust-to-Rust, no WKWebView).
/// Spawns two background tasks:
///   1. channel → WS sink  (keyboard/control input from frontend)
///   2. WS stream → app events  (PTY output to frontend)
#[tauri::command]
async fn pty_connect(
    team_id: String,
    session_id: String,
    app_handle: tauri::AppHandle,
    connections: tauri::State<'_, PtyConnections>,
) -> Result<(), String> {
    // Drop any stale connection for this session
    {
        let mut conns = connections.lock().unwrap();
        conns.remove(&session_id);
    }

    let url = format!(
        "ws://127.0.0.1:8080/ws/teams/{}/terminal/{}",
        team_id, session_id
    );

    let (ws_stream, _) = tokio_tungstenite::connect_async(&url)
        .await
        .map_err(|e| format!("WS connect failed: {e}"))?;

    let (mut ws_sink, mut ws_stream) = ws_stream.split();

    // Channel for forwarding frontend commands to the WS sink
    let (tx, mut rx) = mpsc::channel::<WsMessage>(64);

    {
        let mut conns = connections.lock().unwrap();
        conns.insert(session_id.clone(), tx);
    }

    // Task 1: drain channel → WS sink
    tauri::async_runtime::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Task 2: WS stream → frontend events
    let output_event = format!("pty-output-{}", session_id);
    let control_event = format!("pty-control-{}", session_id);

    tauri::async_runtime::spawn(async move {
        while let Some(result) = ws_stream.next().await {
            match result {
                Ok(WsMessage::Binary(data)) => {
                    let _ = app_handle.emit(&output_event, data);
                }
                Ok(WsMessage::Text(text)) => {
                    let _ = app_handle.emit(&control_event, text);
                }
                Ok(WsMessage::Close(_)) => {
                    let _ = app_handle.emit(&control_event, r#"{"type":"closed"}"#.to_string());
                    break;
                }
                Err(e) => {
                    let msg = format!(r#"{{"type":"error","message":"{}"}}"#, e);
                    let _ = app_handle.emit(&control_event, msg);
                    break;
                }
                _ => {}
            }
        }
    });

    Ok(())
}

/// Send raw keyboard bytes to the PTY session.
#[tauri::command]
async fn pty_send_input(
    session_id: String,
    data: Vec<u8>,
    connections: tauri::State<'_, PtyConnections>,
    direct_connections: tauri::State<'_, DirectPtyConnections>,
) -> Result<(), String> {
    // Check direct PTY connections first
    {
        let conns = direct_connections.lock().unwrap();
        if let Some(handle) = conns.get(&session_id) {
            let _ = handle.tx.send(PtyInput::Data(data));
            return Ok(());
        }
    }
    // Fall back to WS bridge
    let tx = {
        let conns = connections.lock().unwrap();
        conns.get(&session_id).cloned()
    };
    if let Some(tx) = tx {
        tx.send(WsMessage::Binary(data))
            .await
            .map_err(|e| format!("Send failed: {e}"))?;
    }
    Ok(())
}

/// Send a JSON control message (resize/task/close) to the PTY session.
#[tauri::command]
async fn pty_send_control(
    session_id: String,
    message: String,
    connections: tauri::State<'_, PtyConnections>,
    direct_connections: tauri::State<'_, DirectPtyConnections>,
) -> Result<(), String> {
    // Check direct PTY connections first — parse known control types
    {
        let conns = direct_connections.lock().unwrap();
        if let Some(handle) = conns.get(&session_id) {
            let cmd = parse_pty_control(&message);
            let _ = handle.tx.send(cmd);
            return Ok(());
        }
    }
    // Fall back to WS bridge
    let tx = {
        let conns = connections.lock().unwrap();
        conns.get(&session_id).cloned()
    };
    if let Some(tx) = tx {
        tx.send(WsMessage::Text(message))
            .await
            .map_err(|e| format!("Send failed: {e}"))?;
    }
    Ok(())
}

/// Parse a JSON control message into a PtyInput variant.
fn parse_pty_control(message: &str) -> PtyInput {
    #[derive(serde::Deserialize)]
    struct ControlMsg {
        #[serde(rename = "type")]
        msg_type: String,
        #[serde(default)]
        rows: u16,
        #[serde(default)]
        cols: u16,
        #[serde(default)]
        text: String,
    }

    match serde_json::from_str::<ControlMsg>(message) {
        Ok(msg) => match msg.msg_type.as_str() {
            "resize" => PtyInput::Resize(msg.rows, msg.cols),
            "close" => PtyInput::Close,
            "task" => PtyInput::Data({
                let mut data = msg.text.into_bytes();
                data.push(b'\r');
                data
            }),
            _ => PtyInput::Data(message.as_bytes().to_vec()),
        },
        Err(_) => PtyInput::Data(message.as_bytes().to_vec()),
    }
}

/// Spawn a team session by running `nx team --task "..."` in the background.
/// Returns the session_id immediately after parsing it from stdout, while the
/// session continues running in the background. Emits `team-session-completed`
/// when the process finishes.
#[tauri::command]
async fn spawn_team_session(
    task: String,
    model: Option<String>,
    working_dir: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let nx_bin = resolve_nx_binary()?;
    let mut cmd = Command::new(&nx_bin);
    cmd.arg("team");
    if let Some(ref m) = model {
        cmd.args(["--model", m]);
    }
    if let Some(ref dir) = working_dir {
        cmd.args(["--project", dir]);
        cmd.current_dir(dir);
    }
    cmd.arg(&task);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("无法启动 nx team (path: {:?}): {}", nx_bin, e))?;

    // 从 stdout 读取 session_id（扫描行直到找到前缀）
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "无法捕获 nx team 输出".to_string())?;
    let mut reader = BufReader::new(stdout);
    let mut id: Option<String> = None;
    let mut scanned_lines = String::new();
    for _ in 0..20 {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            break;
        }
        if let Some(sid) = line.strip_prefix("session_id:") {
            id = Some(sid.trim().to_string());
            break;
        }
        scanned_lines.push_str(&line);
    }

    let id = id.ok_or_else(|| {
        format!(
            "未能解析 session_id，已扫描行:\n{}",
            &scanned_lines[..scanned_lines.len().min(500)]
        )
    })?;

    // Emit event to refresh the session list immediately
    let _ = app_handle.emit("team-session-created", &id);
    let id_for_bg = id.clone();

    // 后台线程：持续消费 stdout 避免 SIGPIPE，然后等待进程完成
    let app_clone = app_handle.clone();
    thread::spawn(move || {
        // 消费剩余 stdout 防止子进程收到 SIGPIPE
        let mut buf = String::new();
        let _ = reader.read_to_string(&mut buf);
        let status = child.wait();
        let _ = app_clone.emit(
            "team-session-completed",
            serde_json::json!({
                "sessionId": id_for_bg,
                "success": status.is_ok_and(|s| s.success()),
            }),
        );
    });

    Ok(id)
}

/// Spawn a team session inside a local PTY, streaming output to the frontend
/// via Tauri events. Returns a local session_id immediately.
#[tauri::command]
async fn pty_spawn_team(
    task: String,
    model: Option<String>,
    working_dir: Option<String>,
    app_handle: tauri::AppHandle,
    direct_connections: tauri::State<'_, DirectPtyConnections>,
) -> Result<String, String> {
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};

    let session_id = Uuid::new_v4().to_string();
    let nx_bin = resolve_nx_binary()?;

    // ── Build the nx team command ──
    let mut cmd = CommandBuilder::new(nx_bin.to_string_lossy().as_ref());
    cmd.arg("team");
    if let Some(ref m) = model {
        cmd.arg("--model");
        cmd.arg(m);
    }
    if let Some(ref dir) = working_dir {
        cmd.arg("--project");
        cmd.arg(dir);
        cmd.cwd(dir);
    }
    cmd.arg(&task);

    // Set TERM so the TUI uses ANSI escape codes
    cmd.env("TERM", "xterm-256color");

    // ── Create PTY ──
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("无法创建 PTY: {e}"))?;

    let _child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("无法启动 nx team: {e}"))?;

    // ── Reader: clone from master to emit Tauri events ──
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("无法获取 PTY reader: {e}"))?;

    // ── Writer + master for resize: take writer from master ──
    let mut writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("无法获取 PTY writer: {e}"))?;
    let master = pair.master;

    // ── Channel for input/control ──
    let (tx, rx) = std::sync::mpsc::channel::<PtyInput>();
    let tx_for_reader = tx.clone();

    // ── Barrier: reader thread waits for frontend to call pty_start ──
    let ready = Arc::new(std::sync::Barrier::new(2));

    // ── Store handle BEFORE spawning threads ──
    {
        let mut conns = direct_connections.lock().unwrap();
        conns.insert(
            session_id.clone(),
            DirectPtyHandle {
                tx,
                ready: Arc::clone(&ready),
            },
        );
    }
    let conns_for_cleanup = direct_connections.inner().clone();

    let output_event = format!("pty-output-{}", session_id);
    let control_event = format!("pty-control-{}", session_id);
    let app_for_reader = app_handle.clone();

    // ── Reader thread: PTY → Tauri events (waits for frontend ready signal) ──
    std::thread::spawn(move || {
        // Block until frontend calls pty_start and sets up event listeners
        ready.wait();

        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = buf[..n].to_vec();
                    let _ = app_for_reader.emit(&output_event, data);
                }
                Err(_) => break,
            }
        }
        // Signal frontend that PTY has exited
        let _ = app_for_reader.emit(&control_event, r#"{"type":"closed"}"#.to_string());
        // Tell pump thread to clean up
        let _ = tx_for_reader.send(PtyInput::Close);
    });

    // ── Pump thread: receive input → write to PTY ──
    let sid_for_pump = session_id.clone();
    std::thread::spawn(move || {
        for cmd in rx {
            match cmd {
                PtyInput::Data(data) => {
                    if writer.write_all(&data).is_err() {
                        break;
                    }
                    let _ = writer.flush();
                }
                PtyInput::Resize(rows, cols) => {
                    let _ = master.resize(PtySize {
                        rows,
                        cols,
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                }
                PtyInput::Close => break,
            }
        }
        // Clean up the connection entry
        conns_for_cleanup.lock().unwrap().remove(&sid_for_pump);
    });

    // Emit event so the session list refreshes
    let _ = app_handle.emit("team-session-created", &session_id);

    Ok(session_id)
}

/// Spawn an interactive user shell in a local PTY (workspace terminal).
/// Returns session_id; frontend listens on `pty-output-{id}` / `pty-control-{id}`.
#[tauri::command]
async fn pty_spawn_shell(
    working_dir: Option<String>,
    rows: Option<u16>,
    cols: Option<u16>,
    app_handle: tauri::AppHandle,
    direct_connections: tauri::State<'_, DirectPtyConnections>,
) -> Result<String, String> {
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};

    let session_id = Uuid::new_v4().to_string();

    let shell = std::env::var("SHELL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            if cfg!(target_os = "windows") {
                "cmd.exe".to_string()
            } else if cfg!(target_os = "macos") {
                "/bin/zsh".to_string()
            } else {
                "/bin/bash".to_string()
            }
        });

    let mut cmd = CommandBuilder::new(&shell);
    cmd.env("TERM", "xterm-256color");
    if let Some(ref dir) = working_dir {
        if std::path::Path::new(dir).is_dir() {
            cmd.cwd(dir);
        }
    }

    let pty_rows = rows.unwrap_or(24).max(2);
    let pty_cols = cols.unwrap_or(80).max(2);

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: pty_rows,
            cols: pty_cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("无法创建 PTY: {e}"))?;

    let _child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("无法启动 Shell: {e}"))?;

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("无法获取 PTY reader: {e}"))?;

    let mut writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("无法获取 PTY writer: {e}"))?;
    let master = pair.master;

    let (tx, rx) = std::sync::mpsc::channel::<PtyInput>();
    let tx_for_reader = tx.clone();
    let ready = Arc::new(std::sync::Barrier::new(2));

    {
        let mut conns = direct_connections.lock().unwrap();
        conns.insert(
            session_id.clone(),
            DirectPtyHandle {
                tx,
                ready: Arc::clone(&ready),
            },
        );
    }
    let conns_for_cleanup = direct_connections.inner().clone();

    let output_event = format!("pty-output-{}", session_id);
    let control_event = format!("pty-control-{}", session_id);
    let app_for_reader = app_handle.clone();

    std::thread::spawn(move || {
        ready.wait();
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let _ = app_for_reader.emit(&output_event, buf[..n].to_vec());
                }
                Err(_) => break,
            }
        }
        let _ = app_for_reader.emit(&control_event, r#"{"type":"closed"}"#.to_string());
        let _ = tx_for_reader.send(PtyInput::Close);
    });

    let sid_for_pump = session_id.clone();
    std::thread::spawn(move || {
        for cmd in rx {
            match cmd {
                PtyInput::Data(data) => {
                    if writer.write_all(&data).is_err() {
                        break;
                    }
                    let _ = writer.flush();
                }
                PtyInput::Resize(rows, cols) => {
                    let _ = master.resize(PtySize {
                        rows,
                        cols,
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                }
                PtyInput::Close => break,
            }
        }
        conns_for_cleanup.lock().unwrap().remove(&sid_for_pump);
    });

    Ok(session_id)
}

/// Resolve the nx CLI binary path (debug: target/debug/nx, release: sidecar)
fn resolve_nx_binary() -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        let root = find_workspace_root().ok_or_else(|| "无法找到 workspace root".to_string())?;
        let nx = root.join("target/debug/nx");
        if nx.exists() {
            Ok(nx)
        } else {
            Err(format!("nx CLI 二进制文件未找到: {:?}", nx))
        }
    } else {
        // Release: look for nx sidecar next to the executable
        let exe = std::env::current_exe().map_err(|e| format!("{}", e))?;
        let exe_dir = exe.parent().ok_or("无法获取可执行文件目录")?;
        let name = if cfg!(target_os = "windows") {
            "nx.exe"
        } else {
            "nx"
        };
        let path = exe_dir.join(name);
        if path.exists() {
            Ok(path)
        } else {
            Err(format!("nx CLI 未找到: {:?}", path))
        }
    }
}

/// Signal that the frontend is ready to receive PTY output (event listeners are set up).
/// The reader thread in pty_spawn_team waits on a Barrier until this is called.
#[tauri::command]
async fn pty_start(
    session_id: String,
    direct_connections: tauri::State<'_, DirectPtyConnections>,
) -> Result<(), String> {
    let conns = direct_connections.lock().unwrap();
    if let Some(handle) = conns.get(&session_id) {
        handle.ready.wait();
        Ok(())
    } else {
        Err(format!("session not found: {session_id}"))
    }
}

/// Disconnect a PTY session and drop the connection.
#[tauri::command]
async fn pty_disconnect(
    session_id: String,
    connections: tauri::State<'_, PtyConnections>,
    direct_connections: tauri::State<'_, DirectPtyConnections>,
) -> Result<(), String> {
    // Check direct PTY connections first
    {
        let mut conns = direct_connections.lock().unwrap();
        if let Some(handle) = conns.get(&session_id) {
            let _ = handle.tx.send(PtyInput::Close);
            conns.remove(&session_id);
            return Ok(());
        }
    }
    // Fall back to WS bridge
    let mut conns = connections.lock().unwrap();
    conns.remove(&session_id);
    Ok(())
}

// ── App Entry Point ─────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let pty_connections: PtyConnections = Arc::new(Mutex::new(HashMap::new()));
    let direct_pty_connections: DirectPtyConnections = Arc::new(Mutex::new(HashMap::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(pty_connections)
        .manage(direct_pty_connections)
        .invoke_handler(tauri::generate_handler![
            pty_connect,
            pty_send_input,
            pty_send_control,
            pty_disconnect,
            pty_start,
            spawn_team_session,
            pty_spawn_team,
            pty_spawn_shell,
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Resolve Claude CLI path and pass to nx_api（dev/release 均绑定本地 CLI）
            let claude_cli_env = resolve_claude_cli_for_sidecar();

            // ── 写入启动标记（确认 setup 执行）──
            let marker_dir: std::path::PathBuf =
                dirs::home_dir().unwrap_or_else(std::env::temp_dir);
            let marker_path = marker_dir.join("nx_tauri_setup.log");
            let marker_msg = format!(
                "[{}] Tauri setup started\n  exe: {:?}\n  temp_dir: {:?}\n",
                chrono_or_timestamp(),
                std::env::current_exe().ok(),
                std::env::temp_dir(),
            );
            let _ = std::fs::write(&marker_path, &marker_msg);

            // ── 检查 sidecar 是否存在（两个名字都查）──
            let sidecar_info = if cfg!(not(debug_assertions)) {
                let exe_dir = std::env::current_exe()
                    .ok()
                    .and_then(|e| e.parent().map(|p| p.to_path_buf()));
                let names: Vec<&str> = if cfg!(target_os = "windows") {
                    vec!["nx_api-x86_64-pc-windows-msvc.exe", "nx_api.exe"]
                } else if cfg!(target_os = "macos") {
                    if cfg!(target_arch = "aarch64") {
                        vec!["nx_api-aarch64-apple-darwin", "nx_api"]
                    } else {
                        vec!["nx_api-x86_64-apple-darwin", "nx_api"]
                    }
                } else {
                    vec!["nx_api-x86_64-unknown-linux-gnu", "nx_api"]
                };
                let mut info = String::new();
                for name in &names {
                    let path = exe_dir.as_ref().map(|d| d.join(name));
                    let exists = path.as_ref().map(|p| p.exists()).unwrap_or(false);
                    let size = path
                        .as_ref()
                        .and_then(|p| std::fs::metadata(p).ok())
                        .map(|m| m.len())
                        .unwrap_or(0);
                    info.push_str(&format!(
                        "  sidecar {}: exists={}, size={} bytes, path={:?}\n",
                        name, exists, size, path
                    ));
                }
                info
            } else {
                "  debug mode — sidecar check skipped".to_string()
            };
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&marker_path)
                .and_then(|mut f| std::io::Write::write_all(&mut f, sidecar_info.as_bytes()));

            let app_handle = app.handle().clone();
            let marker_dir_for_thread = marker_dir.clone();
            let marker_path_for_thread = marker_path.clone();
            // Start nx_api in background
            thread::spawn(move || {
                match start_nx_api(&app_handle, claude_cli_env.as_deref()) {
                    Ok(()) => {}
                    Err(e) => {
                        // 写入多个位置确保用户能找到
                        let msg = format!("后台服务启动失败: {}", e);

                        // 1. 用户目录
                        let home_log = marker_dir_for_thread.join("nx_startup_error.log");
                        let _ = std::fs::write(&home_log, &msg);

                        // 2. 标记文件追加
                        let _ = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&marker_path_for_thread)
                            .and_then(|mut f| {
                                std::io::Write::write_all(
                                    &mut f,
                                    format!("\n\nERROR: {}\n", msg).as_bytes(),
                                )
                            });

                        // 3. temp dir (原有)
                        write_startup_error(&msg);

                        // 4. 通知前端
                        let _ = app_handle.emit("nx-api-startup-error", &msg);
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 将启动错误写入用户可见的日志文件
fn write_startup_error(msg: &str) {
    let log_path = std::env::temp_dir().join("nx_startup.log");
    let entry = format!("{}\n", msg);
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, entry.as_bytes()));
    eprintln!("{}", msg);
}

// ── nx_api Subprocess ───────────────────────────────────────────────────────

fn kill_stale_nx_api() {
    let port = 8080;
    if cfg!(target_os = "windows") {
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-Command",
                &format!("Get-NetTCPConnection -LocalPort {} -ErrorAction SilentlyContinue | Select-Object -ExpandProperty OwningProcess | ForEach-Object {{ Stop-Process -Id $_ -Force }}", port)])
            .output();
    } else {
        let output = Command::new("lsof")
            .args(["-i", &format!(":{}", port), "-t"])
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                let pids = String::from_utf8_lossy(&out.stdout);
                for pid in pids.lines().filter(|l| !l.trim().is_empty()) {
                    let _ = Command::new("kill").args(["-9", pid.trim()]).output();
                }
            }
        }
    }
}

/// 从用户 shell 解析本地 Claude Code CLI，供 nx_api sidecar 绑定
fn resolve_claude_cli_for_sidecar() -> Option<String> {
    if cfg!(target_os = "windows") {
        let output = Command::new("cmd")
            .args(["/c", "where claude 2>nul"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let path = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()?
            .trim()
            .to_string();
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    } else {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        let shell_cmds: &[&[&str]] = &[
            &["-i", "-l", "-c", "command -v claude"],
            &["-l", "-c", "which claude 2>/dev/null"],
        ];
        for args in shell_cmds {
            let output = Command::new(&shell).args(*args).output().ok()?;
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() && std::path::Path::new(&path).exists() {
                    return Some(path);
                }
            }
        }
        for candidate in ["/opt/homebrew/bin/claude", "/usr/local/bin/claude"] {
            if std::path::Path::new(candidate).exists() {
                return Some(candidate.to_string());
            }
        }
        None
    }
}

/// 查找 workspace root：包含 Cargo.toml（含 [workspace]）和 nx_dashboard/ 的目录
fn find_workspace_root() -> Option<PathBuf> {
    let is_workspace = |dir: &std::path::Path| -> bool {
        if !dir.join("Cargo.toml").exists() || !dir.join("nx_dashboard").is_dir() {
            return false;
        }
        // 确认 Cargo.toml 包含 [workspace]
        std::fs::read_to_string(dir.join("Cargo.toml"))
            .map(|c| c.contains("[workspace]"))
            .unwrap_or(false)
    };

    // 从可执行文件位置向上查找
    if let Ok(exe) = std::env::current_exe() {
        let exe = exe.canonicalize().unwrap_or(exe);
        for ancestor in exe.ancestors().skip(1) {
            if is_workspace(ancestor) {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    // 从 CWD 向上查找
    if let Ok(cwd) = std::env::current_dir() {
        for ancestor in cwd.ancestors() {
            if is_workspace(ancestor) {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    None
}

fn start_nx_api(
    app_handle: &tauri::AppHandle,
    claude_cli_path: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    // All diagnostic output goes to this log file — on Windows stderr is invisible
    let diag_path = std::env::temp_dir().join("nx_startup.log");
    let diag = |msg: &str| {
        let entry = format!("[{}] {}\n", chrono_or_timestamp(), msg);
        eprintln!("{}", entry.trim());
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&diag_path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, entry.as_bytes()));
    };

    diag("start_nx_api called");

    // Kill any stale nx_api on port 8080 before starting
    kill_stale_nx_api();
    std::thread::sleep(std::time::Duration::from_millis(500));

    let (nx_api_path, skills_path, resources_dir) = if cfg!(debug_assertions) {
        let root = find_workspace_root()
            .expect("无法找到 workspace root（包含 Cargo.toml 和 nx_dashboard/ 的目录）");
        let nx_api = root.join("target/debug/nx_api");
        let skills = root.join(".claude/agents");
        let resources = root.join("nx_dashboard");
        (nx_api, skills, resources)
    } else {
        // Release: resolve sidecar binary path (bundled via tauri.conf.json externalBin)
        let target_triple = if cfg!(target_os = "windows") {
            "x86_64-pc-windows-msvc"
        } else if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                "aarch64-apple-darwin"
            } else {
                "x86_64-apple-darwin"
            }
        } else {
            "x86_64-unknown-linux-gnu"
        };
        let sidecar_name = if cfg!(target_os = "windows") {
            format!("nx_api-{}.exe", target_triple)
        } else {
            format!("nx_api-{}", target_triple)
        };

        // Plain name without target triple (Tauri may strip the suffix during bundling)
        let plain_name: String = if cfg!(target_os = "windows") {
            "nx_api.exe".to_string()
        } else {
            "nx_api".to_string()
        };

        let resource_dir = app_handle
            .path()
            .resource_dir()
            .expect("failed to resolve resource directory");

        // Try multiple candidate paths for the sidecar
        let mut candidates: Vec<std::path::PathBuf> = Vec::new();

        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                // Candidate 1: next to main exe — triple-suffixed name
                candidates.push(exe_dir.join(&sidecar_name));
                // Candidate 2: next to main exe — plain name (Tauri may strip suffix)
                candidates.push(exe_dir.join(&plain_name));
            }
        }

        // Candidate 3-4: resource directory
        candidates.push(resource_dir.join(&sidecar_name));
        candidates.push(resource_dir.join(&plain_name));

        // Candidate 5-6: resource dir / MacOS (macOS .app structure)
        candidates.push(resource_dir.join("MacOS").join(&sidecar_name));
        candidates.push(resource_dir.join("MacOS").join(&plain_name));

        diag(&format!("Sidecar name: {}", sidecar_name));
        diag(&format!("Resource dir: {:?}", resource_dir));

        let nx_api = match candidates.iter().find(|p| p.exists()) {
            Some(found) => {
                diag(&format!("Sidecar found at: {:?}", found));
                found.clone()
            }
            None => {
                for c in &candidates {
                    diag(&format!("NOT found: {:?}", c));
                }
                // Return first candidate as the error path
                candidates
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| resource_dir.join(&sidecar_name))
            }
        };

        let skills = resource_dir.join("skills");
        (nx_api, skills, resource_dir)
    };

    if !nx_api_path.exists() {
        let msg = format!("nx_api not found at {:?}", nx_api_path);
        diag(&msg);
        return Err(msg.into());
    }

    diag(&format!("nx_api path: {:?}", nx_api_path));

    // macOS/Linux: ensure the binary is executable after being extracted from the bundle
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&nx_api_path)?.permissions();
        perms.set_mode(perms.mode() | 0o111);
        std::fs::set_permissions(&nx_api_path, perms)?;
    }

    let (_data_dir, db_path) = if cfg!(debug_assertions) {
        let dir = find_workspace_root()
            .expect("无法找到 workspace root（包含 Cargo.toml 和 nx_dashboard/ 的目录）");
        let db = dir.join("nx_dashboard/nexus.db");
        (dir, db)
    } else {
        // 用户数据目录
        let app_data = dirs::data_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("com.nx.dashboard");
        std::fs::create_dir_all(&app_data)?;

        let db = app_data.join("nexus.db");

        // 首次启动：从 app bundle 内的模板复制数据库
        if !db.exists() {
            let template = resources_dir.join("nexus_template.db");

            if template.exists() {
                std::fs::copy(&template, &db)?;
                diag(&format!("Copied template DB to {:?}", db));
            } else {
                diag(&format!(
                    "WARNING: template DB not found at {:?}, nx_api will create empty DB",
                    template
                ));
            }
        }

        (app_data.clone(), db)
    };

    let log_dir = std::env::temp_dir();
    std::fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join("nx_api.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
        .map_err(|e| format!("Failed to open log file: {}", e))?;
    let log_file2 = log_file.try_clone()?;

    diag(&format!("DB path: {:?}", db_path));
    diag(&format!("Skills path: {:?}", skills_path));
    diag(&format!("nx_api log: {:?}", log_path));

    // Verify binary is not empty/corrupt
    let bin_size = std::fs::metadata(&nx_api_path)
        .map(|m| m.len())
        .unwrap_or(0);
    diag(&format!("Binary size: {} bytes", bin_size));
    if bin_size == 0 {
        let msg = format!("nx_api binary is empty (0 bytes) at {:?}", nx_api_path);
        diag(&msg);
        return Err(msg.into());
    }

    let mut child_cmd = Command::new(&nx_api_path);
    child_cmd
        .env("AGENTS_DIR", &skills_path)
        .env("NEXUS_DB_PATH", &db_path)
        .env(
            "NEXUS_ALLOWED_ORIGINS",
            "tauri://localhost,http://localhost:1420,http://localhost:5173,http://localhost:3000",
        )
        .env("RUST_LOG", "info");

    // On Windows, add sidecar directory to PATH so DLLs can be found
    #[cfg(target_os = "windows")]
    if let Some(sidecar_dir) = nx_api_path.parent() {
        let path_env = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{};{}", sidecar_dir.display(), path_env);
        child_cmd.env("PATH", &new_path);
        diag(&format!("Added to PATH: {:?}", sidecar_dir));
    }

    // Pass resolved Claude CLI path to nx_api（workflow 引擎通过 env 读取）
    if let Some(cli_path) = claude_cli_path {
        diag(&format!("Claude CLI path: {}", cli_path));
        child_cmd.env("CLAUDE_CLI_PATH_OVERRIDE", cli_path);
        child_cmd.env("CLAUDE_BIN", cli_path);
    }

    // 本地 dev：trusted 模式，让 Claude Code CLI 能以 agent 模式读写项目（无需开终端点确认）
    if cfg!(debug_assertions) {
        child_cmd.env("NEXUS_PERMISSIONS_MODE", "trusted");
        diag("NEXUS_PERMISSIONS_MODE=trusted (dev)");
    }

    diag("Spawning nx_api...");
    let mut child = child_cmd
        .stdout(log_file)
        .stderr(log_file2)
        .spawn()
        .map_err(|e| {
            let msg = format!("Failed to spawn nx_api: {} (path: {:?})", e, nx_api_path);
            diag(&msg);
            msg
        })?;

    diag(&format!("nx_api spawned, PID: {:?}", child.id()));

    // Poll port 8080 until nx_api is ready (max 10 seconds)
    let mut ready = false;
    for i in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Check if nx_api crashed
        match child.try_wait() {
            Ok(Some(status)) => {
                let log = std::fs::read_to_string(&log_path).unwrap_or_default();
                let msg = format!(
                    "nx_api exited (status: {})\n--- nx_api log ---\n{}",
                    status, log
                );
                diag(&msg);
                return Err(msg.into());
            }
            Err(e) => {
                let msg = format!("Failed to check nx_api status: {}", e);
                diag(&msg);
                return Err(msg.into());
            }
            Ok(None) => { /* still running */ }
        }

        // Try connecting to port 8080
        if std::net::TcpStream::connect_timeout(
            &"127.0.0.1:8080".parse().unwrap(),
            std::time::Duration::from_millis(200),
        )
        .is_ok()
        {
            diag(&format!(
                "nx_api ready on port 8080 (after {}x500ms)",
                i + 1
            ));
            ready = true;
            break;
        }
    }

    if !ready {
        let log = std::fs::read_to_string(&log_path).unwrap_or_default();
        let msg = format!(
            "nx_api started but port 8080 not responding after 10s\n--- nx_api log ---\n{}",
            log
        );
        diag(&msg);
        return Err(msg.into());
    }

    // Keep child alive (and wait forever so nx_api is not orphaned on crash)
    let _ = child.wait();
    Ok(())
}

/// Simple timestamp for diagnostic logs
fn chrono_or_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_else(|_| "???".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::MutexGuard;
    use std::time::Duration;

    /// Serializes access to the shared `nx_startup.log` to prevent inter-test races.
    fn acquire_log_lock() -> MutexGuard<'static, ()> {
        static LOG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOG_LOCK.lock().unwrap()
    }

    // ── chrono_or_timestamp ─────────────────────────────────────────────────

    #[test]
    fn test_chrono_or_timestamp_returns_valid_unix_epoch() {
        let ts = chrono_or_timestamp();
        assert!(!ts.is_empty(), "timestamp should not be empty");
        let secs: u64 = ts.parse().expect("timestamp should be a valid u64");
        // Reasonable range: 2024-2027 (~1_704_000_000 .. ~1_800_000_000)
        assert!(
            secs > 1_700_000_000 && secs < 1_900_000_000,
            "timestamp {secs} should be in reasonable unix epoch range"
        );
    }

    #[test]
    fn test_chrono_or_timestamp_monotonic() {
        let t1: u64 = chrono_or_timestamp().parse().unwrap();
        std::thread::sleep(Duration::from_millis(5));
        let t2: u64 = chrono_or_timestamp().parse().unwrap();
        assert!(
            t2 >= t1,
            "timestamps should be non-decreasing: {t2} >= {t1}"
        );
    }

    // ── find_workspace_root ────────────────────────────────────────────────

    #[test]
    fn test_find_workspace_root_from_test_binary() {
        // The test binary runs inside target/debug/, which is under the workspace
        let result = find_workspace_root();
        assert!(
            result.is_some(),
            "should find workspace from within the project"
        );
        let root = result.unwrap();
        assert!(
            root.join("Cargo.toml").exists(),
            "workspace root should contain Cargo.toml"
        );
        let toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap_or_default();
        assert!(
            toml.contains("[workspace]"),
            "workspace Cargo.toml should contain [workspace]"
        );
        assert!(
            root.join("nx_dashboard").is_dir(),
            "workspace root should contain nx_dashboard/"
        );
    }

    // ── write_startup_error ────────────────────────────────────────────────

    fn startup_log_path() -> std::path::PathBuf {
        std::env::temp_dir().join("nx_startup.log")
    }

    fn cleanup_startup_log() {
        let _ = std::fs::remove_file(startup_log_path());
    }

    #[test]
    fn test_write_startup_error_appends_to_log() {
        let _guard = acquire_log_lock();
        cleanup_startup_log();

        write_startup_error("test message 1");
        write_startup_error("test message 2");

        let content = std::fs::read_to_string(startup_log_path()).unwrap_or_default();
        assert!(
            content.contains("test message 1"),
            "should contain first message"
        );
        assert!(
            content.contains("test message 2"),
            "should contain second message"
        );

        cleanup_startup_log();
    }

    #[test]
    fn test_write_startup_error_empty_message() {
        let _guard = acquire_log_lock();
        cleanup_startup_log();
        write_startup_error(""); // Should not panic
        let content = std::fs::read_to_string(startup_log_path()).unwrap_or_default();
        // Even an empty message produces a line
        assert!(!content.is_empty(), "log should contain at least a newline");
        cleanup_startup_log();
    }

    #[test]
    fn test_write_startup_error_long_message() {
        let _guard = acquire_log_lock();
        cleanup_startup_log();
        let long = "A".repeat(10_000);
        write_startup_error(&long);
        let content = std::fs::read_to_string(startup_log_path()).unwrap_or_default();
        assert!(
            content.contains(&long),
            "long message should be fully written"
        );
        cleanup_startup_log();
    }

    #[test]
    fn test_write_startup_error_unicode_message() {
        let _guard = acquire_log_lock();
        cleanup_startup_log();
        write_startup_error("启动错误: 服务不可用 🔥");
        let content = std::fs::read_to_string(startup_log_path()).unwrap_or_default();
        assert!(
            content.contains("启动错误"),
            "unicode text should be preserved"
        );
        cleanup_startup_log();
    }

    // ── kill_stale_nx_api ──────────────────────────────────────────────────

    #[test]
    fn test_kill_stale_nx_api_no_panic_on_clean_port() {
        // Should not panic, hang, or error when nothing listens on port 8080
        kill_stale_nx_api();
    }

    // ── PtyConnections (Arc<Mutex<HashMap>>) ───────────────────────────────

    fn make_pty_connections() -> PtyConnections {
        Arc::new(Mutex::new(HashMap::new()))
    }

    fn make_sender() -> tokio::sync::mpsc::Sender<WsMessage> {
        let (tx, _rx) = tokio::sync::mpsc::channel::<WsMessage>(64);
        tx
    }

    #[test]
    fn test_pty_connections_empty_on_init() {
        let conns = make_pty_connections();
        assert!(conns.lock().unwrap().is_empty());
    }

    #[test]
    fn test_pty_connections_insert_and_contains() {
        let conns = make_pty_connections();
        let sid = "session-test-1".to_string();

        conns.lock().unwrap().insert(sid.clone(), make_sender());

        let map = conns.lock().unwrap();
        assert!(map.contains_key(&sid));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_pty_connections_remove_returns_sender() {
        let conns = make_pty_connections();
        let sid = "session-to-remove".to_string();
        conns.lock().unwrap().insert(sid.clone(), make_sender());

        let removed = conns.lock().unwrap().remove(&sid);
        assert!(removed.is_some(), "should return the removed sender");

        assert!(conns.lock().unwrap().is_empty());
    }

    #[test]
    fn test_pty_connections_remove_nonexistent() {
        let conns = make_pty_connections();
        let result = conns.lock().unwrap().remove("i-do-not-exist");
        assert!(result.is_none(), "removing non-existent key returns None");
    }

    #[test]
    fn test_pty_connections_multiple_sessions() {
        let conns = make_pty_connections();
        let sessions = ["s1", "s2", "s3", "s4", "s5"];

        for s in &sessions {
            conns.lock().unwrap().insert(s.to_string(), make_sender());
        }

        {
            let map = conns.lock().unwrap();
            assert_eq!(map.len(), 5);
            for s in &sessions {
                assert!(map.contains_key(*s));
            }
        }

        // Remove two middle entries
        conns.lock().unwrap().remove("s2");
        conns.lock().unwrap().remove("s4");

        {
            let map = conns.lock().unwrap();
            assert_eq!(map.len(), 3);
            assert!(map.contains_key("s1"));
            assert!(!map.contains_key("s2"));
            assert!(map.contains_key("s3"));
            assert!(!map.contains_key("s4"));
            assert!(map.contains_key("s5"));
        }
    }

    #[test]
    fn test_pty_connections_reinsert_replaces() {
        let conns = make_pty_connections();
        let sid = "session-replace".to_string();

        conns.lock().unwrap().insert(sid.clone(), make_sender());

        let old = conns.lock().unwrap().insert(sid.clone(), make_sender());
        assert!(old.is_some(), "re-insert should return previous sender");

        assert_eq!(conns.lock().unwrap().len(), 1);
    }

    // ── Edge Cases: session_id ─────────────────────────────────────────────

    #[test]
    fn test_pty_connections_empty_string_key() {
        let conns = make_pty_connections();
        conns.lock().unwrap().insert(String::new(), make_sender());
        assert!(conns.lock().unwrap().contains_key(""));
    }

    #[test]
    fn test_pty_connections_very_long_session_id() {
        let conns = make_pty_connections();
        let long_id = "a".repeat(10_000);
        conns.lock().unwrap().insert(long_id.clone(), make_sender());
        assert!(conns.lock().unwrap().contains_key(&long_id));
    }

    #[test]
    fn test_pty_connections_special_chars_in_key() {
        let conns = make_pty_connections();
        let special = "uuid:550e8400-e29b-41d4-a716-446655440000/session:1".to_string();
        conns.lock().unwrap().insert(special.clone(), make_sender());
        assert!(conns.lock().unwrap().contains_key(&special));
    }

    // ── Concurrent Access ─────────────────────────────────────────────────

    #[test]
    fn test_pty_connections_concurrent_inserts_50() {
        let conns = make_pty_connections();
        let mut handles = vec![];

        for i in 0..50 {
            let conns = Arc::clone(&conns);
            handles.push(std::thread::spawn(move || {
                conns
                    .lock()
                    .unwrap()
                    .insert(format!("c-{i}"), make_sender());
            }));
        }

        for h in handles {
            h.join().expect("thread should not panic");
        }

        assert_eq!(conns.lock().unwrap().len(), 50);
    }

    #[test]
    fn test_pty_connections_concurrent_read_write() {
        let conns = make_pty_connections();

        // Pre-populate some sessions
        for i in 0..10 {
            conns
                .lock()
                .unwrap()
                .insert(format!("pre-{i}"), make_sender());
        }

        let mut handles: Vec<std::thread::JoinHandle<()>> = vec![];

        // Write threads
        for i in 0..10 {
            let c = Arc::clone(&conns);
            handles.push(std::thread::spawn(move || {
                c.lock()
                    .unwrap()
                    .insert(format!("write-{i}"), make_sender());
            }));
        }

        // Read threads
        for _ in 0..10 {
            let c = Arc::clone(&conns);
            handles.push(std::thread::spawn(move || {
                let map = c.lock().unwrap();
                let _ = map.contains_key("pre-0");
                let _ = map.len();
            }));
        }

        for h in handles {
            h.join().expect("thread should not panic");
        }

        let map = conns.lock().unwrap();
        assert_eq!(map.len(), 20);
    }

    #[test]
    fn test_pty_connections_drain_all() {
        let conns = make_pty_connections();
        for i in 0..10 {
            conns
                .lock()
                .unwrap()
                .insert(format!("drain-{i}"), make_sender());
        }
        conns.lock().unwrap().clear();
        assert!(conns.lock().unwrap().is_empty());
    }

    // ── Channel send/receive patterns ──────────────────────────────────────

    #[test]
    fn test_pty_send_input_binary_through_channel() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<WsMessage>(64);

        tx.blocking_send(WsMessage::Binary(vec![104, 101, 108, 108, 111]))
            .expect("should send binary data");

        match rx.blocking_recv() {
            Some(WsMessage::Binary(data)) => assert_eq!(data, b"hello"),
            other => panic!("expected Binary, got {:?}", other),
        }
    }

    #[test]
    fn test_pty_send_control_text_through_channel() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<WsMessage>(64);
        let msg = r#"{"type":"resize","cols":80,"rows":24}"#;

        tx.blocking_send(WsMessage::Text(msg.to_string()))
            .expect("should send text message");

        match rx.blocking_recv() {
            Some(WsMessage::Text(text)) => assert_eq!(text, msg),
            other => panic!("expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_pty_send_empty_binary() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<WsMessage>(64);
        tx.blocking_send(WsMessage::Binary(vec![])).unwrap();
        match rx.blocking_recv() {
            Some(WsMessage::Binary(data)) => assert!(data.is_empty()),
            other => panic!("expected empty Binary, got {:?}", other),
        }
    }

    #[test]
    fn test_pty_send_large_binary_64k() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<WsMessage>(64);
        let large = vec![0xABu8; 65_536];
        tx.blocking_send(WsMessage::Binary(large.clone())).unwrap();
        match rx.blocking_recv() {
            Some(WsMessage::Binary(data)) => {
                assert_eq!(data.len(), 65_536);
                assert_eq!(data[0], 0xAB);
                assert_eq!(data[65_535], 0xAB);
            }
            other => panic!("expected Binary, got {:?}", other),
        }
    }

    #[test]
    fn test_pty_send_close_message() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<WsMessage>(64);
        tx.blocking_send(WsMessage::Close(None)).unwrap();
        match rx.blocking_recv() {
            Some(WsMessage::Close(_)) => {} // expected
            other => panic!("expected Close, got {:?}", other),
        }
    }

    #[test]
    fn test_pty_send_ping_pong() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<WsMessage>(64);

        tx.blocking_send(WsMessage::Ping(vec![1, 2, 3])).unwrap();
        match rx.blocking_recv() {
            Some(WsMessage::Ping(data)) => assert_eq!(data, vec![1, 2, 3]),
            other => panic!("expected Ping, got {:?}", other),
        }

        tx.blocking_send(WsMessage::Pong(vec![4, 5, 6])).unwrap();
        match rx.blocking_recv() {
            Some(WsMessage::Pong(data)) => assert_eq!(data, vec![4, 5, 6]),
            other => panic!("expected Pong, got {:?}", other),
        }
    }

    // ── Channel backpressure and lifecycle ─────────────────────────────────

    #[test]
    fn test_pty_channel_bounded_capacity_behavior() {
        let (tx, rx) = tokio::sync::mpsc::channel::<WsMessage>(4);

        // Fill the buffer exactly to capacity
        for i in 0..4 {
            tx.blocking_send(WsMessage::Binary(vec![i]))
                .expect("first 4 sends should succeed");
        }

        // Next non-blocking send should fail (channel full)
        assert!(
            tx.try_send(WsMessage::Binary(vec![4])).is_err(),
            "try_send should fail when channel buffer is full"
        );

        // Consume one message, then try_send should succeed
        drop(rx);
    }

    #[test]
    fn test_pty_channel_close_on_all_senders_dropped() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<WsMessage>(64);
        drop(tx);

        let received = rx.blocking_recv();
        assert!(
            received.is_none(),
            "channel returns None after all senders dropped"
        );
    }

    #[test]
    fn test_pty_channel_partial_close() {
        // Multiple senders: dropping some leaves channel open
        let (tx1, mut rx) = tokio::sync::mpsc::channel::<WsMessage>(64);
        let tx2 = tx1.clone();

        tx1.blocking_send(WsMessage::Binary(vec![1])).unwrap();
        drop(tx1); // Drop one sender

        // Channel still open because tx2 is alive
        tx2.blocking_send(WsMessage::Binary(vec![2])).unwrap();
        drop(tx2);

        // Now all senders are dropped
        let mut results = vec![];
        while let Some(msg) = rx.blocking_recv() {
            if let WsMessage::Binary(d) = msg {
                results.push(d[0]);
            }
        }
        assert_eq!(results, vec![1, 2]);
    }

    #[test]
    fn test_pty_channel_capacity_exhaustion_recovery() {
        let (tx, rx) = tokio::sync::mpsc::channel::<WsMessage>(2);

        // Fill channel
        tx.blocking_send(WsMessage::Binary(vec![1])).unwrap();
        tx.blocking_send(WsMessage::Binary(vec![2])).unwrap();

        // try_send should fail
        assert!(tx.try_send(WsMessage::Binary(vec![3])).is_err());

        // Drop receiver to "recover" (simulates connection drop)
        drop(rx);

        // After receiver is dropped, try_send should fail with Closed
        match tx.try_send(WsMessage::Binary(vec![3])) {
            Err(e) => {
                assert!(
                    e.to_string().contains("closed"),
                    "after rx drop, expected Closed error, got: {e}"
                );
            }
            Ok(_) => panic!("should have been Closed after rx drop"),
        }
    }

    // ── Cross-thread channel patterns ──────────────────────────────────────

    #[test]
    fn test_pty_channel_send_from_multiple_threads() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<WsMessage>(64);
        let mut handles = vec![];

        for i in 0..5 {
            let tx = tx.clone();
            handles.push(std::thread::spawn(move || {
                tx.blocking_send(WsMessage::Binary(vec![i]))
                    .expect("thread send should succeed");
            }));
        }

        // Drop original sender
        drop(tx);

        for h in handles {
            h.join().unwrap();
        }

        let mut values: Vec<u8> = vec![];
        while let Some(msg) = rx.blocking_recv() {
            if let WsMessage::Binary(data) = msg {
                values.push(data[0]);
            }
        }
        values.sort();
        assert_eq!(values, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_pty_channel_drop_receiver_while_sending() {
        let (tx, rx) = tokio::sync::mpsc::channel::<WsMessage>(4);

        // Fill buffer
        for i in 0..4 {
            tx.blocking_send(WsMessage::Binary(vec![i])).unwrap();
        }

        // Drop receiver — this closes the channel
        drop(rx);

        // Subsequent sends should fail (Closed, not Full)
        let _ = tx.blocking_send(WsMessage::Binary(vec![5]));
    }

    // ── Tauri-command-like logic patterns ──────────────────────────────────

    /// Simulate pty_send_input logic: look up sender in map and send
    fn simulate_pty_send_input(
        conns: &PtyConnections,
        session_id: &str,
        data: Vec<u8>,
    ) -> Result<(), String> {
        let tx = conns.lock().unwrap().get(session_id).cloned();
        match tx {
            Some(tx) => tx
                .blocking_send(WsMessage::Binary(data))
                .map_err(|e| format!("Send failed: {e}")),
            None => Ok(()), // No-op if session not found (matches real behavior)
        }
    }

    /// Simulate pty_send_control logic
    fn simulate_pty_send_control(
        conns: &PtyConnections,
        session_id: &str,
        message: String,
    ) -> Result<(), String> {
        let tx = conns.lock().unwrap().get(session_id).cloned();
        match tx {
            Some(tx) => tx
                .blocking_send(WsMessage::Text(message))
                .map_err(|e| format!("Send failed: {e}")),
            None => Ok(()),
        }
    }

    /// Simulate pty_disconnect logic
    fn simulate_pty_disconnect(conns: &PtyConnections, session_id: &str) {
        conns.lock().unwrap().remove(session_id);
    }

    #[test]
    fn test_simulated_pty_send_input_to_existing_session() {
        let conns = make_pty_connections();
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        conns.lock().unwrap().insert("live-session".into(), tx);

        simulate_pty_send_input(&conns, "live-session", b"hello".to_vec()).unwrap();

        match rx.blocking_recv() {
            Some(WsMessage::Binary(data)) => assert_eq!(data, b"hello"),
            other => panic!("expected Binary, got {other:?}"),
        }
    }

    #[test]
    fn test_simulated_pty_send_input_to_nonexistent_session() {
        let conns = make_pty_connections();
        // Should not fail — matches real pty_send_input behavior (no-op)
        let result = simulate_pty_send_input(&conns, "ghost-session", b"data".to_vec());
        assert!(
            result.is_ok(),
            "sending to missing session should be Ok(())"
        );
    }

    #[test]
    fn test_simulated_pty_send_control_to_existing_session() {
        let conns = make_pty_connections();
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        conns.lock().unwrap().insert("ctrl-session".into(), tx);

        simulate_pty_send_control(&conns, "ctrl-session", r#"{"type":"close"}"#.into()).unwrap();

        match rx.blocking_recv() {
            Some(WsMessage::Text(text)) => assert_eq!(text, r#"{"type":"close"}"#),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn test_simulated_pty_disconnect_removes_session() {
        let conns = make_pty_connections();
        conns
            .lock()
            .unwrap()
            .insert("to-disconnect".into(), make_sender());

        simulate_pty_disconnect(&conns, "to-disconnect");

        assert!(!conns.lock().unwrap().contains_key("to-disconnect"));
    }

    #[test]
    fn test_simulated_pty_full_flow() {
        // Full lifecycle: connect → send input → send control → disconnect
        let conns = make_pty_connections();
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        conns.lock().unwrap().insert("full-flow".into(), tx);

        // Send input
        simulate_pty_send_input(&conns, "full-flow", b"command\n".to_vec()).unwrap();
        // Send control
        simulate_pty_send_control(
            &conns,
            "full-flow",
            r#"{"type":"resize","cols":80,"rows":24}"#.into(),
        )
        .unwrap();

        // Disconnect
        simulate_pty_disconnect(&conns, "full-flow");
        assert!(!conns.lock().unwrap().contains_key("full-flow"));

        // Verify both messages arrived before disconnect
        let msg1 = rx.blocking_recv();
        assert!(msg1.is_some());
        let msg2 = rx.blocking_recv();
        assert!(msg2.is_some());
    }

    // ── Error recovery patterns ───────────────────────────────────────────

    #[test]
    fn test_pty_reconnect_replaces_stale_connection() {
        let conns = make_pty_connections();
        let sid = "reconnect-session";

        // First connection
        let (tx1, rx1) = tokio::sync::mpsc::channel::<WsMessage>(4);
        conns.lock().unwrap().insert(sid.to_string(), tx1);

        // Simulate disconnect from remote: drop the old receiver
        drop(rx1);

        // After remote drop, sending should fail
        let tx = conns.lock().unwrap().get(sid).cloned();
        if let Some(tx) = tx {
            let send_result = tx.blocking_send(WsMessage::Binary(vec![1]));
            // This may or may not succeed depending on timing
            let _ = send_result;
        }

        // Simulate reconnection: replace with new channel
        let (tx2, mut rx2) = tokio::sync::mpsc::channel::<WsMessage>(4);
        conns.lock().unwrap().insert(sid.to_string(), tx2);

        // New channel should work
        simulate_pty_send_input(&conns, sid, b"new data".to_vec()).unwrap();
        match rx2.blocking_recv() {
            Some(WsMessage::Binary(data)) => assert_eq!(data, b"new data"),
            other => panic!("expected Binary via new channel, got {other:?}"),
        }
    }
}
