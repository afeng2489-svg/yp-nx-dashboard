# NexusFlow 故障排查（AF-05）

## 安装与启动

| 问题 | 处理 |
|------|------|
| 应用打不开 / 闪退 | 查看 `%TEMP%\nx_startup.log`（Windows）或 Console.app（macOS） |
| 「后端服务启动失败」 | 重启应用；开发模式运行 `cargo build --bin nx_api` 后 `npm run tauri:dev` |
| 端口 8080 被占用 | 关闭其他 nx_api 实例，或设置 `NEXUS_API_PORT` |

## Claude CLI

| 问题 | 处理 |
|------|------|
| 工厂台提示「未绑定 CLI」 | **设置 → AI** → 检测/手动指定 `claude` 路径 |
| CLI 找不到 | `npm install -g @anthropic-ai/claude-code` |
| 权限过严 agent 无法改文件 | 设置 permissions mode；开发可用 `NEXUS_PERMISSIONS_MODE=trusted` |

## 工厂台 / Golden Path

| 问题 | 处理 |
|------|------|
| `from-template` 405 | 重建 `nx_api` 并重启 Tauri（见 [GOLDEN-PATH.md](./GOLDEN-PATH.md)） |
| Run 卡住 | 看底栏 WS 状态；断线应显示「轮询中」 |
| 无 diff | 确认顶栏已选**工作区**；CLI 在该目录有写权限 |
| 审批不出现 | 确认 `solo-dev` 含「交付审批」stage |

## 构建 macOS 包

```bash
cd nx_dashboard
npm run tauri:build
# 产物：src-tauri/target/release/bundle/macos/*.app / *.dmg
```

详见 [packaging/README.md](../packaging/README.md)。

## 获取帮助

- Golden Path 步骤：[GOLDEN-PATH.md](./GOLDEN-PATH.md)
- WS 行为：[ws-reliability.md](./ws-reliability.md)
- 内测反馈模板：[dogfood/gate-2-results.md](./dogfood/gate-2-results.md)
