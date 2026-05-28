# NexusFlow 桌面版发布指南（AF-05）

## 前置

- macOS（Phase1 仅 macOS）
- Rust 1.75+、Node 18+
- Claude Code CLI（用户本机安装，不打进包）

## 本地 Release 构建

```bash
cd nx_dashboard
npm ci
npm run tauri:build
```

产物：`nx_dashboard/src-tauri/target/release/bundle/dmg/` 或 `.app`。

Sidecar `nx_api` 由 Tauri 配置打包；开发时 `npm run tauri:dev` 会启动 Vite **1420** + 后端 **8080**。

## 版本号

- 应用版本：`nx_dashboard/package.json` → `version`
- 开发更新检测：`nx_dashboard/public/version.json`（生产可换 GitHub Releases API）

## GitHub Release（推荐）

1. 打 tag：`git tag v0.x.y && git push origin v0.x.y`
2. CI：`.github/workflows/build.yml` 构建产物（若已配置）
3. 上传 `.dmg` / `.app.zip` 到 Release Assets
4. 更新 `public/version.json` 的 `latestVersion` 与 `releaseNotes`

## 首次启动向导

新用户路径（`OnboardingWizard`）：

1. 检测 Claude CLI
2. 选择工作区文件夹
3. 从模板创建 Solo 团队
4. 跳转 `/factory`

跳过：DevTools 执行 `localStorage.setItem('nexus-onboarding-v1','done'); location.reload()`

## 分发检查清单

- [ ] Release 包在干净 macOS 可安装
- [ ] 首次向导 10 分钟内到达 Console
- [ ] Golden Path 可跑通（见 [GOLDEN-PATH.md](./GOLDEN-PATH.md)）
- [ ] [TROUBLESHOOTING.md](./TROUBLESHOOTING.md) 链到常见问题

## 朋友测 / 内测

- GATE-2 模板：[dogfood/gate-2-results.md](./dogfood/gate-2-results.md)
- AF-06 内测 kit：[dogfood/af-06-beta-kit.md](./dogfood/af-06-beta-kit.md)
