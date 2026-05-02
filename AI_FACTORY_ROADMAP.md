# NexusFlow — AI 软件工厂

> **产品定位**：一个普通开发者，用这个软件就能完成一个完整软件产品的设计、开发、测试、监控。
> 你描述需求，AI 生产线自动分解→并行执行→自动测试→自动修复→交付可运行代码。
>
> **当前阶段**：单机单用户，不考虑多租户。
> **核心原则**：所有改动都让 AI 自己实现，模型无关（claude / gpt / qwen / glm 都能跑）。

---

## 📊 实时进度

> 详细进度见 [docs/PROGRESS.md](docs/PROGRESS.md) 和 [docs/progress.json](docs/progress.json)。
> **AI 恢复指令**：找到第一个 ⬜ pending 的 sprint，读对应 `docs/sprints/*.yaml`，开始执行。

### Phase 0 — 地基（必须先做）
| Sprint | 标题 | 状态 | 估时 |
|--------|------|------|------|
| v0.0.1 | 修复关键 Bug（16处expect崩溃+Mutex混用+数据静默损坏） | ✅ done | 11h |
| v0.0.2 | A2UI路由挂载 + 统一API格式 + Docker部署 | ✅ done | 12h |
| v0.0.3 | 接入 core/orchestrator（接线，不是新建，6h） | ✅ done | 6h |

### Phase 1 — 生产线核心（这才是"AI工厂"的本质）
| Sprint | 标题 | 状态 | 估时 |
|--------|------|------|------|
| v1.1 | Pipeline真正跑通（dispatch→执行→完成→推进） | ⬜ pending | 16h |
| v1.2 | 断点续跑（Checkpoint真正写入+恢复） | ⬜ pending | 12h |
| v1.3 | 质量门自动化（执行→测试→失败重试） | ⬜ pending | 20h |
| v1.4 | 可观测性看板（进度+成本+产物） | ⬜ pending | 20h |

### Phase 2 — 用户体验
| Sprint | 标题 | 状态 | 估时 |
|--------|------|------|------|
| v2.1 | 团队对话体验重构（CLI优先+流式输出） | ⬜ pending | 16h |
| v2.2 | 产物管理完整（预览+下载，后端90%完成） | ⬜ pending | 20h |
| v2.3 | 项目状态感知（AI知道项目做到哪了） | ⬜ pending | 24h |

### Phase 3 — 功能扩展
| Sprint | 标题 | 状态 | 估时 |
|--------|------|------|------|
| v3.1 | Git集成（每stage自动commit+失败回滚） | ⬜ pending | 16h |
| v3.2 | 触发器系统（Cron+Webhook+链式，core已有实现） | ⬜ pending | 8h |
| v3.3 | Token/Cost监控 | ⬜ pending | 16h |
| v3.4 | RAG知识库（文档上传+向量检索+注入prompt） | ⬜ pending | 32h |

### Phase 4 — 智能化升级
| Sprint | 标题 | 状态 | 估时 |
|--------|------|------|------|
| v4.1 | 多模型路由（按复杂度自动选模型，降成本60%） | ⬜ pending | 24h |
| v4.2 | 失败自愈（失败→重试→换模型→回滚→通知） | ⬜ pending | 20h |
| v4.3 | 低代码可视化画布（拖拽搭建AI流水线） | ⬜ pending | 80h |
| v4.4 | 浏览器自动化验证（AI写完UI自动打开浏览器验证） | ⬜ pending | 24h |
| v4.5 | 用户需求分解UI（界面拆需求→AI接棒执行） | ⬜ pending | 20h |
| v4.6 | 多模态工具链（图片/视频/设计稿生成） | ⬜ pending | 32h |

**图例**：⬜ pending | 🔄 in_progress | ✅ completed | ❌ failed

### v0.1.S1 剩余任务

| ID | 任务 | 估时 | 优先级 |
|----|------|------|--------|
| T1 | 文件预览（Markdown渲染+代码高亮） | 8h | P0 |
| T2 | 二进制判断+大文件降级展示 | 4h | P0 |
| T3 | 执行列表产物数量badge | 4h | P1 |
---

## 📑 目录

- [产品愿景](#-产品愿景)
- [架构决策](#-架构决策)
- [AI 自改代码机制](#-ai-自改代码机制)
- [Phase 0 — 地基](#-phase-0--地基)
- [Phase 1 — 生产线核心](#-phase-1--生产线核心)
- [Phase 2 — 用户体验](#-phase-2--用户体验)
- [Phase 3 — 功能扩展](#-phase-3--功能扩展)
- [成功指标](#-成功指标)

---

## 🎯 产品愿景

**一个普通开发者，用这个软件就能完成一个完整软件产品的设计、开发、测试、监控。**

你描述需求 → AI 生产线自动分解 → 并行执行 → 自动测试 → 自动修复 → 交付可运行代码。
每个步骤可见、可控、可回滚。中断后能从断点继续，不从零开始。

### 现有能力（真实可用）

| 模块 | 状态 | 说明 |
|------|------|------|
| PTY 终端面板 | ✅ | xterm.js 实时渲染，真实 PTY |
| 工作流引擎 | ✅ | stage 顺序执行、条件跳转、重试 |
| 产物追踪 | ✅ | 每个 stage 前后 diff，后端完整 |
| AI 配置 | ✅ | 多模型、API key 管理 |
| Pipeline 数据结构 | ✅ | phase/step/状态机 |
| Checkpoint 表 | ✅ | 结构存在，未写入 |
| core/orchestrator | ✅ | 完整调度器，未接入 nx_api |

### 关键缺口（阻断"生产线"定位）

| 缺口 | 影响 |
|------|------|
| Pipeline dispatch 完成后不回调 | 进度条永远不动 |
| Checkpoint 从未写入 | 中断后无法续跑 |
| 无质量门 | AI 写完代码不知道对不对 |
| 团队对话输出质量差 | PTY 字节流抠文本，体验极差 |
| Phase 1 | **v0.x 自用工厂** | 2-6 周 | 团队 dogfood 可用 |
| Phase 2 | **v1.x 第一个客户** | 4-12 周 | 单租户私有部署 |
| Phase 3 | **v2.x 多租户 SaaS** | 12-24 周 | 商业化运营 |
---

## 🏗️ 架构决策

```
TaskPipeline（大脑）→ PTY/CLI（执行手）→ Git（审计层）
```

| 决策 | 原因 |
|------|------|
| PTY 只用于展示，CLI 用于提取文本 | PTY 字节流不适合提取干净文本 |
| SQLite 单机，不迁 PostgreSQL | 单用户场景够用，迁移成本高 |
| core/orchestrator 接线不新建 | 调度器已完整实现，只差接入 |
| Phase 0 必须先做 | 16处崩溃点不修，后面一切不稳定 |

---

## 🤖 AI 自改代码机制

整套方案能跑通的前提是先让当前系统能跑通"AI 改 AI"。这是 **Sprint #0**。

### Sprint #0：建立"AI 改代码"标准流程（3 天）

**目标**：定义一个工作流模板，输入是任务卡片，AI 自动完成"研究 → 设计 → 编码 → 测试 → 提交"。

#### 任务卡片模板（每个 sprint 都用这个格式）

```yaml
sprint_id: v0.1.S1
title: 产物管理 — 让用户看到工作流跑出了什么
objective: |
  跑完一次 workflow execution 后，前端能看到该次执行新增/修改的所有文件，
  支持预览、下载、差异对比。

inputs:
  - codebase_path: /path/to/yp-nx-dashboard
  - existing_modules: [workflow_engine, executions_table]
  - constraints:
      - 不破坏现有 workflow 执行流程
      - 后端用 Rust 加 module，前端 React 加 page tab

deliverables:
  - backend:
      - file: nx_api/src/services/artifact_tracker.rs (new)
      - functions: [snapshot_workdir, diff_workdir, list_artifacts]
      - api_routes:
          - GET /api/v1/executions/:id/artifacts
          - GET /api/v1/executions/:id/artifacts/:path/preview
  - frontend:
      - component: ExecutionsPage 加 "产物" tab
      - feature: 文件列表 + Markdown/code 预览 + 一键下载 zip
  - tests:
      - rust unit: artifact_tracker (3 cases)
      - frontend smoke: tab 渲染 + 列表显示

acceptance:
  - 跑一次包含写文件的工作流后，产物 tab 显示该次执行所有文件
  - 文件多于 50 时分页加载
  - 二进制文件不预览，只显示大小

quality_gates:
  - cargo test --bin nx_api 全过
  - cargo clippy 无 warning
  - npx tsc --noEmit 无 error
  - 手动 smoke：跑一次 demo workflow 验证

rollback: |
  本次改动只新增文件 + 在 ExecutionsPage 加一个 tab。
  失败回滚: git revert <commit>
```

#### 驱动这个卡片的元工作流

```yaml
name: ai-self-improvement-v1
agents:
  - id: planner
    role: 架构师
    model: ${MODEL_PLANNER}
    prompt: |
      你是架构师。根据任务卡片，输出实现计划：
      - 涉及哪些文件
      - 每个文件改什么
      - 测试用例怎么写
      输出 plan.md

  - id: backend_dev
    role: Rust 后端
    model: ${MODEL_DEV}
    prompt: |
      根据 {planner_output}，写后端代码。
      要求：通过 cargo build 和 cargo clippy。

  - id: frontend_dev
    role: 前端
    model: ${MODEL_DEV}
    prompt: |
      根据 {planner_output}，写前端代码。
      要求：通过 tsc + prettier。

  - id: tester
    role: 测试
    model: ${MODEL_TESTER}
    prompt: |
      跑测试，输出报告。如果失败，反馈给 dev agent。

  - id: reviewer
    role: Code Reviewer
    model: ${MODEL_REVIEWER}
    prompt: |
      Review 所有改动。检查安全/性能/是否符合 acceptance。
      失败 reject 回 dev。

stages:
  - name: 计划
    agents: [planner]
  - name: 并行实现
    agents: [backend_dev, frontend_dev]
    parallel: true
  - name: 测试
    agents: [tester]
    on_fail:
      retry_stage: 并行实现
      max_retries: 3
  - name: 审查
    agents: [reviewer]
    on_fail:
      retry_stage: 并行实现
  - name: 提交
    type: shell
    command: |
      cd {workspace}
      cargo fmt
      git add .
      git commit -m "feat: {sprint_id} {title}"
```

> **关键洞察**：把"AI 改代码"做成 workflow，未来所有 sprint 都用同一个工作流跑，**只换任务卡片**。这就是"AI 改 AI"的 dogfood loop。

---

## 🔧 Phase 0 — 地基

> 详细任务见各 sprint yaml 文件。

### v0.0.1 — 修复关键 Bug（11h）
- B1: 16处 `.expect()` → `anyhow::Context` + `?`
- B2: 统一 `parking_lot::Mutex`
- B3: 删除 DateTime `#[serde(default)]`
- B4: `println!` → `tracing`（P2）

### v0.0.2 — A2UI路由挂载 + 统一API格式 + Docker部署（✅ done）
- D1: 挂载 A2UI 路由（工作流人机交互断裂点）✅
- D2: 统一 API 响应格式 `{ok, data, error}` ✅
- D3: docker-compose 一键部署 ✅
- D4: Migration Runner — 17个schema集中管理 ✅

### v0.0.3 — 接入 core/orchestrator（6h，接线不是新建）（✅ done）
- P1: nx_api/Cargo.toml 加 nexus-orchestrator 依赖 ✅
- P2: 替换 AppState 中的 scheduler ✅
- P3: 删除旧 nx_api/src/scheduler/ ✅
- P4: 验证重启恢复 ✅

---

## 🏭 Phase 1 — 生产线核心

### v1.1 — Pipeline 真正跑通（16h）
- E1: pty_task_watcher 完成时回调 pipeline
- E2: PTY 执行注册到进程监控
- E3: Pipeline 前端进度联动
- E4: 项目级互斥锁

### v1.2 — 断点续跑（12h）
- R1: 任务开始时写入 checkpoint
- R2: 执行过程中定期更新 checkpoint
- R3: 启动时检测中断任务并重新入队
- R4: 前端显示中断/续跑状态

### v1.3 — 质量门自动化（20h）
- Q1: 工作流引擎加 quality_gate 支持
- Q2: 5套内置质量门模板（rust/ts/python/go/docker）
- Q3: 质量门可视化（通过/失败 chip）
- Q4: 团队对话质量门

### v1.4 — 可观测性看板（20h）
- O1: 实时进度看板（当前 stage + 耗时）
- O2: Token/Cost 实时显示
- O3: 产物文件列表完整（T1-T4）

---

## 🎨 Phase 2 — 用户体验

### v2.1 — 团队对话体验重构（16h）
- C1: 团队对话改 CLI 优先（删除 PTY 优先逻辑）
- C2: 流式输出到前端（打字机效果）
- C3: 对话历史完整保存
- C4: 确认机制支持多次
- C5: 修复嵌套 tokio runtime

### v2.2 — 产物管理完整（20h）
- 文件预览（Markdown + 代码高亮）
- 二进制文件降级显示
- 执行列表产物 badge
- 分页加载（>50文件）

### v2.3 — 项目状态感知（24h）
- S1: 项目状态快照（模块级别）
- S2: AI 执行前自动注入项目状态
- S3: 项目状态看板 UI
- S4: 智能续跑提示

---

## 🔌 Phase 3 — 功能扩展

### v3.1 — Git 集成（16h）
- 每 stage 自动 commit
- 失败回滚（revert/keep/branch）
- 自动生成 PR 描述

### v3.2 — 触发器系统（8h，core 已有实现）
- Cron 触发
- Webhook 触发
- 链式触发

### v3.3 — Token/Cost 监控（16h）
- Token 计数接入
- 预算告警
- Cost Dashboard

### v3.4 — RAG 知识库（32h）
- 文档上传 + 向量化（sqlite-vec）
- 检索 + 注入 prompt
- 知识库管理 UI

---

## 🤖 Phase 4 — 智能化升级

### v4.1 — 多模型路由（24h）
不被单一厂商锁死，降成本 60-70%：
- M1: 工作流 YAML 支持 per-stage 模型配置
- M2: 复杂度自动评估路由器（.rs→Claude，.tsx→MiMo，总结→Qwen）
- M3: 路由规则配置 UI
- M4: 成本对比看板

### v4.2 — 失败自愈（20h）
失败→重试→换模型→回滚→通知，生产可用：
- F1: 换模型重试策略（同模型失败→自动升级强模型）
- F2: 失败告警通知（Telegram + 前端推送）
- F3: 全链路日志追踪（trace_id 贯穿整条链路）
- F4: 健康检查 + 卡死任务自动恢复

### v4.3 — 低代码可视化画布（80h）
5 分钟拖拽搭建 AI 流水线，开发者 + 业务都能用：
- L1: 画布基础框架（React Flow）
- L2: 7种节点类型（AI调用/代码执行/质量门/条件/HTTP/人工审批/循环）
- L3: YAML 双向同步（画布↔YAML 实时转换）
- L4: 实时执行状态可视化（节点动画+token计数+错误高亮）
- L5: 5个内置模板（全栈开发/Bug修复/代码审查/文档生成/数据处理）

### v4.4 — 浏览器自动化验证（24h）
AI 写完 UI 代码后自动打开浏览器验证效果，闭环"写→看→修"：
- B1: Playwright/Puppeteer 集成（Headless Chrome 启动+页面加载）
- B2: 截图+视觉对比（AI 生成的页面 vs 设计稿/预期）
- B3: 交互验证（自动点击关键按钮、填写表单、验证跳转）
- B4: 验证结果回喂 AI（失败截图+错误信息→自动修复→重试）

### v4.5 — 用户需求分解 UI（20h）
用户在界面上拆需求→AI 接棒执行的完整链路：
- U1: 需求输入界面（自然语言描述→结构化任务卡片）
- U2: AI 自动拆解（大需求→子任务树，用户可编辑调整）
- U3: 任务看板（拖拽排序、优先级、依赖关系可视化）
- U4: 执行进度绑定（每个子任务关联 pipeline stage，实时状态同步）

### v4.6 — 多模态工具链（32h）
支持图片/视频/设计稿生成，扩展 AI 工厂的产品形态：
- MM1: 图片生成接入（DALL-E/Flux/ComfyUI，支持 prompt→图）
- MM2: 设计稿生成（AI 生成 UI 设计稿，导出 Figma/Sketch 格式）
- MM3: 视频生成接入（文生视频/图生视频，用于产品演示）
- MM4: 多模态产物管理（图片/视频/设计稿统一在产物面板展示+预览）

---

## ⚠️ 风险点与应对

| 风险 | 应对 |
|------|------|
| AI 生成代码质量不稳定 | quality_gate 是底线，加 reviewer agent 二次验证 |
| Token 成本失控 | budget_limit + 每 sprint 预算 |
| 大改 break 现有功能 | 每 sprint 跑全量测试；pre-{sprint} tag 兜底 |
| 复杂任务 AI hold 不住 | 任务卡片切到 1-2 天粒度，太大就拆 |

