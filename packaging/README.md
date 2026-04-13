# NexusFlow 团队分发包

## 文件说明

```
├── NX Dashboard.app          # 桌面应用（双击打开）
├── nx_api                   # 后端服务（通过 start.sh 启动）
├── start.sh                  # 一键启动脚本
└── README.md                # 说明文档
```

## 首次使用安装步骤

### 1. 安装 Claude CLI（只需一次）

打开终端，运行：

```bash
npm install -g @anthropic-ai/claude-code@latest
```

### 2. 启动后端服务

```bash
./start.sh
```

看到 `✓ NexusFlow API 服务就绪` 后，进入下一步。

### 4. 打开桌面应用

双击 `NX Dashboard.app` 即可使用。

---

## 常见问题

**Q: start.sh 提示 "找不到 nx_api 文件"**
A: 确保 nx_api 文件和 start.sh 在同一目录下

**Q: Claude CLI 命令找不到**
A: 运行 `npm install -g @anthropic-ai/claude-code@latest`

**Q: API Key 错误**
A: 确认 ANTHROPIC_API_KEY 设置正确，没有多余的空格

**Q: 启动脚本报错**
A: 用 `chmod +x start.sh` 赋予执行权限

**Q: 团队成员没有 Mac，只有 Windows/Linux**
A: 需要在对应平台分别构建：
- Mac: `npm run tauri build` → .app
- Windows: Windows 上运行同样命令 → .exe
- Linux: Linux 上运行同样命令 → .AppImage

---

## 更新版本

1. 替换 `NX Dashboard.app` 为新版
2. 替换 `nx_api` 为新版
3. 重启服务即可

---

## 自动更新功能

桌面应用会在启动时检查版本更新。

### 发布新版本

当有新版本发布时，你需要：

1. 在你的服务器上创建一个 `version.json` 文件，格式如下：

```json
{
  "latestVersion": "0.2.0",
  "releaseNotes": "新版本功能介绍...",
  "downloadUrl": "https://your-server.com/downloads/NX Dashboard.app"
}
```

2. 确保桌面应用启动时能访问到这个 URL（可以是内网服务器）

3. 团队成员打开应用后，如果发现新版本，会收到弹窗通知

### 版本号规范

使用语义化版本号，如 `0.1.0`、`1.0.0` 等。
应用会比较当前安装版本与 `latestVersion`，来决定是否提示更新。
