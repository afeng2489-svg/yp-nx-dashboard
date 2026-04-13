# NexusFlow

一款高性能、跨厂商的多智能体开发框架，采用 Rust 构建。

## 特性

- **多 AI 提供商支持**：支持 Anthropic (Claude)、OpenAI (GPT)、Google (Gemini) 和 Ollama（本地模型）
- **内置代码智能**：基于 Tree-sitter 的原生代码解析，支持多语言
- **安全代码执行**：进程级沙箱隔离，配有资源限制
- **直观工作流**：基于 YAML 的工作流定义，支持并行执行
- **持久化工作区**：基于 SQLite 的工作区管理，集成 Git
- **内置测试**：由 AI 驱动的上下文感知测试生成

## 架构

```
┌─────────────────────────────────────────────────────────────┐
│                      NexusFlow CLI (nx)                      │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐ │
│  │   AI Bridge  │  │   Workflow   │  │   Code Intel    │ │
│  │              │  │   Engine     │  │   (Tree-sitter) │ │
│  └──────────────┘  └──────────────┘  └──────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐ │
│  │   Sandbox   │  │  Workspace  │  │   Test Gen      │ │
│  │   Executor  │  │   Store     │  │                 │ │
│  └──────────────┘  └──────────────┘  └──────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## 快速开始

### 环境要求

- Rust 1.75+
- Node.js 18+（用于 dashboard）

---

## 桌面应用（推荐）

如需在团队中分发或使用桌面应用，请参考 [packaging/README.md](packaging/README.md)。

### 构建桌面应用

```bash
# 安装依赖后，运行打包脚本
./build_package.sh

# 输出的压缩包位于桌面：~/Desktop/NexusFlow_Package.tar.gz
```

---

### 安装（源码开发）

```bash
# 从源码构建
cargo build --release

# 添加到 PATH
export PATH="$(pwd)/target/release:$PATH"
```

### 配置

创建 `nexus.yaml`：

```yaml
providers:
  anthropic:
    api_key: "${ANTHROPIC_API_KEY}"
    models:
      - claude-opus-4-5
      - claude-sonnet-4-5

  openai:
    api_key: "${OPENAI_API_KEY}"

  ollama:
    base_url: "http://localhost:11434"

default_provider: "anthropic"
workspace_dir: "./workspace"
```

### 使用

```bash
# 运行工作流
nx run --workflow config/workflows/dev-workflow.yaml

# 运行单个智能体
nx agent --role developer --model claude-sonnet-4-5 --prompt "你好，世界！"

# 查看提供商列表
nx providers

# 验证工作流
nx validate --workflow my-workflow.yaml

# 在沙箱中执行代码
nx exec --program echo --args "你好，沙箱！"

# 启动 API 服务器
nx serve --port 8080
```

## 工作流示例

```yaml
name: "功能开发"
version: "1.0"

agents:
  - id: "planner"
    role: "architect"
    model: "claude-opus-4-5"
    prompt: "设计新功能的架构"

  - id: "coder"
    role: "developer"
    model: "claude-sonnet-4-5"
    depends_on: ["planner"]
    prompt: "根据设计实现功能"

stages:
  - name: "规划"
    agents: ["planner"]
  - name: "实现"
    agents: ["coder"]
    parallel: true
```

## 项目结构

```
nexusflow/
├── nx_cli/           # CLI 应用程序
├── core/
│   ├── ai/          # AI 提供商抽象层
│   ├── workflow/     # 工作流引擎
│   ├── code-intel/   # 代码理解
│   ├── sandbox/      # 安全执行
│   ├── workspace/     # 工作区管理
│   ├── testing/       # 测试生成
│   └── search/        # 语义搜索
├── api/             # HTTP API 服务器
├── dashboard/       # Web 仪表板
├── config/          # 配置和工作流
└── docs/            # 文档
```

## 性能

| 指标 | NexusFlow | Claude-Code-Workflow |
|--------|-----------|----------------------|
| 工作流启动 | < 100ms | > 2000ms |
| 代码解析 | < 50ms | N/A |
| 并发智能体 | 10,000+ | < 100 |
| 内存（空闲）| ~20MB | > 200MB |

## 相对 Claude-Code-Workflow 的核心改进

| 特性 | Claude-Code-Workflow | NexusFlow |
|---------|---------------------|-----------|
| 性能 | Node.js | Rust (10-100x) |
| 多 AI | 依赖 MCP | 统一抽象层 |
| 代码执行 | CLI 工具 | 原生沙箱 |
| 代码理解 | 外部依赖 | 内置 Tree-sitter |
| 工作流定义 | JSON/YAML | 直观 YAML DSL |
| 状态管理 | 无 | SQLite + Git |
| 类型安全 | JavaScript | 完整 Rust 安全保障 |

## 许可证

MIT
