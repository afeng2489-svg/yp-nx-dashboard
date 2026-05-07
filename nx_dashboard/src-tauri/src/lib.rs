
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

            // ── 写入启动标记（确认 setup 执行）──
            let marker_dir: std::path::PathBuf = dirs::home_dir()
                .unwrap_or_else(std::env::temp_dir);
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
                    let size = path.as_ref()
                        .and_then(|p| std::fs::metadata(p).ok())
                        .map(|m| m.len())
                        .unwrap_or(0);
                    info.push_str(&format!("  sidecar {}: exists={}, size={} bytes, path={:?}\n", name, exists, size, path));
                }
                info
            } else {
                "  debug mode — sidecar check skipped".to_string()
            };
            let _ = std::fs::OpenOptions::new()
                .create(true).append(true)
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
                            .create(true).append(true)
                            .open(&marker_path_for_thread)
                            .and_then(|mut f| std::io::Write::write_all(
                                &mut f, format!("\n\nERROR: {}\n", msg).as_bytes()));

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

fn start_nx_api(app_handle: &tauri::AppHandle, claude_cli_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    // All diagnostic output goes to this log file — on Windows stderr is invisible
    let diag_path = std::env::temp_dir().join("nx_startup.log");
    let diag = |msg: &str| {
        let entry = format!("[{}] {}\n", chrono_or_timestamp(), msg);
        eprintln!("{}", entry.trim());
        let _ = std::fs::OpenOptions::new()
            .create(true).append(true)
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
            if cfg!(target_arch = "aarch64") { "aarch64-apple-darwin" } else { "x86_64-apple-darwin" }
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

        let resource_dir = app_handle.path().resource_dir()
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
                candidates.into_iter().next().unwrap_or_else(|| resource_dir.join(&sidecar_name))
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
                diag(&format!("WARNING: template DB not found at {:?}, nx_api will create empty DB", template));
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
        .env("NEXUS_ALLOWED_ORIGINS", "tauri://localhost,http://localhost:5173,http://localhost:3000")
        .env("RUST_LOG", "info");

    // On Windows, add sidecar directory to PATH so DLLs can be found
    #[cfg(target_os = "windows")]
    if let Some(sidecar_dir) = nx_api_path.parent() {
        let path_env = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{};{}", sidecar_dir.display(), path_env);
        child_cmd.env("PATH", &new_path);
        diag(&format!("Added to PATH: {:?}", sidecar_dir));
    }

    // Pass resolved Claude CLI path to nx_api
    if let Some(cli_path) = claude_cli_path {
        diag(&format!("Claude CLI path: {}", cli_path));
        child_cmd.env("CLAUDE_CLI_PATH_OVERRIDE", cli_path);
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
                let msg = format!("nx_api exited (status: {})\n--- nx_api log ---\n{}", status, log);
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
        ).is_ok() {
            diag(&format!("nx_api ready on port 8080 (after {}x500ms)", i + 1));
            ready = true;
            break;
        }
    }

    if !ready {
        let log = std::fs::read_to_string(&log_path).unwrap_or_default();
        let msg = format!("nx_api started but port 8080 not responding after 10s\n--- nx_api log ---\n{}", log);
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
