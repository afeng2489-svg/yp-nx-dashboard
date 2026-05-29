# TeamFlow

> AI 驱动的团队协作与工作流自动化桌面应用

TeamFlow 是一个基于 Tauri 构建的跨平台桌面应用，集成了 AI 工作流编排、团队协作、任务管理等功能，旨在提升团队开发效率。

## 核心功能

### 工作流编排

- **可视化工作流设计** - 使用 @xyflow/react 构建的流程图编辑器
- **工作流执行引擎** - 支持多阶段、多步骤的工作流执行
- **执行记录与监控** - 实时查看工作流执行状态和结果

### AI 能力

- **AI 工作流** - 集成 Claude 等 AI 模型的智能工作流
- **技能系统** - 可复用的 AI 技能库
- **RAG 知识库** - 基于文档向量检索的知识增强（开发中）

### 团队协作

- **团队管理** - 创建和管理团队成员
- **角色权限** - 灵活的角色配置系统
- **群聊功能** - 团队内部即时通讯
- **项目管理** - 项目创建、跟踪和管理

### 开发工具

- **内置终端** - 基于 xterm.js 的完整终端模拟器
- **代码编辑器** - 集成 Monaco Editor
- **内置浏览器** - 用于测试和预览
- **Sprint 看板** - 敏捷开发看板管理

### 成本管理

- **Token 成本统计** - AI 模型使用成本追踪
- **用量分析** - 详细的使用情况报表

## 技术栈

### 前端

| 技术           | 用途         |
| -------------- | ------------ |
| React 18       | UI 框架      |
| TypeScript     | 类型安全     |
| Vite           | 构建工具     |
| Tailwind CSS   | 样式系统     |
| Radix UI       | 无样式组件库 |
| Zustand        | 状态管理     |
| TanStack Query | 数据获取     |
| Recharts       | 图表可视化   |
| Monaco Editor  | 代码编辑器   |
| @xyflow/react  | 流程图编辑   |
| xterm.js       | 终端模拟     |
| Motion         | 动画效果     |

### 后端

| 技术      | 用途       |
| --------- | ---------- |
| Rust      | 系统语言   |
| Axum      | Web 框架   |
| SQLite    | 本地数据库 |
| WebSocket | 实时通信   |
| Tokio     | 异步运行时 |

### 桌面框架

| 技术      | 用途           |
| --------- | -------------- |
| Tauri 2.0 | 跨平台桌面框架 |

## 项目结构

```
yp-nx-dashboard/
├── nx_dashboard/          # 前端应用
│   ├── src/
│   │   ├── pages/         # 页面组件
│   │   ├── components/    # 通用组件
│   │   │   ├── canvas/    # 画布组件
│   │   │   ├── team/      # 团队相关
│   │   │   ├── workflow/  # 工作流组件
│   │   │   ├── terminal/  # 终端组件
│   │   │   └── ui/        # 基础 UI 组件
│   │   ├── stores/        # Zustand 状态管理
│   │   ├── api/           # API 接口
│   │   └── lib/           # 工具函数
│   └── src-tauri/         # Tauri 配置
│       └── binaries/      # 后端二进制文件
│
└── nx_api/                # 后端服务
    └── src/
        ├── routes/        # API 路由
        ├── services/      # 业务逻辑
        ├── models/        # 数据模型
        └── ws/            # WebSocket 处理
```

## 快速开始

### 环境要求

| 工具       | 最低版本 | 检查命令           |
| ---------- | -------- | ------------------ |
| Node.js    | >= 18    | `node -v`           |
| Rust       | >= 1.70  | `rustc --version`   |
| npm        | >= 9     | `npm -v`            |

> macOS 用户需额外安装 Xcode Command Line Tools：`xcode-select --install`

### 第一步：克隆仓库

```bash
git clone <repo-url>
cd yp-nx-dashboard
```

### 第二步：安装依赖

```bash
# 进入前端目录
cd nx_dashboard

# 安装前端依赖
npm install

# Tauri CLI 会通过 package.json 的 devDependencies 自动安装
# Rust 后端依赖会在首次构建时通过 Cargo 自动拉取
```

### 第三步：配置环境变量（可选）

项目已内置 `.env.development` 用于开发模式，默认配置可直接使用。如需自定义：

```bash
# 复制并编辑开发环境配置
cp .env.development .env.development.local

# 按需编辑以下变量：
#   VITE_API_BASE_URL  - 后端 API 地址（默认 localhost:8080）
#   VITE_WS_BASE_URL   - WebSocket 地址（默认 ws://localhost:8080）
```

后端可通过环境变量调整数据库路径和端口：

```bash
export NEXUS_DB_PATH=~/.teamflow/nexus.db   # 数据库文件路径
export NEXUS_API_PORT=8080                   # API 服务端口
export RUST_LOG=info                         # 日志级别
```

### 第四步：启动开发模式

```bash
# Tauri 开发模式——同时启动前端 dev server 和后端 Rust 服务
npm run tauri:dev
```

开发模式下：
- 前端运行在 Vite dev server（热更新）
- 后端 Rust 服务通过 Tauri sidecar 启动
- 修改后端代码后需重新编译：`npm run build:backend:dev`

### 第五步：验证安装

启动后应能看到 TeamFlow 桌面窗口。如果遇到问题，检查：

- Node.js 和 Rust 版本是否满足要求
- 前端依赖是否完整安装（`npm install` 无报错）
- macOS 用户是否安装了 Xcode Command Line Tools

### 其他常用命令

```bash
# 仅启动前端开发服务器（不启动 Tauri 窗口）
npm run dev

# 仅编译后端（开发模式，含调试信息）
npm run build:backend:dev

# 构建生产版本
npm run tauri:build

# 代码格式化
npm run format

# 格式化检查
npm run format:check

# 运行测试
npm run test

# ESLint 检查
npm run lint
```

## 页面路由

| 路径            | 页面         | 说明       |
| --------------- | ------------ | ---------- |
| `/`             | Dashboard    | 仪表盘首页 |
| `/workflows`    | Workflows    | 工作流管理 |
| `/executions`   | Executions   | 执行记录   |
| `/terminal`     | Terminal     | 终端       |
| `/editor`       | Editor       | 代码编辑器 |
| `/tasks`        | Tasks        | 任务管理   |
| `/skills`       | Skills       | 技能库     |
| `/teams`        | Teams        | 团队管理   |
| `/roles`        | Roles        | 角色管理   |
| `/projects`     | Projects     | 项目管理   |
| `/group-chat`   | Group Chat   | 群聊       |
| `/browser`      | Browser      | 内置浏览器 |
| `/settings`     | Settings     | 应用设置   |
| `/ai-settings`  | AI Settings  | AI 配置    |
| `/cost`         | Cost         | 成本统计   |
| `/canvas`       | Canvas       | 画布       |
| `/sprint-board` | Sprint Board | 看板       |
| `/search`       | Search       | 全局搜索   |
| `/templates`    | Templates    | 模板库     |

## 配置

### 环境变量

| 变量名           | 说明           | 默认值                 |
| ---------------- | -------------- | ---------------------- |
| `NEXUS_DB_PATH`  | 数据库文件路径 | `~/.teamflow/nexus.db` |
| `NEXUS_API_PORT` | API 服务端口   | `8080`                 |
| `RUST_LOG`       | 日志级别       | `info`                 |

### 数据存储

应用数据存储在用户目录下：

- macOS: `~/Library/Application Support/com.nx.dashboard/`
- Windows: `%APPDATA%\com.nx.dashboard\`
- Linux: `~/.local/share/com.nx.dashboard/`

## 开发指南

### 代码规范

- 使用 Prettier 格式化代码
- 遵循 TypeScript 严格模式
- 组件使用函数式组件 + Hooks
- 状态管理使用 Zustand
- 数据获取使用 TanStack Query

### Git 提交规范

```
<type>: <description>

类型：feat, fix, refactor, docs, test, chore, perf, ci
```

## 许可证

私有项目，未经授权禁止使用。

---

**TeamFlow** - 让 AI 赋能团队协作
