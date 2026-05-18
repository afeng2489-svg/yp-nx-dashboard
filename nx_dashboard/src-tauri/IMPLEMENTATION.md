# TeamFlow Tauri 桌面端 — 实施计划

> 基于 [ARCHITECTURE.md](./ARCHITECTURE.md) 的模块拆分与代码迁移方案

## 目标

将 `lib.rs` (~718 行) 按功能域拆分为 5 个独立模块：

```
src/
├── main.rs          # 入口（不变）
├── lib.rs           # run() 入口 + 模块声明 + 类型导出
├── pty.rs           # PTY WebSocket 桥接（~160 行）
├── session.rs       # 团队会话 spawn（~100 行）
├── binary.rs        # 二进制路径解析（~60 行）
├── nx_api.rs        # nx_api 启动/监控/日志（~240 行）
└── diag.rs          # 启动诊断与日志（~80 行）
```

## 1. 模块依赖关系

```
lib.rs
  ├── diag.rs        （无依赖）
  ├── binary.rs      （无依赖）
  ├── nx_api.rs      → binary.rs, diag.rs
  ├── pty.rs         （无外部模块依赖，仅依赖 tauri/tokio-tungstenite）
  └── session.rs     → binary.rs
```

各模块之间**不互相依赖**，均通过 `lib.rs` 聚合。

## 2. 模块详细设计

### 2.1 `diag.rs` — 启动诊断与日志

**导出符号：**
- `pub fn write_startup_error(msg: &str)`
- `pub fn chrono_or_timestamp() -> String`
- `pub fn create_diag_logger() -> DiagLogger` (可选封装)

**代码来源：** `lib.rs`
- `write_startup_error()` 函数（L404-413）
- `chrono_or_timestamp()` 函数（L712-717）
- `kill_stale_nx_api()` 中与诊断相关的部分

**新增内容：**
- `DiagLogger` 结构体：统一封装文件日志 + `eprintln!`，替代分散的 `diag` 闭包

```rust
use std::fs::OpenOptions;
use std::io::Write;

/// 诊断日志器：同时写入临时目录日志文件和 stderr
pub struct DiagLogger {
    log_path: std::path::PathBuf,
}

impl DiagLogger {
    pub fn new(name: &str) -> Self {
        let log_path = std::env::temp_dir().join(name);
        Self { log_path }
    }

    pub fn log(&self, msg: &str) {
        let entry = format!("[{}] {}\n", chrono_or_timestamp(), msg);
        eprint!("{}", entry);
        let _ = OpenOptions::new()
            .create(true).append(true)
            .open(&self.log_path)
            .and_then(|mut f| f.write_all(entry.as_bytes()));
    }
}

pub fn chrono_or_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_else(|_| "???".to_string())
}

pub fn write_startup_error(msg: &str) {
    let log_path = std::env::temp_dir().join("nx_startup.log");
    let entry = format!("{}\n", msg);
    let _ = OpenOptions::new()
        .create(true).append(true)
        .open(&log_path)
        .and_then(|mut f| f.write_all(entry.as_bytes()));
    eprintln!("{}", msg);
}
```

### 2.2 `binary.rs` — 二进制路径解析

**导出符号：**
- `pub fn resolve_nx_binary() -> Result<PathBuf, String>`
- `pub fn resolve_nx_api_path(app_handle: &tauri::AppHandle) -> Result<(PathBuf, PathBuf, PathBuf), String>`
- `pub fn resolve_db_and_skills_paths(app_handle: &tauri::AppHandle) -> Result<(PathBuf, PathBuf, PathBuf), String>`
- `pub fn find_workspace_root() -> Option<PathBuf>`

**代码来源：** `lib.rs`
- `resolve_nx_binary()`（L204-230）
- `find_workspace_root()`（L440-469）

**新增内容：**
- 将 `start_nx_api()` 中的路径解析逻辑抽取为独立函数
- 按 debug/release 模式返回不同类型路径

```rust
use std::path::PathBuf;

/// 在 debug/release 模式下解析 nx_cli 路径
pub fn resolve_nx_binary() -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        let root = find_workspace_root()
            .ok_or_else(|| "无法找到 workspace root".to_string())?;
        let nx = root.join("target/debug/nx");
        if nx.exists() {
            Ok(nx)
        } else {
            Err(format!("nx CLI 二进制文件未找到: {:?}", nx))
        }
    } else {
        let exe = std::env::current_exe().map_err(|e| format!("{}", e))?;
        let exe_dir = exe.parent().ok_or("无法获取可执行文件目录")?;
        let name = if cfg!(target_os = "windows") { "nx.exe" } else { "nx" };
        let path = exe_dir.join(name);
        if path.exists() { Ok(path) }
        else { Err(format!("nx CLI 未找到: {:?}", path)) }
    }
}

/// 查找 workspace root：包含 Cargo.toml（含 [workspace]）和 nx_dashboard/ 的目录
pub fn find_workspace_root() -> Option<PathBuf> {
    let is_workspace = |dir: &std::path::Path| -> bool {
        if !dir.join("Cargo.toml").exists() || !dir.join("nx_dashboard").is_dir() {
            return false;
        }
        std::fs::read_to_string(dir.join("Cargo.toml"))
            .map(|c| c.contains("[workspace]"))
            .unwrap_or(false)
    };
    if let Ok(exe) = std::env::current_exe() {
        let exe = exe.canonicalize().unwrap_or(exe);
        for ancestor in exe.ancestors().skip(1) {
            if is_workspace(ancestor) {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        for ancestor in cwd.ancestors() {
            if is_workspace(ancestor) { return Some(ancestor.to_path_buf()); }
        }
    }
    None
}
```

### 2.3 `pty.rs` — PTY WebSocket 桥接

**导出符号：**
- `pub type PtyConnections = Arc<Mutex<HashMap<String, mpsc::Sender<WsMessage>>>>`
- `pub fn pty_connect(...)` — `#[tauri::command]`
- `pub fn pty_send_input(...)` — `#[tauri::command]`
- `pub fn pty_send_control(...)` — `#[tauri::command]`
- `pub fn pty_disconnect(...)` — `#[tauri::command]`

**代码来源：** `lib.rs`
- PTY 类型别名（L14-15）
- `pty_connect()`（L24-95）
- `pty_send_input()`（L99-114）
- `pty_send_control()`（L118-133）
- `pty_disconnect()`（L234-241）

**迁移说明：**
- 本模块基本保持原样迁移
- 需要 `use` 新增 `/ use std::sync::{Arc, Mutex}` 等
- 导入 `tauri::Emitter`, `tauri::Manager`, `futures_util`, `tokio_tungstenite`

### 2.4 `session.rs` — 团队会话管理

**导出符号：**
- `pub fn spawn_team_session(...)` — `#[tauri::command]`

**代码来源：** `lib.rs`
- `spawn_team_session()`（L139-201）

**迁移说明：**
- 依赖 `binary.rs` 的 `resolve_nx_binary()`
- 消费 stdout 的逻辑保持原样
- 导入 `tauri::Emitter`, `serde_json`

### 2.5 `nx_api.rs` — nx_api 后台服务启动

**导出符号：**
- `pub fn start_nx_api(app_handle: &tauri::AppHandle, claude_cli_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>>`
- `pub fn kill_stale_nx_api()`
- `pub fn write_setup_marker(app_handle: &tauri::AppHandle) -> PathBuf`

**代码来源：** `lib.rs`
- `kill_stale_nx_api()`（L417-437）
- `start_nx_api()`（L471-709）
- setup 阶段中的端口健康检查轮询逻辑

**依赖：** `binary.rs`（路径解析）、`diag.rs`（诊断日志）

**关键改动：**
- 使用 `DiagLogger` 替代内联 `diag` 闭包
- 将路径解析委托给 `binary.rs`
- 将 sidecar 候选路径查找逻辑抽取为辅助函数

```rust
use diag::DiagLogger;
use binary::{find_workspace_root, resolve_nx_api_path};

/// 查找 sidecar 候选路径
fn find_sidecar_candidates(exe_dir: &std::path::Path, resource_dir: &std::path::Path)
    -> Vec<std::path::PathBuf>
{
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
    let plain_name: String = if cfg!(target_os = "windows") {
        "nx_api.exe".to_string()
    } else {
        "nx_api".to_string()
    };

    let mut candidates = Vec::new();
    candidates.push(exe_dir.join(&sidecar_name));
    candidates.push(exe_dir.join(&plain_name));
    candidates.push(resource_dir.join(&sidecar_name));
    candidates.push(resource_dir.join(&plain_name));
    candidates.push(resource_dir.join("MacOS").join(&sidecar_name));
    candidates.push(resource_dir.join("MacOS").join(&plain_name));
    candidates
}
```

### 2.6 `lib.rs` — 聚合入口

**改动内容：**
- 声明子模块
- 重新导出公共类型（`PtyConnections`）
- 保持 `run()` 函数，其中组装各模块功能

```rust
mod binary;
mod diag;
mod nx_api;
mod pty;
mod session;

// 重新导出给 main.rs（main.rs 仅调用 lib::run()）
pub use pty::PtyConnections;

// 注册所有 tauri::command
// pty_connect, pty_send_input, pty_send_control, pty_disconnect
// spawn_team_session
```

## 3. 实施步骤

### Phase 1：提取无依赖模块

| 步骤 | 文件 | 操作 |
|------|------|------|
| 1 | `src/diag.rs` | 从 `lib.rs` 提取 `write_startup_error`、`chrono_or_timestamp`，新增 `DiagLogger` |
| 2 | `src/binary.rs` | 从 `lib.rs` 提取 `resolve_nx_binary`、`find_workspace_root`，新增路径解析函数 |

### Phase 2：提取核心功能模块

| 步骤 | 文件 | 操作 |
|------|------|------|
| 3 | `src/pty.rs` | 从 `lib.rs` 提取全部 PTY 相关代码 |
| 4 | `src/session.rs` | 从 `lib.rs` 提取 `spawn_team_session` |
| 5 | `src/nx_api.rs` | 从 `lib.rs` 提取 `start_nx_api`、`kill_stale_nx_api`，集成 `DiagLogger` |

### Phase 3：重构 lib.rs

| 步骤 | 操作 |
|------|------|
| 6 | 缩减 `lib.rs` 为模块声明 + `run()` 入口 |
| 7 | 验证 `cargo check` 通过 |
| 8 | 运行 `cargo fmt` 统一格式 |
| 9 | 运行 `cargo clippy` 修复 lint |

### Phase 4：测试与验证

| 步骤 | 操作 |
|------|------|
| 10 | 运行 `cargo test` 确保已有测试通过 |
| 11 | 为各模块新增单元测试模块 |

## 4. 模块大小预估

| 模块 | 拆分后行数 | 说明 |
|------|-----------|------|
| `lib.rs` | ~80 | 模块声明 + `run()` 入口 + setup 逻辑 |
| `pty.rs` | ~140 | 4 个 command + 类型别名 |
| `session.rs` | ~70 | 1 个 command + stdout 解析 |
| `binary.rs` | ~90 | 路径解析相关函数 |
| `nx_api.rs` | ~260 | 启动逻辑 + 端口轮询 + 进程监控 |
| `diag.rs` | ~70 | 诊断日志工具 |
| **合计** | ~710 | 与原 lib.rs 相当 |

## 5. 风险与注意事项

1. **函数可见性**：拆分后注意 `pub` vs `pub(crate)` 的边界，command 函数需 `pub`（`#[tauri::command]` 要求）
2. **类型依赖**：`PtyConnections` 被多个 command 共享，需从 `pty.rs` 重新导出
3. **生命周期**：`app_handle` 在 setup 闭包中捕获，拆分后通过参数传递到各模块
4. **Tauri 状态管理**：`.manage(pty_connections)` 注入的 `PtyConnections` 类型路径需一致
5. **Windows 兼容**：`#[cfg(target_os = "windows")]` 条件编译在拆分后保持正确

## 6. 成功标准

- [ ] `cargo check` 通过，无警告
- [ ] `cargo clippy` 通过（`-D warnings`）
- [ ] `cargo test` 全部通过
- [ ] 各模块 < 300 行，职责单一
- [ ] 模块间无循环依赖
- [ ] 所有现有功能不受影响
