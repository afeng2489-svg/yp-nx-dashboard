
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
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

fn start_nx_api() -> Result<(), Box<dyn std::error::Error>> {
    // Determine the path to nx_api
    let nx_api_path = if cfg!(debug_assertions) {
        // In development, use the built binary in target/release
        PathBuf::from("/Users/Zhuanz/Desktop/yp-nx-dashboard/target/release/nx_api")
    } else {
        // In release, use the bundled nx_api in Resources
        let exe_path = std::env::current_exe()?;
        let app_bundle = exe_path.parent().unwrap().parent().unwrap().parent().unwrap();
        app_bundle.join("Resources/nx_api")
    };

    // Check if nx_api exists
    if !nx_api_path.exists() {
        return Err(format!("nx_api not found at {:?}", nx_api_path).into());
    }

    // Set skills path
    let skills_path = if cfg!(debug_assertions) {
        PathBuf::from("/Users/Zhuanz/Desktop/yp-nx-dashboard/.claude/agents")
    } else {
        let exe_path = std::env::current_exe()?;
        let app_bundle = exe_path.parent().unwrap().parent().unwrap().parent().unwrap();
        app_bundle.join("Resources/skills")
    };

    // Create a writable data directory for the database
    // Always use the project directory for consistent data
    let data_dir = PathBuf::from("/Users/Zhuanz/Desktop/yp-nx-dashboard");

    // Create data directory if it doesn't exist
    std::fs::create_dir_all(&data_dir)?;

    // Set working directory and environment for nx_api
    let mut cmd = Command::new(&nx_api_path);
    cmd.env("AGENTS_DIR", &skills_path)
        .env("NEXUS_DB_PATH", data_dir.join("nexus.db"))
        .env("RUST_LOG", "info")
        .current_dir(&data_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    println!("[NX Dashboard] Starting nx_api from {:?}", nx_api_path);
    println!("[NX Dashboard] Data directory: {:?}", data_dir);

    // Spawn nx_api
    let mut child = cmd.spawn()
        .map_err(|e| format!("Failed to spawn nx_api: {}", e))?;

    // Read output to check for errors
    use std::io::{BufRead, BufReader};
    let _stdout = child.stdout.take().map(BufReader::new);
    let stderr = child.stderr.take().map(BufReader::new);

    // Wait a moment and check if process is still running
    thread::sleep(std::time::Duration::from_secs(2));

    match child.try_wait() {
        Ok(Some(status)) => {
            return Err(format!("nx_api exited immediately with status: {}", status).into());
        }
        Ok(None) => {
            // Process is still running, success
            println!("[NX Dashboard] nx_api started successfully (PID: {:?})", child.id());
        }
        Err(e) => {
            return Err(format!("Failed to check nx_api status: {}", e).into());
        }
    }

    // Log any stderr output
    if let Some(stderr) = stderr {
        for line in stderr.lines().take(5) {
            if let Ok(line) = line {
                println!("[NX Dashboard] nx_api: {}", line);
            }
        }
    }

    Ok(())
}