# NexusFlow 团队分发包

## 文件说明

```
├── NX Dashboard.app    # 桌面应用（双击打开）
└── README.md          # 说明文档
```

## 首次使用安装步骤

### 1. 安装 Claude CLI（只需一次）

打开终端，运行：

```bash
npm install -g @anthropic-ai/claude-code@latest
```

### 2. 打开桌面应用

双击 `NX Dashboard.app` 即可使用。

后端服务会自动启动，无需手动运行任何脚本。

---

## 常见问题

**Q: 提示"应用已损坏"无法打开**
A: 这是 macOS 安全机制。运行以下命令解除限制：
```bash
xattr -d -r com.apple.quarantine "/path/to/NX Dashboard.app"
```

**Q: Claude CLI 命令找不到**
A: 运行 `npm install -g @anthropic-ai/claude-code@latest`

**Q: 技能库为空**
A: 技能已内置在应用包中。

**Q: 团队成员没有 Mac，只有 Windows/Linux**
A: 需要在对应平台分别构建：
- Mac: `npm run tauri build` → .app
- Windows: Windows 上运行同样命令 → .exe
- Linux: Linux 上运行同样命令 → .AppImage

---

## 更新版本

1. 替换 `NX Dashboard.app` 为新版
2. 直接打开即可
