# Claude CLI 集成方案

## 概述

通过调用本地 Claude Code CLI 实现 AI 模型调用，Claude Switch（或后续的 web UI）切换模型后，Claude CLI 会自动使用新配置的模型。

## 当前架构

```
┌─────────────────────────────────────────────────────────────┐
│                      AgentTeamService                       │
│                   execute_role_ai()                        │
└─────────────────────────┬─────────────────────────────────┘
                          │
                          ▼
              ┌───────────────────────────┐
              │   tokio::process::Command │
              │   ("claude", ["-p", ...]) │
              └───────────────────────────┘
                          │
                          ▼
              ┌───────────────────────────┐
              │     Claude Code CLI         │
              │   (本地安装的 CLI 工具)     │
              └───────────────────────────┘
                          │
                          ▼
              ┌───────────────────────────┐
              │   Claude CLI 配置文件       │
              │  ~/.claude/settings.json   │
              │  (Claude Switch 修改此处)  │
              └───────────────────────────┘
```

## 已更新的文件

| 文件 | 修改内容 |
|------|----------|
| `nx_api/src/services/claude_cli.rs` | **新建** - 共享的 Claude CLI 调用函数 |
| `nx_api/src/routes/ai_config.rs` | `execute_cli` 和 `chat_with_selected` 端点改用 Claude CLI |
| `nx_api/src/services/agent_team_service.rs` | `execute_role_ai` 改用 Claude CLI |
| `core/workflow/src/engine.rs` | `execute_agent` 改用 Claude CLI，移除 AIProviderRegistry 依赖 |
| `nx_api/src/services/test_generator.rs` | `call_ai` 改用 Claude CLI |
| `nx_cli/src/commands.rs` | `run_agent` 和 `run_workflow` 改用 Claude CLI |

## 共享函数

**文件**: `nx_api/src/services/claude_cli.rs`

```rust
// 调用 Claude CLI
pub async fn call_claude_cli(prompt: &str) -> Result<String, String>

// 将 ChatMessage 列表转换为 prompt
pub fn messages_to_prompt(messages: &[ChatMessage]) -> String
```

## 流程

1. Claude Switch 切换模型 → 修改 `~/.claude/settings.json`
2. Rust App 调用 `claude -p "<prompt>"`
3. Claude CLI 读取本地配置
4. Claude CLI 使用当前配置的模型处理请求
5. 返回结果给 Rust App

## 架构变化

### 之前
```
AgentTeamService → AIModelManager → ClaudeSwitchProvider → MiniMax API
```

### 现在
```
AgentTeamService → Claude CLI → Claude CLI 配置 → MiniMax 等后端
```

## 后续扩展

### Web UI 配置保存到 Claude CLI 配置

将 AI Provider 页面配置直接写入 Claude CLI 配置文件：

```rust
// 页面保存时调用
async fn sync_provider_to_claude_config(provider: &AIProvider) -> Result<()> {
    let config_path = dirs::config_dir()
        .join("claude")
        .join("settings.json");

    let mut settings = load_settings(&config_path)?;
    settings.set("api_key", &provider.api_key);
    settings.set("base_url", &provider.base_url);
    settings.set("model", &provider.default_model);
    save_settings(&config_path, &settings)?;

    Ok(())
}
```

### 配置文件位置

Claude Code CLI 配置文件通常位于：
- macOS: `~/.claude/settings.json`
- Linux: `~/.claude/settings.json`
- Windows: `%USERPROFILE%\.claude\settings.json`

## 依赖

- Claude Code CLI 已安装并可在 PATH 中访问
- Claude CLI 配置正确（API Key 等）
