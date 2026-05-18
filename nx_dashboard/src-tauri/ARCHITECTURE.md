# TeamFlow Tauri 桌面端架构设计

## 1. 概览与定位

TeamFlow 是一个 AI 开发团队协作桌面应用。Tauri 层（即本模块 `nx_dashboard/src-tauri`）承担三个核心角色：

| 角色 | 说明 |
|------|------|
| **桌面壳** | 承载 React 前端，提供原生窗口、菜单、系统托盘 |
| **进程管理器** | 负责启动/监控 `nx_api` 后台服务（HTTP + WebSocket）和 `nx` CLI 子进程 |
| **IPC 桥接** | 前端 ↔ 原生层 ↔ 外部进程 之间的双向通信 |

```
┌─────────────────────────────────────────────────┐
│                  TeamFlow Desktop                │
│  ┌─────────────────┐  ┌──────────────────────┐  │
│  │  React Frontend  │  │   Tauri Rust Core    │  │
│  │  (WebView)       │◄─┤   (src-tauri/src/)   │  │
│  └─────────────────┘  └──────────┬───────────┘  │
│                                  │               │
│                    ┌─────────────┼───────────┐   │
│                    │             │           │   │
│               nx CLI       nx_api       PTY    │   │
│               (子进程)     (后台服务)   (WS)    │   │
└─────────────────────────────────────────────────┘
```

## 2. 模块划分

```
src-tauri/
├── src/
│   ├── main.rs          # 入口：windows_subsystem 配置，委托 lib::run()
│   ├── lib.rs           # 核心模块（当前单文件，建议后续拆分）
│   └── (future)         # 规划中的模块拆分
├── binaries/            # 随应用分发的 sidecar 二进制 (nx_api)
├── capabilities/        # Tauri 权限声明 (允许的 API 集合)
├── icons/               # 应用图标资源
├── gen/schemas/         # 自动生成的 JSON Schema
├── Cargo.toml           # 依赖声明
├── tauri.conf.json      # Tauri 构建 & 打包配置
└── build.rs             # Tauri 构建脚本
```

### 2.1 核心模块 (`src/lib.rs`) 功能域

当前 `lib.rs` 约 700 行，按功能可划分为以下子模块（建议后续拆分）：

| 子模块 | 职责 | 预估行数 |
|--------|------|---------|
| `pty_bridge` | WebSocket PTY 连接管理、消息双向转发 | ~160 |
| `team_session` | `nx team` 子进程启动、stdout 解析、生命周期管理 | ~100 |
| `binary_resolver` | nx CLI / nx_api 路径解析（debug/release/多平台） | ~60 |
| `nx_api_launcher` | nx_api 后台服务启动、端口健康检查、启动诊断 | ~240 |
| `startup_diag` | 启动标记写入、sidecar 检测、错误日志 | ~80 |

### 2.2 建议的模块目录结构

```
src/
├── main.rs
├── lib.rs              # run() 入口 + 模块声明
├── pty.rs              # PTY WebSocket 桥接
├── session.rs          # 团队会话 spawn
├── binary.rs           # 二进制路径解析
├── nx_api.rs           # nx_api 启动/监控/日志
└── diag.rs             # 启动诊断与日志
```

## 3. IPC 接口定义

### 3.1 Tauri Commands（前端调用的 Rust 函数）

所有命令通过 `#[tauri::command]` 注册，类型安全地暴露给前端。

#### PTY 终端连接

```
pty_connect(team_id: String, session_id: String) → Result<(), String>
```
- 建立到 `ws://127.0.0.1:8080/ws/teams/{team_id}/terminal/{session_id}` 的 WebSocket 连接
- 创建 mpsc channel 缓存发送端，替换旧的同 session_id 连接
- 启动两个异步任务：channel → WS（转发前端输入）、WS → 事件（推送输出到前端）
- 前端通过事件接收输出：
  - `pty-output-{session_id}`: Binary 帧（原始终端输出）
  - `pty-control-{session_id}`: Text 帧（JSON 控制消息）

```
pty_send_input(session_id: String, data: Vec<u8>) → Result<(), String>
```
- 向前台发送原始键盘输入字节

```
pty_send_control(session_id: String, message: String) → Result<(), String>
```
- 发送 JSON 控制消息（resize / task / close）

```
pty_disconnect(session_id: String) → Result<(), String>
```
- 移除 session 连接，触发 WS 关闭

#### 团队会话管理

```
spawn_team_session(task: String, model: Option<String>) → Result<String, String>
```
- 启动 `nx team {task} [--model {model}]` 子进程
- 从 stdout 解析 `session_id:` 前缀行返回给前端
- 后台线程消费剩余 stdout 防止 SIGPIPE，等待进程结束后 emit `team-session-completed` 事件

### 3.2 前端事件（Rust → 前端推送）

| 事件名 | 载荷 | 触发场景 |
|--------|------|---------|
| `pty-output-{id}` | `Vec<u8>` | PTY 终端输出数据到达 |
| `pty-control-{id}` | `String` (JSON) | PTY 控制消息到达 |
| `team-session-created` | `String` (session_id) | 团队会话成功启动 |
| `team-session-completed` | `{"sessionId": "...", "success": bool}` | 团队会话进程结束 |
| `nx-api-startup-error` | `String` | nx_api 后台服务启动失败 |

## 4. 数据流

### 4.1 PTY 终端数据流

```
用户键盘输入 (前端 WebView)
    │
    │  invoke('pty_send_input', { sessionId, data })
    ▼
Tauri IPC Handler ──► mpsc::Sender ──► WS Sink ──► nx_api (127.0.0.1:8080)
                                                       │
                                                       │  PTY 进程
                                                       ▼
                                                    终端输出
                                                       │
                                                       ▼
前端 ◄── tauri::Emitter ◄── WS Stream ◄── WebSocket ◄──┘
(事件: pty-output-{id} / pty-control-{id})
```

### 4.2 团队会话数据流

```
前端: invoke('spawn_team_session', { task, model })
    │
    ▼
Tauri IPC Handler
    │
    ├── resolve_nx_binary() → 找到 nx CLI 路径
    │
    ├── Command::new(nx).arg("team").arg(task) 启动子进程
    │
    ├── stdout 扫描 (前 20 行) → 解析 session_id:... → 返回前端
    │
    ├── emit("team-session-created", session_id)
    │
    └── 后台线程:
        ├── 消费剩余 stdout (防 SIGPIPE)
        ├── child.wait() 等待进程结束
        └── emit("team-session-completed", {sessionId, success})
```

### 4.3 nx_api 启动数据流

```
Tauri setup() 阶段
    │
    ├── Claude CLI 路径解析 (debug: None / release: which claude)
    ├── 启动标记写入 (~/.nx_tauri_setup.log)
    ├── sidecar 存在性检测
    │
    └── 后台线程: start_nx_api()
        ├── kill_stale_nx_api()              清理 8080 端口旧进程
        ├── resolve nx_api 路径              (debug: target/debug, release: sidecar)
        ├── resolve DB 路径                  (debug: 项目目录, release: 用户数据目录)
        ├── 设置环境变量: AGENTS_DIR, NEXUS_DB_PATH, NEXUS_ALLOWED_ORIGINS
        ├── Command::spawn()                 启动 nx_api
        ├── 轮询 127.0.0.1:8080 (20×500ms)  等待就绪
        └── child.wait()                     阻塞等待 (防止孤儿进程)
```

## 5. 跨平台策略

| 维度 | macOS (aarch64/x86_64) | Windows (x86_64) | Linux (x86_64) |
|------|------------------------|-------------------|----------------|
| **Sidecar 命名** | `nx_api-aarch64-apple-darwin` / `nx_api-x86_64-apple-darwin` | `nx_api-x86_64-pc-windows-msvc.exe` | `nx_api-x86_64-unknown-linux-gnu` |
| **Claude CLI 发现** | `$SHELL -l -c "which claude"` | `cmd /c where claude` | `$SHELL -l -c "which claude"` |
| **端口清理** | `lsof -i :8080 -t \| xargs kill -9` | `powershell Get-NetTCPConnection ...` | `lsof -i :8080 -t \| xargs kill -9` |
| **二进制权限** | `chmod +x` 确保可执行 | PATH 注入（DLL 搜索） | `chmod +x` |
| **sidecar 备选名** | 支持无 triple 后缀的短名 | 支持 `nx_api.exe` | 支持 `nx_api` |

### 5.1 Debug vs Release 模式

| 维度 | Debug | Release |
|------|-------|---------|
| `nx` CLI 路径 | `{workspace}/target/debug/nx` | 可执行文件同目录的 `nx` |
| `nx_api` 路径 | `{workspace}/target/debug/nx_api` | bundled sidecar (候选路径搜索) |
| DB 路径 | `{workspace}/nx_dashboard/nexus.db` | `~/.local/share/com.nx.dashboard/nexus.db` |
| Skills 路径 | `{workspace}/.claude/agents` | `{resource_dir}/skills` |
| 日志插件 | 启用 (level: Info) | 未启用 |
| Claude CLI | 从 shell PATH 自动发现 | 显式解析后传递 `CLAUDE_CLI_PATH_OVERRIDE` |

## 6. 技术选型理由

| 技术 | 版本 | 选型理由 |
|------|------|---------|
| **Tauri 2** | 2.4 | 相比 Electron 内存占用低 ~90%，Rust 原生性能，内置进程管理与 sidecar 支持 |
| **tokio-tungstenite** | 0.21 | 异步 WebSocket 客户端，与 Tauri 的 tokio 运行时兼容，支持 split stream/sink |
| **tokio mpsc** | 1.x | 异步通道，在 IPC handler 和 WS sink 之间解耦消息传递 |
| **futures-util** | 0.3 | Stream/Sink split 抽象，桥接 tungstenite 与 mpsc |
| **tauri-plugin-dialog** | 2.2-rc | 原生文件对话框，替代 Web 端受限的文件选择器 |
| **tauri-plugin-fs** | 2.2-rc | 原生文件系统访问，读取/写入权限远大于浏览器沙箱 |
| **tauri-plugin-log** | 2.0-rc | 统一日志输出到终端/文件（debug 模式），方便开发调试 |
| **dirs** | 5 | 跨平台用户目录解析（data_dir, home_dir），避免硬编码路径 |
| **serde/serde_json** | 1.0 | Rust 标准序列化方案，与 Tauri 命令/事件系统深度集成 |

### 6.1 关键设计决策

1. **nx_api 作为独立后台服务而非内嵌库**：前后端分离部署，可独立升级/重启/调试，通过 HTTP + WebSocket 通信，易于扩展为远程连接场景。

2. **WebSocket 做 PTY 桥接**：终端 I/O 是双向流式场景，WebSocket 提供全双工通信，`tokio-tungstenite` 的 Stream/Sink split 模式天然适配读写分离。

3. **mpsc channel 解耦**：前端可能高频发送输入事件，通过 channel 缓冲（容量 64）防止 WS 反压阻塞 IPC handler。

4. **子进程 stdout 消费线程**：`nx team` 可能持续输出大量日志，必须消费 stdout 防止写端阻塞导致 SIGPIPE → 子进程异常退出。

5. **启动诊断系统**：多层日志（标记文件 / temp dir / emit 事件）覆盖无界面启动失败的排查场景。

## 7. 安全考量

| 关注点 | 措施 |
|--------|------|
| **WebSocket 域** | 仅连接 `127.0.0.1:8080`，不暴露到公网 |
| **CSP** | 当前 `csp: null`（开发灵活性），上线前应设置合理的 CSP 策略 |
| **文件系统权限** | `fs:allow-open` 限制为 `**` 通配符，未来可按需收紧 |
| **子进程注入** | `task` 参数直接传入 CLI，无 shell 包装（`Command::new` 而非 `sh -c`），可防止命令注入 |
| **WebView 创建** | 已授予 `webview:*` 权限，用于多窗口功能，需注意跨窗口通信安全 |
| **环境变量** | Claude CLI 路径通过 `CLAUDE_CLI_PATH_OVERRIDE` 传递，不混入系统 PATH |

## 8. 后续演进方向

1. **模块拆分**：将 `lib.rs` (~700行) 按功能域拆分为 4-5 个文件
2. **nx_api 看门狗**：检测 nx_api 崩溃后自动重启
3. **健康检查 API**：暴露 `/health` 端点，前端可查询后端状态
4. **更新机制**：集成 Tauri updater plugin，支持自动更新
5. **日志系统**：统一使用 `tracing` 替代 `eprintln!` + 临时日志文件
6. **错误处理优化**：将 `String` 错误类型替换为结构化错误枚举
