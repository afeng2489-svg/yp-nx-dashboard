# NexusFlow - 多智能体开发框架设计

## 概述

**项目名称**：NexusFlow

**目标**：设计并实现一个比 Claude-Code-Workflow 更优秀的的多智能体开发框架。

## 相对 Claude-Code-Workflow 的核心改进

| 特性 | Claude-Code-Workflow | NexusFlow |
|---------|---------------------|-----------|
| 性能 | Node.js | Rust（快 10-100 倍）|
| 多 AI | 依赖 MCP | 统一抽象层 |
| 代码执行 | CLI 工具 | 原生沙箱 |
| 代码理解 | 外部依赖 | 内置 Tree-sitter |
| 工作流定义 | JSON/YAML | 直观 YAML DSL |
| 状态管理 | 无 | SQLite + Git |
| 类型安全 | JavaScript | 完整 Rust 安全保障 |

## 技术栈

- **核心**：Rust（异步、内存安全、零 GC）
- **Web**：Axum（异步 HTTP 框架）
- **数据库**：SQLite + PostgreSQL
- **AI**：统一提供商抽象
- **代码解析**：Tree-sitter
- **沙箱**：seccomp-bpf + cgroups
- **前端**：React + TypeScript

## 核心模块

1. **AI Bridge**（`core/ai/`）
   - 统一 AI 提供商接口
   - 支持 Anthropic、OpenAI、Google、Ollama
   - 自动模型路由

2. **工作流引擎**（`core/workflow/`）
   - 基于 YAML 的工作流定义
   - 并行/串行执行
   - 事件驱动的智能体编排

3. **代码智能**（`core/code-intel/`）
   - Tree-sitter 多语言解析
   - 符号提取
   - 引用查找

4. **沙箱执行器**（`core/sandbox/`）
   - 进程隔离
   - 资源限制
   - 安全代码执行

5. **工作区**（`core/workspace/`）
   - SQLite 持久化
   - Git 集成
   - 快照管理

6. **测试生成器**（`core/testing/`）
   - 上下文感知测试生成
   - 覆盖率跟踪

## 目录结构

```
nexusflow/
├── nx_cli/                    # CLI 入口点
├── core/
│   ├── ai/                   # AI 提供商抽象层
│   ├── code-intel/           # 代码理解
│   ├── sandbox/              # 安全执行
│   ├── workflow/             # 工作流引擎
│   ├── workspace/             # 工作区持久化
│   ├── testing/               # 测试生成
│   └── search/                # 语义搜索
├── api/                      # HTTP API
├── dashboard/                # React 仪表板
└── config/                  # 配置和工作流
```

## 实现状态

- [x] 项目结构
- [x] AI 提供商抽象层（Anthropic、OpenAI、Google、Ollama）
- [x] 工作流引擎核心（解析器、状态、事件、引擎）
- [x] 代码智能（Tree-sitter 封装、符号、引用、索引）
- [x] 沙箱执行器（进程隔离、资源限制）
- [x] CLI 入口点和命令

## 下一步计划

- [ ] SQLite 工作区持久化
- [ ] 基于 Axum 的 API 服务器
- [ ] React 仪表板
- [ ] 测试生成器集成
- [ ] 语义搜索索引
- [ ] 插件系统

## 与 Claude-Code-Workflow 对比

| 方面 | Claude-Code-Workflow | NexusFlow |
|--------|---------------------|-----------|
| 语言 | TypeScript | Rust |
| 运行时 | Node.js | 原生 |
| 内存 | >200MB | ~20MB |
| 启动时间 | >2s | <100ms |
| AI 提供商 | MCP 协议 | 原生 SDK |
| 代码解析 | 外部依赖 | 内置 |
| 沙箱 | CLI 封装 | 原生支持 |

## 为什么选择 Rust？

- **性能**：比 Node.js 快 10-100 倍
- **内存安全**：无 GC 停顿，内存安全
- **并发**：内置 async/await
- **类型安全**：编译时保证
- **二进制分发**：单一可执行文件

## 设计原则

1. **简单优先**：KISS，避免过度工程化
2. **性能**：零成本抽象
3. **安全**：默认沙箱化
4. **可扩展性**：基于插件的架构
5. **开发者体验**：直观的 CLI 和 API
