# NexusFlow CLI 安全策略

AF-00 引入的可配置 Claude Code CLI 安全边界。

## permissions_mode

| 模式 | CLI 行为 | 适用场景 |
|------|----------|----------|
| **strict**（默认） | 不传 `--dangerously-skip-permissions` | 生产、Golden Path、公测 |
| **trusted** | 传 `--dangerously-skip-permissions` | 本地开发、内测（需知情） |

### 配置方式

1. **环境变量**（持久）：`NEXUS_PERMISSIONS_MODE=strict` 或 `trusted`
2. **API**（运行时，重启后仍受 env 影响 unless 再次设置）：
   - `GET /api/v1/ai/security/permissions-mode`
   - `PUT /api/v1/ai/security/permissions-mode` body: `{ "mode": "strict" | "trusted" }`

### trusted 模式风险

Claude Code 可在 workspace 内自动执行 shell、改文件，无逐条确认。仅在可信项目与可信网络下使用。

## workspace 边界

- CLI 的 `current_dir` / `--project` 必须在用户选定的 workspace 路径内（canonical 路径前缀校验）。
- 路径落在 workspace 外时拒绝启动，返回清晰错误。

## 危险命令 blocklist

quality_gate 的 `sh -c` 命令与 Claude prompt 在提交前会扫描默认 blocklist，包括但不限于：

- `rm -rf /`
- `curl|bash` / `wget|sh`
- `chmod 777`
- `mkfs` / `dd of=/dev/`
- fork bomb 等

命中 blocklist 时拒绝执行并记录日志。

### 扩展 blocklist

实现位于 `core/sandbox/src/cli_security.rs` 的 `default_blocklist_patterns()`。后续可接配置文件。

## 相关代码

- `core/sandbox/src/cli_security.rs` — 策略核心
- `nx_api/src/services/claude_cli.rs` — API 层 Claude 调用
- `core/workflow/src/engine.rs` — 工作流引擎 + quality_gate
- `nx_api/src/services/agent_team_service.rs` — 团队任务 PTY/CLI

## Golden Path

默认 **strict**。若 Claude 频繁请求权限导致无法无人值守，可临时设 `trusted` 完成内测，但 AF-00 验收目标是 strict 下可跑通或文档化明确例外。
