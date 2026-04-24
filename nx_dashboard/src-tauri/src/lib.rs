
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use futures_util::{SinkExt, StreamExt};
use tauri::Emitter;
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

            // Start nx_api in background
            thread::spawn(move || {
                if let Err(e) = start_nx_api() {
                    eprintln!("[ERROR] Failed to start nx_api: {}", e);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ── nx_api Subprocess ───────────────────────────────────────────────────────

fn start_nx_api() -> Result<(), Box<dyn std::error::Error>> {
    let _ = std::fs::write("/tmp/nx_start_called.txt", "start_nx_api called");
    let nx_api_path = if cfg!(debug_assertions) {
        PathBuf::from("/Users/Zhuanz/Desktop/yp-nx-dashboard/target/release/nx_api")
    } else {
        let exe_path = std::env::current_exe()?;
        let contents = exe_path.parent().unwrap().parent().unwrap();
        contents.join("Resources/nx_api")
    };

    if !nx_api_path.exists() {
        return Err(format!("nx_api not found at {:?}", nx_api_path).into());
    }

    let skills_path = if cfg!(debug_assertions) {
        PathBuf::from("/Users/Zhuanz/Desktop/yp-nx-dashboard/.claude/agents")
    } else {
        let exe_path = std::env::current_exe()?;
        let contents = exe_path.parent().unwrap().parent().unwrap();
        contents.join("Resources/skills")
    };

    let data_dir = if cfg!(debug_assertions) {
        PathBuf::from("/Users/Zhuanz/Desktop/yp-nx-dashboard")
    } else {
        let exe = std::env::current_exe()?;
        exe.parent().unwrap().parent().unwrap()
           .join("Resources")
    };
    std::fs::create_dir_all(&data_dir)?;

    let log_path = PathBuf::from("/tmp/nx_api.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true).write(true).truncate(true)
        .open(&log_path)
        .map_err(|e| format!("Failed to open log file: {}", e))?;
    let log_file2 = log_file.try_clone()?;

    let mut child = Command::new(&nx_api_path)
        .env("AGENTS_DIR", &skills_path)
        .env("NEXUS_DB_PATH", data_dir.join("nx_dashboard/nexus.db"))
        .env("NEXUS_ALLOWED_ORIGINS", "tauri://localhost,http://localhost:5173,http://localhost:3000")
        .env("RUST_LOG", "info")
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
