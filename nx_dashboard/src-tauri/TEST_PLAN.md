# TeamFlow 测试计划

## 1. 测试策略总览

### 测试目标
- **行覆盖率 ≥ 80%**（纯函数目标 90%+）
- 覆盖所有 Tauri Command、工具函数、错误路径
- 模块化后的每个模块独立测试

### 测试层级

| 层级 | 范围 | 技术 | 位置 | 目标覆盖率 |
|------|------|------|------|-----------|
| **Unit** | 纯函数、工具函数 | `#[test]` | `src/` 内 `#[cfg(test)]` | 90%+ |
| **Integration** | 子进程管理、文件 I/O | `#[tokio::test]` + tempfile | `tests/` | 80% |
| **Mock** | Tauri Command、WS 连接 | mockall + 自定义 mock | `tests/` + `src/` | 85% |

### 测试环境

- `cargo test` — 并行运行所有测试
- `cargo test -- --nocapture` — 显示日志输出
- `cargo llvm-cov --html` — 覆盖率报告
- `cargo clippy -- -D warnings` — lint 检查

---

## 2. 模块测试

### 2.1 `binary` 模块 — nx CLI 二进制路径解析

**测试文件**: `src/binary.rs` (内联 `#[cfg(test)]`)

| # | 测试用例 | 类型 | 输入 | 预期结果 |
|---|---------|------|------|---------|
| 1 | `find_workspace_root_from_exe` | Unit | 模拟可执行文件路径 `.../target/debug/nx_dashboard`，上方存在含 `[workspace]` 的 Cargo.toml | 返回正确的 workspace root |
| 2 | `find_workspace_root_from_cwd` | Unit | 模拟 CWD 为 workspace 子目录 | 返回正确的 workspace root |
| 3 | `find_workspace_root_no_workspace` | Unit | Cargo.toml 存在但不含 `[workspace]` | `None` |
| 4 | `find_workspace_root_no_cargo` | Unit | 目录树中无 Cargo.toml | `None` |
| 5 | `find_workspace_root_missing_nx_dashboard` | Unit | Cargo.toml 含 `[workspace]` 但缺少 `nx_dashboard/` 子目录 | `None` |
| 6 | `resolve_nx_binary_debug_found` | Unit | cfg!(debug_assertions)=true，target/debug/nx 存在 | `Ok(path)` |
| 7 | `resolve_nx_binary_debug_not_found` | Unit | cfg!(debug_assertions)=true，target/debug/nx 不存在 | `Err(包含"未找到")` |
| 8 | `resolve_nx_binary_debug_no_workspace` | Unit | cfg!(debug_assertions)=true，找不到 workspace root | `Err(包含"无法找到 workspace root")` |

**测试代码示例** — `find_workspace_root`：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_workspace(dir: &TempDir) -> std::path::PathBuf {
        let cargo = dir.path().join("Cargo.toml");
        fs::write(&cargo, "[package]\nname = \"test\"\n[workspace]\n").unwrap();
        fs::create_dir_all(dir.path().join("nx_dashboard")).unwrap();
        cargo
    }

    #[test]
    fn find_workspace_root_from_exe() {
        // Simulate: exe is at <root>/target/debug/nx_dashboard
        let root = TempDir::new().unwrap();
        setup_workspace(&root);

        let debug_dir = root.path().join("target/debug");
        fs::create_dir_all(&debug_dir).unwrap();
        let exe = debug_dir.join("nx_dashboard");

        // Temporarily redirect current_exe
        // (using a helper that takes a mock path — refactor needed for testability)
        let found = find_workspace_root_with_hint(Some(&exe));
        assert_eq!(found, Some(root.path().to_path_buf()));
    }

    #[test]
    fn find_workspace_root_no_workspace_section() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"\n").unwrap();
        fs::create_dir_all(dir.path().join("nx_dashboard")).unwrap();

        assert_eq!(find_workspace_root_with_hint(Some(&dir.path().join("dummy"))), None);
    }

    #[test]
    fn find_workspace_root_missing_nx_dashboard() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"\n[workspace]\n").unwrap();
        // no nx_dashboard/ subdir

        assert_eq!(find_workspace_root_with_hint(Some(&dir.path().join("dummy"))), None);
    }
}
```

> **注意**: 当前 `find_workspace_root()` 内部调用 `std::env::current_exe()`，无法在单元测试中模拟。建议重构为接受可选路径参数的版本，Unit test 注入模拟路径，Integration test 测试真实行为。

```rust
// 重构建议版本
fn find_workspace_root() -> Option<PathBuf> {
    _find_workspace_root_inner(std::env::current_exe().ok().as_ref())
}

// 可测试的内部函数
fn _find_workspace_root_inner(exe_hint: Option<&std::path::Path>) -> Option<PathBuf> {
    let is_workspace = |dir: &std::path::Path| -> bool {
        if !dir.join("Cargo.toml").exists() || !dir.join("nx_dashboard").is_dir() {
            return false;
        }
        std::fs::read_to_string(dir.join("Cargo.toml"))
            .map(|c| c.contains("[workspace]"))
            .unwrap_or(false)
    };

    if let Some(exe) = exe_hint {
        let exe = exe.canonicalize().unwrap_or(exe.to_path_buf());
        for ancestor in exe.ancestors().skip(1) {
            if is_workspace(ancestor) {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        for ancestor in cwd.ancestors() {
            if is_workspace(ancestor) {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    None
}
```

---

### 2.2 `diag` 模块 — 诊断和日志工具

**测试文件**: `tests/diag_test.rs`

| # | 测试用例 | 类型 | 输入 | 预期结果 |
|---|---------|------|------|---------|
| 1 | `chrono_or_timestamp_format` | Unit | 无参 | 返回非空字符串，格式为纯数字 |
| 2 | `write_startup_error_creates_file` | Integration | 任意错误消息 | temp_dir/nx_startup.log 被创建且包含该消息 |
| 3 | `write_startup_error_appends` | Integration | 两次调用 | 文件包含两行内容 |
| 4 | `write_startup_error_empty_msg` | Unit | 空字符串 `""` | 文件写入一个换行符 |

**测试代码**:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn chrono_or_timestamp_format() {
        let ts = chrono_or_timestamp();
        assert!(!ts.is_empty(), "timestamp should not be empty");
        assert!(ts.chars().all(|c| c.is_ascii_digit()), "timestamp should be digits only, got: {}", ts);
    }

    #[test]
    fn chrono_or_timestamp_is_recent() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let ts: u64 = chrono_or_timestamp().parse().unwrap();
        // Within 5 seconds of actual time
        assert!(ts.abs_diff(now) < 5, "timestamp {} differs from actual {} by >5s", ts, now);
    }

    #[test]
    fn write_startup_error_creates_file() {
        let msg = "test error message";
        write_startup_error(msg);

        let log_path = std::env::temp_dir().join("nx_startup.log");
        let content = fs::read_to_string(&log_path).unwrap_or_default();
        assert!(content.contains(msg), "log should contain: {}\nactual: {}", msg, content);

        // Cleanup
        let _ = fs::remove_file(&log_path);
    }

    #[test]
    fn write_startup_error_appends() {
        write_startup_error("first");
        write_startup_error("second");

        let log_path = std::env::temp_dir().join("nx_startup.log");
        let content = fs::read_to_string(&log_path).unwrap_or_default();
        let lines: Vec<&str> = content.lines().collect();
        assert!(lines.len() >= 2, "expected >=2 lines, got {}: {:?}", lines.len(), lines);
        assert!(content.contains("first"), "missing 'first'");
        assert!(content.contains("second"), "missing 'second'");

        let _ = fs::remove_file(&log_path);
    }
}
```

---

### 2.3 `nx_api` 模块 — 后台服务子进程管理

**测试文件**: `tests/nx_api_test.rs`

| # | 测试用例 | 类型 | 输入 | 预期结果 |
|---|---------|------|------|---------|
| 1 | `kill_stale_nx_api_no_process` | Integration | 端口 8080 无进程 | 不 panic，正常返回 |
| 2 | `kill_stale_nx_api_handles_lsof_error` | Unit | lsof 命令失败（mock） | 不 panic，正常返回 |
| 3 | `start_nx_api_binary_not_found` | Integration | nx_api 路径不存在 | 返回 Err |
| 4 | `start_nx_api_binary_empty` | Integration | nx_api 文件存在但大小为 0 | 返回 Err(包含"empty") |
| 5 | `resolve_nx_api_path_debug` | Integration | cfg!(debug_assertions)=true, 存在 target/debug/nx_api | Ok |
| 6 | `resolve_nx_api_path_debug_not_found` | Integration | cfg!(debug_assertions)=true, 不存在 target/debug/nx_api | Err |
| 7 | `resolve_nx_api_path_release_sidecar` | Integration | cfg!(debug_assertions)=false, sidecar 在候选路径存在 | 返回正确的 path |
| 8 | `start_nx_api_os_permissions` | Integration | Release 模式下 unix 可执行权限 | 二进制获得 +x 权限 |

**测试注意事项**:
- `kill_stale_nx_api` 直接调用了 `lsof`/`Get-NetTCPConnection` 等系统命令，集成测试应确保不 panic（即命令不存在时优雅降级）
- `start_nx_api` 涉及子进程生命周期，单元测试聚焦路径解析和错误处理
- 使用 `tempfile::TempDir` 创建临时目录模拟文件结构

---

### 2.4 `session` 模块 — 团队会话管理

**测试文件**: `tests/session_test.rs`

| # | 测试用例 | 类型 | 输入 | 预期结果 |
|---|---------|------|------|---------|
| 1 | `spawn_team_session_parse_session_id` | Unit | 模拟 stdout 包含 `session_id: abc-123` | 返回 `Ok("abc-123".to_string())` |
| 2 | `spawn_team_session_parse_with_whitespace` | Unit | 模拟 stdout 包含 `session_id:  abc-123  \n` | 返回 `Ok("abc-123")`（trimmed） |
| 3 | `spawn_team_session_no_session_id` | Unit | 模拟 stdout 不包含 session_id | `Err(包含"未能解析 session_id")` |
| 4 | `spawn_team_session_empty_stdout` | Unit | 模拟 stdout 为空 | `Err` |
| 5 | `spawn_team_session_model_flag` | Unit | 传入 `model = Some("gpt-4")` | 命令参数包含 `--model gpt-4` |
| 6 | `spawn_team_session_binary_not_found` | Integration | nx_bin 路径不存在 | `Err` |
| 7 | `spawn_team_session_emit_on_created` | Mock | 解析成功后 | app_handle.emit("team-session-created", ...) 被调用 |

**测试代码示例** — session ID 解析:

```rust
// 从 spawn_team_session 提取的 stdout 解析函数
fn parse_session_id<R: std::io::BufRead>(reader: &mut R) -> Result<String, String> {
    let mut scanned = String::new();
    for _ in 0..20 {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            break;
        }
        if let Some(sid) = line.strip_prefix("session_id:") {
            return Ok(sid.trim().to_string());
        }
        scanned.push_str(&line);
    }
    Err(format!("未能解析 session_id，已扫描行:\n{}", &scanned[..scanned.len().min(500)]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn parse_session_id_found() {
        let input = "some output\nsession_id: abc-123\nmore output\n";
        let mut reader = BufReader::new(input.as_bytes());
        let result = parse_session_id(&mut reader);
        assert_eq!(result, Ok("abc-123".to_string()));
    }

    #[test]
    fn parse_session_id_with_whitespace() {
        let input = "session_id:   abc-123  \n";
        let mut reader = BufReader::new(input.as_bytes());
        let result = parse_session_id(&mut reader);
        assert_eq!(result, Ok("abc-123".to_string()));
    }

    #[test]
    fn parse_session_id_not_found() {
        let input = "some output\nno session here\n";
        let mut reader = BufReader::new(input.as_bytes());
        let result = parse_session_id(&mut reader);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未能解析"));
    }

    #[test]
    fn parse_session_id_empty_input() {
        let input = "";
        let mut reader = BufReader::new(input.as_bytes());
        let result = parse_session_id(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn parse_session_id_exceeds_max_lines() {
        let input = (0..25).map(|i| format!("line_{}\n", i)).collect::<String>();
        let mut reader = BufReader::new(input.as_bytes());
        let result = parse_session_id(&mut reader);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("line_19")); // 最多 20 行
    }
}
```

---

### 2.5 `pty` 模块 — PTY WebSocket 连接管理

**测试文件**: `tests/pty_test.rs`

| # | 测试用例 | 类型 | 输入 | 预期结果 |
|---|---------|------|------|---------|
| 1 | `pty_connect_invalid_url` | Integration | 无效的 team_id/session_id | WS 连接失败 → Err |
| 2 | `pty_connect_refused` | Integration | 127.0.0.1:1（无服务） | Err(包含"WS connect failed") |
| 3 | `pty_connect_duplicate_session` | Integration | 同一 session_id 连接两次 | 旧连接被移除，新连接创建 |
| 4 | `pty_send_input_no_connection` | Unit | session_id 不在 connections 中 | `Ok(())`（静默跳过） |
| 5 | `pty_send_control_no_connection` | Unit | session_id 不在 connections 中 | `Ok(())`（静默跳过） |
| 6 | `pty_disconnect_removes` | Unit | 断开存在的 session | connections 中不再包含该 session |
| 7 | `pty_disconnect_nonexistent` | Unit | 断开不存在的 session | `Ok(())`（静默返回） |
| 8 | `pty_ws_message_binary` | Unit | WS 返回 Binary 消息 | 触发 `pty-output-{id}` 事件 |
| 9 | `pty_ws_message_text` | Unit | WS 返回 Text 消息 | 触发 `pty-control-{id}` 事件 |
| 10 | `pty_ws_message_close` | Unit | WS 返回 Close 消息 | 触发 `{"type":"closed"}` 事件 |
| 11 | `pty_ws_message_error` | Unit | WS stream 返回 Err | 触发 `{"type":"error"}` 事件 |

**测试代码示例** — Connections HashMap 操作：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    type TestConnections = Arc<Mutex<HashMap<String, String>>>;

    fn setup() -> TestConnections {
        Arc::new(Mutex::new(HashMap::new()))
    }

    fn insert(conns: &TestConnections, id: &str) {
        conns.lock().unwrap().insert(id.to_string(), id.to_string());
    }

    #[test]
    fn disconnect_removes_session() {
        let conns = setup();
        insert(&conns, "session-1");
        insert(&conns, "session-2");

        conns.lock().unwrap().remove("session-1");
        assert!(conns.lock().unwrap().get("session-1").is_none());
        assert!(conns.lock().unwrap().get("session-2").is_some());
    }

    #[test]
    fn disconnect_removes_nonexistent() {
        let conns = setup();
        conns.lock().unwrap().remove("ghost-session");
        // Should not panic
        assert!(conns.lock().unwrap().is_empty());
    }

    #[test]
    fn duplicate_connection_replaces() {
        let conns = setup();
        insert(&conns, "session-1");
        insert(&conns, "session-1"); // replace
        assert_eq!(conns.lock().unwrap().len(), 1);
    }

    #[test]
    fn send_input_to_nonexistent_session() {
        let conns: Arc<Mutex<HashMap<String, mpsc::Sender<WsMessage>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        // Must not panic — just returns Ok(())
        // Note: in the actual command, the logic is:
        //   let tx = conns.lock().get(&session_id).cloned()
        //   if let Some(tx) = tx { tx.send(...) }
        // So missing session → no-op
        let guard = conns.lock().unwrap();
        let tx = guard.get("nonexistent").cloned();
        assert!(tx.is_none());
    }
}
```

---

## 3. Tauri Command 集成测试

使用 `tauri::test::mock_builder` 模拟 Tauri 运行时进行 Command 测试。

| # | 测试用例 | Command | 模拟条件 | 预期结果 |
|---|---------|---------|---------|---------|
| 1 | `pty_connect_invalid_ws_url` | `pty_connect` | State 注入空 HashMap | Err |
| 2 | `pty_disconnect_ok` | `pty_disconnect` | State 注入含 session 的 HashMap | Ok |
| 3 | `pty_send_input_ok` | `pty_send_input` | State 注入含 session 的 HashMap | Ok（静默成功） |
| 4 | `pty_send_control_ok` | `pty_send_control` | State 注入含 session 的 HashMap | Ok（静默成功） |
| 5 | `spawn_team_session_missing_binary` | `spawn_team_session` | resolve_nx_binary 返回 Err | Err |

**Tauri mock builder 示例**:

```rust
#[cfg(test)]
mod tauri_tests {
    use super::*;
    use tauri::test::{mock_builder, MockInvokeContext};

    #[test]
    fn pty_disconnect_removes_connection() {
        let connections: PtyConnections = Arc::new(Mutex::new(HashMap::new()));
        {
            let mut conns = connections.lock().unwrap();
            let (tx, _rx) = mpsc::channel(64);
            conns.insert("test-session".to_string(), tx);
        }

        let app = tauri::test::mock_builder()
            .manage(connections)
            .build();

        let state: tauri::State<PtyConnections> = app.state();

        // 调用 pty_disconnect
        let result = tauri::test::get_command::<pty_disconnect>(
            &app,
            tauri::test::MockInvokeContext::new(),
            ("test-session".to_string(),),
        );

        assert!(result.is_ok());
        let guard = state.inner().lock().unwrap();
        assert!(guard.get("test-session").is_none());
    }
}
```

---

## 4. 边界条件测试

### 4.1 字符串和路径边界

| # | 场景 | 输入 | 预期 |
|---|------|------|------|
| 1 | `session_id` 超长（10KB） | 10KB 的 session_id 字符串 | 正常处理，不 panic |
| 2 | `session_id` 含特殊字符 | `../../etc/passwd` | 作为普通字符串处理，不造成路径穿越 |
| 3 | `task` 参数超长 | 1MB 的 task 字符串 | 正常传递，不 panic |
| 4 | stdout 行超长 | 单行 1MB 无换行 | `read_line` 正常处理 |
| 5 | 路径含 unicode | `workspace/中文/target/debug/nx` | 正常工作 |

### 4.2 并发边界

| # | 场景 | 操作 | 预期 |
|---|------|------|------|
| 1 | 并发连接 | 10 个 session 同时 `pty_connect` | 全部成功，无死锁 |
| 2 | 并发读写 | 同一 session 的 input 和 output 同时操作 | 无数据竞争 |
| 3 | 并发 disconnect | 多个线程同时 `pty_disconnect` 同一 session | 无 panic，最终状态一致 |
| 4 | 连接后立即断开 | `pty_connect` 刚返回就 `pty_disconnect` | 后台任务优雅终止 |

### 4.3 子进程边界

| # | 场景 | 操作 | 预期 |
|---|------|------|------|
| 1 | nx_api 启动超时 | TCP 端口 8080 10 秒内无响应 | Err(包含"not responding") |
| 2 | nx_api 启动后立即崩溃 | 子进程在 500ms 内退出 | Err(包含"exited") |
| 3 | nx_api 二进制为 0 字节 | 使用空文件模拟 | Err(包含"empty") |
| 4 | nx_api 二进制无执行权限 | Unix 下缺少 +x | spawn 失败 |
| 5 | 磁盘空间不足 | 日志写入失败 | 静默处理（write! 已用 `let _ =`） |
| 6 | nx 命令返回非零退出码 | `nx team` 执行失败 | spawn_team_session 的线程中 success=false |

---

## 5. 异常路径测试

### 5.1 系统命令失败

| # | 场景 | 对应函数 | 预期行为 |
|---|------|---------|---------|
| 1 | `lsof` 命令不存在 | `kill_stale_nx_api` | 不 panic（Command::new 返回 Err） |
| 2 | `kill` 命令不存在 | `kill_stale_nx_api` | 不 panic |
| 3 | `lsof` 输出包含非法 PID | `kill_stale_nx_api` | `parse()` 失败，跳过该行 |
| 4 | `which claude` 找不到 | `run()` setup | `claude_cli_env = None`，不阻塞启动 |
| 5 | `shell` 环境变量为空 | `run()` setup Linux | 默认使用 `/bin/zsh` |

### 5.2 文件系统异常

| # | 场景 | 对应函数 | 预期行为 |
|---|------|---------|---------|
| 1 | temp_dir() 不可写 | `write_startup_error` | 静默失败（`let _ =`） |
| 2 | Cargo.toml 无法读取 | `find_workspace_root` | `None` |
| 3 | marker 文件无法写入 | `run()` setup | 静默失败，启动继续 |
| 4 | DB 模板文件损坏 | `start_nx_api` release 路径 | `fs::copy` 返回 Err，但已用 `if template.exists()` 预检 |
| 5 | APP data 目录创建失败 | `start_nx_api` release 路径 | `create_dir_all` 返回 Err，传播到调用方 |

### 5.3 Tokio/Async 异常

| # | 场景 | 对应任务 | 预期行为 |
|---|------|---------|---------|
| 1 | WS sink send 失败 | PTY Task 1 (channel → WS) | break，任务终止 |
| 2 | WS stream recv 返回 None | PTY Task 2 (WS → events) | 循环结束，任务终止 |
| 3 | mpsc channel 满 | PTY Task 1 | `send().await` 等待（背压） |
| 4 | app_handle.emit 失败 | PTY Task 2 | `let _ =` 静默处理 |

---

## 6. Testability 重构建议

当前代码中有以下影响可测试性的模式，建议在拆分模块时一并重构：

| 问题 | 位置 | 影响 | 重构建议 |
|------|------|------|---------|
| `current_exe()` 硬依赖 | `find_workspace_root` | 无法 mock 路径 | 抽离为 `_find_workspace_root_inner(exe_hint)` |
| `stdout 解析` 内联在 Command 中 | `spawn_team_session` | 无法单独测试 | 提取为 `parse_session_id(reader: impl BufRead) -> Result` |
| `connections` 逻辑混在 Command 中 | `pty_send_input/control` | 无法单独测试 | 提取 `get_sender()` 等辅助函数 |
| `diag` 闭包内联 | `start_nx_api` | 无法验证日志输出 | 提取为 `DiagLogger` 结构体 |
| `kill_stale_nx_api` 调用系统命令 | 全局函数 | 集成测试可能误杀进程 | 预留环境变量禁用或 mock |
| 启动标记路径不可配置 | `run()` setup | 测试会写入真实用户目录 | `write_setup_marker(dir: impl AsRef<Path>)` |

### 关键重构示例

```rust
// 1. 提取 parse_session_id
pub fn parse_session_id<R: BufRead>(reader: &mut R) -> Result<String, String> {
    // ... (见 2.4 节)
}

// 2. 提取 diag logger
pub struct DiagLogger {
    log_path: PathBuf,
}

impl DiagLogger {
    pub fn new(log_path: PathBuf) -> Self { Self { log_path } }
    pub fn log(&self, msg: &str) {
        let entry = format!("[{}] {}\n", chrono_or_timestamp(), msg);
        eprintln!("{}", entry.trim());
        let _ = std::fs::OpenOptions::new()
            .create(true).append(true)
            .open(&self.log_path)
            .and_then(|mut f| f.write_all(entry.as_bytes()));
    }
}

// 3. 提取 get_sender
pub fn get_sender(
    connections: &PtyConnections,
    session_id: &str,
) -> Option<mpsc::Sender<WsMessage>> {
    let conns = connections.lock().unwrap();
    conns.get(session_id).cloned()
}
```

---

## 7. 运行测试

### 7.1 完整测试

```bash
# 所有测试
cargo test

# 仅单元测试
cargo test --lib

# 仅集成测试
cargo test --test '*' -- --test-threads=4

# 特定模块
cargo test pty::tests
cargo test diag_test
cargo test parse_session_id

# 覆盖率报告
cargo llvm-cov --html
open target/llvm-cov/html/index.html
```

### 7.2 CI 集成（.github/workflows/ci.yml）

```yaml
- name: Run tests
  run: cargo test -- --test-threads=4

- name: Coverage
  run: |
    cargo llvm-cov --fail-under-lines 80
```

### 7.3 已知限制

| 限制 | 原因 | 替代方案 |
|------|------|---------|
| `pty_connect` 无法真实测试 | 需要运行中的 nx_api WS 服务 | 使用 mock WS server（`tokio_tungstenite::accept_async`） |
| `start_nx_api` 完整流程 | 会实际启动子进程 | 只在 CI 中运行，本地用 mock |
| `run()` 完整启动 | 需要 Tauri 运行时和窗口 | 只测试 `setup` 闭包中的逻辑 |
| `spawn_team_session` 真实执行 | 需要 nx CLI | Extract `parse_session_id` 单元测试 |
| 跨平台 `kill_stale_nx_api` | Windows/macOS/Linux 命令不同 | 分别在各平台运行 |

---

## 8. 测试验收标准

| 检查项 | 标准 |
|--------|------|
| 行覆盖率 | ≥ 80%（`cargo llvm-cov --fail-under-lines 80`） |
| 单元测试通过 | `cargo test --lib` 全部通过 |
| 集成测试通过 | `cargo test --test '*'` 全部通过 |
| Clippy | `cargo clippy -- -D warnings` 无警告 |
| 新增代码测试先行 | TDD 流程（先写测试，再实现） |
| 测试独立性 | 每个测试独立，可并行运行（`--test-threads=8` 无竞争） |
| 无 flaky 测试 | 连续 3 次运行结果一致 |
