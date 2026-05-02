
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use futures_util::{SinkExt, StreamExt};
use tauri::Emitter;
use tauri::Manager;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

/// Active PTY bridge connections: session_id → sender to WS sink task.
type PtyConnections = Arc<Mutex<HashMap<String, mpsc::Sender<WsMessage>>>>;

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
                    let _ = app_handle.emit(
                        &control_event,
                        r#"{"type":"closed"}"#.to_string(),
                    );
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
) -> Result<(), String> {
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
) -> Result<(), String> {
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

/// Disconnect a PTY session and drop the WS connection.
#[tauri::command]
async fn pty_disconnect(
    session_id: String,
    connections: tauri::State<'_, PtyConnections>,
) -> Result<(), String> {
    let mut conns = connections.lock().unwrap();
    conns.remove(&session_id);
    Ok(())
}

// ── App Entry Point ─────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let pty_connections: PtyConnections = Arc::new(Mutex::new(HashMap::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(pty_connections)
        .invoke_handler(tauri::generate_handler![
            pty_connect,
            pty_send_input,
            pty_send_control,
            pty_disconnect,
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Resolve resource directory via Tauri API (works cross-platform)
            let resource_dir = app.path().resource_dir()
                .expect("failed to resolve resource directory");

            // Resolve Claude CLI path and pass to nx_api as env var
            let claude_cli_env = if cfg!(debug_assertions) {
                // Debug: let nx_api find it from shell PATH
                None
            } else if cfg!(target_os = "windows") {
                // Windows: use 'where' command to locate claude (finds claude.cmd)
                let output = Command::new("cmd")
                    .args(["/c", "where claude 2>nul"])
                    .output();
                if let Ok(out) = output {
                    if out.status.success() {
                        let path = String::from_utf8_lossy(&out.stdout)
                            .lines()
                            .next()
                            .unwrap_or("")
                            .trim()
                            .to_string();
                        if !path.is_empty() {
                            Some(path)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                // macOS / Linux: resolve from login shell PATH
                let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
                let output = Command::new(&shell)
                    .args(["-l", "-c", "which claude 2>/dev/null || echo ''"])
                    .output();
                if let Ok(out) = output {
                    if out.status.success() {
                        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        if !path.is_empty() && path != "''" {
                            Some(path)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            let app_handle = app.handle().clone();
            // Start nx_api in background
            thread::spawn(move || {
                match start_nx_api(&resource_dir, claude_cli_env.as_deref()) {
                    Ok(()) => {}
                    Err(e) => {
                        let msg = format!("后台服务启动失败: {}\n请查看日志: %TEMP%/nx_startup.log", e);
                        write_startup_error(&msg);
                        // 尝试通知前端（app_handle 可能还不可用，忽略错误）
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

/// 查找 workspace root：包含 Cargo.toml（含 [workspace]）和 nx_dashboard/ 的目录
#[allow(dead_code)]
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

fn start_nx_api(resource_dir: &std::path::Path, claude_cli_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let _ = std::fs::write(std::env::temp_dir().join("nx_start_called.txt"), "start_nx_api called");

    // Kill any stale nx_api on port 8080 before starting
    kill_stale_nx_api();
    std::thread::sleep(std::time::Duration::from_millis(500));

    let (nx_api_path, skills_path, resources_dir) = if cfg!(debug_assertions) {
        let root = PathBuf::from("/Users/Zhuanz/Desktop/yp-nx-dashboard");
        let nx_api = root.join("target/debug/nx_api");
        let skills = root.join(".claude/agents");
        let resources = root.join("nx_dashboard");
        (nx_api, skills, resources)
    } else {
        // Release: use Tauri-resolved resource directory (cross-platform)
        let nx_api = if cfg!(target_os = "windows") {
            resource_dir.join("nx_api.exe")
        } else {
            resource_dir.join("nx_api")
        };
        let skills = resource_dir.join("skills");
        (nx_api, skills, resource_dir.to_path_buf())
    };

    if !nx_api_path.exists() {
        return Err(format!("nx_api not found at {:?}", nx_api_path).into());
    }

    let (_data_dir, db_path) = if cfg!(debug_assertions) {
        let dir = PathBuf::from("/Users/Zhuanz/Desktop/yp-nx-dashboard");
        let db = dir.join("nx_dashboard/nexus.db");
        (dir, db)
    } else {
        // 用户数据目录
        let app_data = dirs::data_dir()
            .unwrap_or_else(|| std::env::temp_dir())
            .join("com.nx.dashboard");
        std::fs::create_dir_all(&app_data)?;

        let db = app_data.join("nexus.db");

        // 首次启动：从 app bundle 内的模板复制数据库
        if !db.exists() {
            let template = resources_dir.join("nexus_template.db");

            if template.exists() {
                std::fs::copy(&template, &db)?;
                println!("[NX Dashboard] Copied template DB to {:?}", db);
            } else {
                eprintln!("[NX Dashboard] WARNING: template DB not found at {:?}, nx_api will create empty DB", template);
            }
        }

        (app_data.clone(), db)
    };

    let log_dir = std::env::temp_dir();
    std::fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join("nx_api.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true).write(true).truncate(true)
        .open(&log_path)
        .map_err(|e| format!("Failed to open log file: {}", e))?;
    let log_file2 = log_file.try_clone()?;

    let mut child_cmd = Command::new(&nx_api_path);
    child_cmd
        .env("AGENTS_DIR", &skills_path)
        .env("NEXUS_DB_PATH", &db_path)
        .env("NEXUS_ALLOWED_ORIGINS", "tauri://localhost,http://localhost:5173,http://localhost:3000")
        .env("RUST_LOG", "info");

    // Pass resolved Claude CLI path to nx_api
    if let Some(cli_path) = claude_cli_path {
        eprintln!("[NX Dashboard] Claude CLI path: {}", cli_path);
        child_cmd.env("CLAUDE_CLI_PATH_OVERRIDE", cli_path);
    }

    let mut child = child_cmd
        .stdout(log_file)
        .stderr(log_file2)
        .spawn()
        .map_err(|e| format!("Failed to spawn nx_api: {}", e))?;

    thread::sleep(std::time::Duration::from_secs(2));

    match child.try_wait() {
        Ok(Some(status)) => {
            let log = std::fs::read_to_string(&log_path).unwrap_or_default();
            return Err(format!("nx_api exited (status: {})\nlog: {}", status, log).into());
        }
        Ok(None) => {
            println!("[NX Dashboard] nx_api started (PID: {:?}), log: {:?}", child.id(), log_path);
        }
        Err(e) => {
            return Err(format!("Failed to check nx_api status: {}", e).into());
        }
    }

    // Keep child alive (and wait forever so nx_api is not orphaned on crash)
    let _ = child.wait();
    Ok(())
}
