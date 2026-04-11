# OpenClaw Doctor (龙虾医生) 完整部署指南

> AI 修复助手与技能管理 - https://xskillhub.com

---

## 目录

1. [功能概览](#1-功能概览)
2. [下载安装](#2-下载安装)
3. [界面介绍](#3-界面介绍)
4. [本地连接](#4-本地连接)
5. [远程连接 (SSH)](#5-远程连接-ssh)
6. [技能管理](#6-技能管理)
7. [模型配置](#7-模型配置)
8. [一键诊断与修复](#8-一键诊断与修复)
9. [常见问题](#9-常见问题)

---

## 1. 功能概览

### 核心功能

| 功能 | 说明 |
|------|------|
| **AI 智能诊断** | 自动检测 OpenClaw 运行状态、配置问题 |
| **智能修复** | 一键修复发现的问题 |
| **本地连接** | 直接连接本地运行的 OpenClaw Gateway |
| **SSH 远程连接** | 通过 SSH 连接到远程服务器的 OpenClaw |
| **技能广场** | 浏览和安装社区技能 |
| **模型配置** | 配置 AI 模型提供商 |
| **自动更新** | 自动检测和安装更新 |

---

## 2. 下载安装

### 官方下载地址

**https://xskillhub.com/download**

### 支持平台

| 平台 | 安装包 | 架构 |
|------|--------|------|
| macOS | .dmg | Apple Silicon (M1-M4) / Intel |
| Windows | .exe | 64-bit |

### 系统要求

**macOS**
- 版本: macOS 10.15 (Catalina) 或更高
- 内存: 4GB+
- 磁盘: 200MB+

**Windows**
- 版本: Windows 10 (64-bit) 或更高
- 处理器: x64
- 内存: 4GB+
- 磁盘: 200MB+

### 安装步骤

1. 下载对应平台的安装包
2. 打开 .dmg 或 .exe 文件
3. 将「龙虾医生」拖入 Applications 文件夹
4. 首次运行需要在「系统设置 > 隐私与安全性」中允许

---

## 3. 界面介绍

### 主界面布局

```
┌─────────────────────────────────────────────────────────────┐
│  [龙虾图标]  龙虾医生                          [−][□][×]  │
├──────────┬────────────────────────────────────────────────┤
│          │                                                │
│  🏠 连接  │     ┌─────────────────────────────────────┐   │
│          │     │   本地连接   │   远程连接   │         │   │
│  ✨ 技能  │     ├─────────────────────────────────────┤   │
│          │     │                                     │   │
│  📦 模型  │     │   连接地址: [http://127.0.0.1:18789]│   │
│          │     │   认证口令:  [••••••••••••••••]       │   │
│  ⚙️ 设置  │     │                                     │   │
│          │     │          [ 连接 ]                    │   │
│          │     │                                     │   │
│          │     │     连接失败                         │   │
│          │     │     Error: connect ECONNREFUSED     │   │
│          │     │              127.0.0.1:18789         │   │
│          │     │                                     │   │
│          │     └─────────────────────────────────────┘   │
│          │                                                │
├──────────┴────────────────────────────────────────────────┤
│  OpenClaw Gateway 诊断专家                                │
│  一键诊断 · 智能修复                                        │
│  https://xskillhub.com                                     │
└─────────────────────────────────────────────────────────────┘
```

### 侧边栏菜单

| 图标 | 菜单 | 功能 |
|------|------|------|
| 🏠 | 连接 | 连接模式选择（本地/远程） |
| ✨ | 技能 | 技能广场和已安装技能 |
| 📦 | 模型 | AI 模型配置 |
| ⚙️ | 设置 | 应用设置 |

---

## 4. 本地连接

### 适用场景

当 OpenClaw Gateway 在你本地机器上运行时使用。

### 配置信息

```
连接地址: http://127.0.0.1:18789
认证口令: (查看 ~/.openclaw/openclaw.json 中的 gateway.auth.token)
```

### 连接步骤

1. 启动「龙虾医生」应用
2. 点击左侧「连接」菜单
3. 选择「本地连接」标签
4. 确认连接地址（默认已填充）
5. 输入认证口令
6. 点击「连接」按钮

### 成功连接后

- 连接按钮变为「已连接 ✅」
- 可以访问技能、模型等功能
- 可进行一键诊断

### 连接失败常见原因

| 错误 | 原因 | 解决方案 |
|------|------|----------|
| ECONNREFUSED | Gateway 未运行 | 在本地启动 `openclaw-gateway &` |
| 认证失败 | Token 错误 | 检查 `~/.openclaw/openclaw.json` 中的 token |
| 连接超时 | 端口错误 | 确认 Gateway 端口号 |

---

## 5. 远程连接 (SSH)

### 适用场景

OpenClaw Gateway 运行在远程 VPS/服务器上。

### 你的 VPS 配置

```
主机: 187.124.168.120
端口: 22
用户名: root
密码: Aa3924894111-
```

### 连接步骤

1. 启动「龙虾医生」应用
2. 点击左侧「连接」菜单
3. 选择「远程连接」标签
4. 填写服务器信息:
   - 主机: `187.124.168.120`
   - 端口: `22`
   - 用户名: `root`
   - 密码: `Aa3924894111-`
5. 点击「连接」按钮

### SSH 连接功能

通过 SSH 远程连接后，龙虾医生可以:

- 自动检测远程服务器的 OpenClaw 状态
- 读取远程配置文件
- 执行诊断命令
- 查看远程日志
- 远程重启 OpenClaw 服务

### 安全说明

SSH 密码仅在连接时使用，不会保存。连接建立后通过 SSH 隧道通信。

---

## 6. 技能管理

### 入口

点击左侧菜单「✨ 技能」

### 功能说明

| 功能 | 说明 |
|------|------|
| 技能广场 | 浏览社区开发的技能 |
| 已安装 | 查看本地已安装的技能 |
| 安装 | 一键安装选中的技能 |
| 更新 | 更新已有技能 |

### 技能目录结构

VPS 上的技能存储在 `/root/.openclaw/agents/`

```
/root/.openclaw/
├── agents/          # Agent 技能
├── flows/           # Flow 技能
├── tasks/           # 任务技能
├── canvas/          # Canvas 技能
├── telegram/        # Telegram 集成
├── qqbot/           # QQ Bot 集成
├── cron/            # 定时任务
└── workspace/       # 工作区
```

### 常用技能命令

```bash
# 查看已安装技能
ls -la /root/.openclaw/agents/

# 查看技能配置
cat /root/.openclaw/openclaw.json | grep -A 5 "skills"
```

---

## 7. 模型配置

### 入口

点击左侧菜单「📦 模型」

### 当前配置 (你的 VPS)

```json
{
  "models": {
    "mode": "merge",
    "providers": {
      "xai": {
        "baseUrl": "https://api.x.ai/v1",
        "api": "openai-completions",
        "models": [
          {
            "id": "grok-4",
            "name": "Grok 4",
            "contextWindow": 131072,
            "maxTokens": 8192
          }
        ]
      }
    }
  },
  "agents": {
    "defaults": {
      "model": {
        "primary": "xai/grok-4",
        "fallbacks": ["openai-codex/gpt-5.1"]
      }
    }
  }
}
```

### 支持的模型提供商

- **xAI** (Grok 4)
- **OpenAI** (GPT-4, GPT-5, Codex)
- **Anthropic** (Claude)
- **Google** (Gemini)
- **Ollama** (本地模型)

### 模型配置项

| 配置项 | 说明 |
|--------|------|
| Provider | 模型提供商 |
| API Key | 服务商的 API 密钥 |
| Base URL | API 端点地址 |
| Model ID | 具体模型标识 |
| Context Window | 上下文窗口大小 |
| Max Tokens | 最大输出 token |

---

## 8. 一键诊断与修复

### 功能说明

点击底部「一键诊断 · 智能修复」按钮

### 诊断项目

| 检查项 | 说明 |
|--------|------|
| Gateway 状态 | 检查 OpenClaw Gateway 是否运行 |
| 端口监听 | 确认端口正常监听 |
| 认证配置 | 验证 Token 是否有效 |
| 模型配置 | 检查模型是否正确配置 |
| 技能状态 | 检查技能是否完整安装 |
| 网络连接 | 测试对外连接是否正常 |
| 磁盘空间 | 检查磁盘空间是否充足 |
| 内存使用 | 检查内存使用情况 |

### 修复能力

- 自动重启 Gateway
- 重新生成认证 Token
- 修复配置文件错误
- 清理日志文件
- 重新安装损坏的技能

---

## 9. 常见问题

### Q1: 本地连接失败 ECONNREFUSED

**原因**: OpenClaw Gateway 未在本地运行

**解决方案**:
```bash
# 启动本地 Gateway
openclaw-gateway &

# 或重启
pkill openclaw-gateway
openclaw-gateway &
```

### Q2: 远程连接超时

**原因**: VPS 防火墙阻止或网络问题

**解决方案**:
```bash
# 在 VPS 上检查防火墙
ufw status

# 开放 SSH 端口
ufw allow 22

# 开放 Gateway 端口（如果需要）
ufw allow 18789
```

### Q3: 认证 Token 无效

**原因**: Token 与服务器配置不匹配

**解决方案**:
```bash
# 查看当前 Token
cat /root/.openclaw/openclaw.json | grep -A 3 "auth"
```

### Q4: 技能无法安装

**原因**: 网络问题或权限不足

**解决方案**:
```bash
# 检查权限
ls -la /root/.openclaw/agents/

# 手动安装技能
cd /root/.openclaw/agents/
git clone <skill-repo>
```

### Q5: 模型调用失败

**原因**: API Key 错误或余额不足

**解决方案**:
```bash
# 检查模型配置
cat /root/.openclaw/openclaw.json | grep -A 10 "models"
```

### Q6: 如何查看详细日志

```bash
# 实时查看日志
tail -f /root/.openclaw/logs/*.log

# 查看命令日志
cat /root/.openclaw/logs/commands.log

# 查看配置审计日志
cat /root/.openclaw/logs/config-audit.jsonl
```

### Q7: 如何完全重启 OpenClaw

```bash
# 停止所有进程
pkill openclaw-gateway

# 等待 2 秒
sleep 2

# 重新启动
openclaw-gateway &

# 验证运行
ps aux | grep openclaw
```

---

## 联系方式

- 官网: https://xskillhub.com
- 技术支持: 通过 Telegram 联系

---

*文档更新时间: 2026-04-09*
