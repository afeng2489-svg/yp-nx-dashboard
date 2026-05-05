# 给 AI 测试员的入口文件

## 你的任务
对 NexusFlow 项目进行全功能端到端测试，报告每个模块的通过/失败状态。

## 第一步：读这两个文件

1. `docs/e2e-test-cards.md` — 36 张测试卡片，每张含 curl 命令 + 预期结果（后端 API 测试）
2. `nx_dashboard/e2e/` 目录 — 19 个 Playwright spec 文件（自动化测试）

## 第二步：启动环境

```bash
# 后端
cd /Users/Zhuanz/Desktop/yp-nx-dashboard
cargo run --bin nx_api

# 前端（Playwright 测试需要）
cd nx_dashboard && npm run dev
```

## 第三步：执行测试

**方式 A — 自动化（推荐）**
```bash
cd /Users/Zhuanz/Desktop/yp-nx-dashboard/nx_dashboard
npx playwright test --reporter=list
```

**方式 B — 手动逐卡执行**
读 `docs/e2e-test-cards.md`，按 CARD-01 到 CARD-36 顺序执行每张卡片的 curl 命令。

## 第四步：汇报结果

按此格式输出：
```
CARD-01 健康检查        ✅
CARD-02 Workflow CRUD   ✅
CARD-03 Execution       ❌ 原因：...
...
```

## 文件地图

| 文件 | 用途 |
|------|------|
| `docs/e2e-test-cards.md` | 36 张手动测试卡片（curl） |
| `nx_dashboard/e2e/helpers.ts` | Playwright 共享工具函数 |
| `nx_dashboard/e2e/workflow-crud.spec.ts` | Workflow CRUD 测试 |
| `nx_dashboard/e2e/execution-lifecycle.spec.ts` | Execution 生命周期 |
| `nx_dashboard/e2e/pipeline-lifecycle.spec.ts` | Pipeline 完整流程 |
| `nx_dashboard/e2e/session-lifecycle.spec.ts` | Session 管理 |
| `nx_dashboard/e2e/checkpoint-resume.spec.ts` | 断点续跑 |
| `nx_dashboard/e2e/teams-roles.spec.ts` | 团队与角色 |
| `nx_dashboard/e2e/skills.spec.ts` | 技能系统 |
| `nx_dashboard/e2e/ai-config.spec.ts` | AI 配置（CLI/模型/Provider） |
| `nx_dashboard/e2e/tasks-projects-issues.spec.ts` | 任务/项目/问题追踪 |
| `nx_dashboard/e2e/triggers-scheduler.spec.ts` | 触发器与调度器 |
| `nx_dashboard/e2e/knowledge-base.spec.ts` | RAG 知识库 |
| `nx_dashboard/e2e/workspaces.spec.ts` | 工作区与 Git |
| `nx_dashboard/e2e/costs-search-wisdom.spec.ts` | 费用/搜索/知识沉淀 |
| `nx_dashboard/e2e/websockets.spec.ts` | 4 个 WebSocket 端点 |
| `nx_dashboard/e2e/templates-plugins-testgen.spec.ts` | 模板/插件/测试生成 |
| `nx_dashboard/e2e/group-sessions-processes.spec.ts` | 群组会话与进程 |
| `nx_dashboard/e2e/terminal-multiwindow.spec.ts` | 终端多窗口 |
| `nx_dashboard/e2e/workflow-creation.spec.ts` | Workflow 创建 UI |

## 已知风险（测试前必读）

- `checkpoints` 表可能为空 → 断点续跑测试预期失败
- 需要本机安装 `claude` 或 `gemini` CLI → AI 执行类测试才能通过
- WebSocket 测试结果 `error` 或 `timeout` 均视为"端点存在"，不算失败
