# AF-01 工厂台 MVP — 技术架构设计

> **Sprint ID**: AF-01 | **Status**: in_progress | **估时**: 80h  
> **设计日期**: 2026-05-27 | **架构师**: AI Architect

---

## 一、总体架构

### 当前状态摘要

| 任务 | Sprint 状态 | 实际情况 | 关键差距 |
|------|-------------|----------|----------|
| F1 Factory Shell | partial | partial | **ContextPanel 完全缺失** |
| F2 5项导航+重定向 | done | done | — |
| F3 Console 视图 | done | done | quick-run 未传 team_id |
| F4 Runs/Approvals/Deliverables | partial | partial | Deliverables 无真实产物数据管道 |
| F5 Solo 团队向导 | partial | partial | **POST /teams/from-template 不存在** |
| F6 团队-工作流绑定 | pending | pending | **全链路缺 team_id / trigger_source / role overlay** |

### 目标架构

```
┌─────────────────────────────────────────────────────────┐
│              Dashboard Shell (F1 补完)                     │
├──────────┬────────────────────────────────┬──────────────┤
│ Sidebar  │  FactoryGlobalBar              │ ContextPanel │  ← F1: 新增
│  (F2 ✅) │  ┌──────┬──────┬────────┐     │  diff 摘要    │
│          │  │项目  │团队  │Run计数 │     │  可收起侧栏   │
│          │  └──────┴──────┴────────┘     │              │
├──────────┴───────────────┬────────────────┴──────────────┤
│   FactoryPage (F3 ✅)    │ StatusBar                        │
│   ┌──┬──┬──┬──┐          │ permissions-mode · WS连接状态      │
│   │C │R │A │D │          │                                  │
│   └──┴──┴──┴──┘          │                                  │
│                          │                                  │
│  Console (F3 ✅)          │  Runs (F4 — 增强过滤)              │
│  - IntentInput           │  - 按 team_id 过滤               │
│  - QuickLines            │  - trigger_source=factory       │
│  - +team_id 传递 (F6)    │  - WebSocket snapshot team 过滤  │
│                          │                                  │
│  Approvals (F4 基础)      │  Deliverables (F4 — 增强)        │
│  - user_input pause      │  - 真实 artifact API 管道        │
│                          │  - diff 预览 + 文件列表           │
│                          │                                  │
├──────────────────────────┴──────────────────────────────────┤
│              SoloTeamWizardBanner (F5 补后端)                 │
│              ┌──────────┐                                    │
│              │ 一键创建  │ → POST /teams/from-template        │
│              │ Solo全栈  │   创建 Team + Roles + Workflow     │
│              └──────────┘    Binding + Skills                │
└─────────────────────────────────────────────────────────────┘
```

---

## 二、模块详细设计

### F1 — ContextPanel（新增组件）

**目标**：Dashboard 右侧可收起侧栏，选中 Run 时展示 diff 摘要和进度。

#### 文件清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `nx_dashboard/src/components/factory/ContextPanel.tsx` | **新增** | ContextPanel 组件本体 |
| `nx_dashboard/src/components/layout/Dashboard.tsx` | **修改** | 集成 ContextPanel，右侧 slot |
| `nx_dashboard/src/stores/contextPanelStore.ts` | **新增** | ContextPanel 状态管理 (展开/收起/选中的execution) |

#### 数据模型

```typescript
// contextPanelStore.ts
interface ContextPanelState {
  isOpen: boolean;
  selectedExecutionId: string | null;
  toggle: () => void;
  selectExecution: (id: string | null) => void;
  close: () => void;
}
```

#### ContextPanel 组件设计

```
┌─ ContextPanel ──────────────────────┐
│ [×] 收起                          │
│ ────────────────────────────────── │
│ Run: {id.slice(0,8)}              │
│ Status: running / completed / ...  │
│ ────────────────────────────────── │
│ Stage 进度:                        │
│  ████████░░ 3/5 stages            │
│ ────────────────────────────────── │
│ 当前 Stage: code-generation        │
│ Agent: backend-dev (running)       │
│ ────────────────────────────────── │
│ 最近 diff:                         │
│  + src/foo.rs  (12 lines)         │
│  ~ src/bar.tsx (3 lines)          │
│ ────────────────────────────────── │
│ [查看完整产物 →]                   │
└───────────────────────────────────┘
```

数据来源：
- Execution 状态：现有 `executionStore` (WebSocket 实时更新)
- Diff 文件列表：`GET /api/v1/executions/:id/artifacts` (已有接口)
- Stage 进度：`stage_results` 数组 + `current_stage` 字段

#### Dashboard 集成方案

```tsx
// Dashboard.tsx 修改点
<div className="flex h-full">
  <Sidebar />
  <main className="flex-1">
    <header>GlobalBar + WorkspaceSelector</header>
    <Outlet />
    <StatusBar />
  </main>
  {contextPanel.isOpen && <ContextPanel />}  {/* 新增 */}
</div>
```

ContextPanel 宽度 320px，右侧固定。通过 `contextPanelStore` 控制显隐。

#### 风险与对策

| 风险 | 对策 |
|------|------|
| ContextPanel 与 Allotment 分屏冲突 | 仅在不打开编辑器分屏时渲染 ContextPanel |
| diff 数据量大导致渲染卡顿 | 限制展示最近 10 条 diff，分页加载 |
| WebSocket 事件风暴导致频繁更新 | throttle 300ms |

---

### F4 — Deliverables Tab 增强

**目标**：从真实 artifact API 管道获取数据，而非仅遍历内存中的 stage_results。

#### 文件清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `nx_dashboard/src/components/factory/FactoryDeliverablesTab.tsx` | **重写** | 接入真实 API + 团队过滤 |
| `nx_api/src/routes/artifacts.rs` | **新增** | 按 execution_id + team_id 聚合产物列表 |

#### 数据流设计

```
FactoryDeliverablesTab
  │
  ├─ 1. 获取完成/运行中的 executions
  │     GET /api/v1/executions?team_id={currentTeam.id}&status=completed,running
  │     (新增 team_id 过滤参数，见 F6)
  │
  ├─ 2. 对每个 execution 获取产物摘要
  │     GET /api/v1/executions/:id/artifacts/summary  (已有)
  │
  └─ 3. 渲染文件列表
       ┌─ 文件路径 + 预览 (Markdown/code 高亮)
       └─ 二进制文件降级显示 (大小+下载链接)
```

#### 后端新增路由

```rust
// nx_api/src/routes/artifacts.rs
// GET /api/v1/executions/:id/artifacts/deliverables
// 按 execution 聚合产物，返回结构：
#[derive(Serialize)]
pub struct DeliverableSummary {
    pub execution_id: String,
    pub workflow_name: String,
    pub stage_name: String,
    pub files: Vec<DeliverableFile>,
    pub created_at: DateTime<Utc>,
    pub total_files: usize,
}

#[derive(Serialize)]
pub struct DeliverableFile {
    pub path: String,
    pub change_type: ChangeType, // Added, Modified, Deleted
    pub size_bytes: u64,
    pub preview_url: Option<String>,  // for text/code files
    pub is_binary: bool,
}
```

#### 前端接口

```typescript
// FactoryDeliverablesTab 重写核心逻辑
interface DeliverablesState {
  items: DeliverableSummary[];
  loading: boolean;
  selectedExecution: string | null;
}

// 使用 useEffect 获取，依赖 currentTeam
useEffect(() => {
  if (currentTeam?.id) {
    fetchExecutions({ team_id: currentTeam.id });
  }
}, [currentTeam?.id]);
```

---

### F5 — Solo 团队模板 (`POST /teams/from-template`)

**目标**：一个 API 调用完成团队 + 角色 + 工作流绑定 + 技能分配，3分钟内创建完整的 Solo 团队。

#### 模板定义

```yaml
template_id: solo-fullstack
name: "Solo 全栈"
description: "一人即团队：架构 → 开发 → 测试 → 审查"
roles:
  - id: solo-architect
    name: 架构师
    instructions: "你是全栈架构师，负责分析需求并输出实现计划"
    model: claude-sonnet-4-7
    skills: [plan, architecture-design, tech-stack-selection]

  - id: solo-developer
    name: 全栈开发者
    instructions: "你是全栈开发者，负责编写前后端代码，遵循项目现有代码风格"
    model: claude-sonnet-4-7
    skills: [rust-backend, react-frontend, code-generation]

  - id: solo-tester
    name: 测试工程师
    instructions: "你负责编写和运行测试，确保代码质量"
    model: claude-haiku-4-5
    skills: [unit-test, integration-test, cargo-test]

  - id: solo-reviewer
    name: 代码审查员
    instructions: "你审查代码质量、安全性和性能。只 approve 或 reject 并提供具体理由"
    model: claude-sonnet-4-7
    skills: [code-review, security-check, performance-check]

workflow_bindings:
  - workflow_name: solo-dev
    role_overlay:
      architect: solo-architect
      developer: solo-developer
      tester: solo-tester
      reviewer: solo-reviewer

  - workflow_name: quick-fix
    role_overlay:
      investigator: solo-architect
      fixer: solo-developer
      tester: solo-tester
```

#### API 设计

```
POST /api/v1/teams/from-template
Content-Type: application/json

{
  "template_id": "solo-fullstack",
  "name": "Solo 全栈",         // 可选覆盖
  "workspace_id": "..."         // 可选绑定工作区
}

Response 201:
{
  "ok": true,
  "data": {
    "team": { id, name, ... },
    "roles": [ { id, name, ... }, ... ],
    "workflow_bindings": [ { workflow_name, role_overlay }, ... ]
  }
}
```

#### 文件清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `nx_api/src/routes/teams.rs` | **修改** | 新增 from_template 路由和处理器 |
| `nx_api/src/services/team_template_service.rs` | **新增** | 模板实例化逻辑 |
| `nx_api/src/data/team_templates.rs` | **新增** | 模板定义常量（solo-fullstack） |
| `nx_api/src/routes/mod.rs` | **修改** | 注册新路由 |
| `nx_dashboard/src/components/factory/SoloTeamWizardBanner.tsx` | **修改** | 调用新 API，显示创建进度 |
| `nx_dashboard/src/stores/teamStore.ts` | **修改** | 添加 createTeamFromTemplate 方法 |

#### 实现要点

1. **模板定义**：硬编码在 `nx_api/src/data/team_templates.rs` 中。不做模板市场，不存数据库。
2. **事务性**：模板创建是 all-or-nothing。任一步骤失败，回滚已创建的资源。
3. **幂等性**：如果已存在同名 Solo 团队，返回现有团队（不重复创建）。
4. **角色复用**：如果系统已有同名全局角色（如 "架构师"），复用现有角色的 skills；否则新建。

#### 风险与对策

| 风险 | 对策 |
|------|------|
| 模板创建的 workflow 与用户已有的 workflow 冲突 | from-template 不创建新 workflow，只做绑定 (workflow_bindings 引用已有 workflow) |
| 前端已创建空壳 Solo 团队 | from-template 检查已有 Solo 团队，若存在则补创建角色+绑定（迁移模式） |
| 技能表为空导致角色创建后无能力 | 内置默认 skills 表，模板创建时同步种子数据 |

---

### F6 — Team ↔ Workflow 绑定（全链路 team_id 注入）

**目标**：工厂台启动的每个 Run 都记录 `team_id` 和 `trigger_source=factory`，团队角色 prompt 作为变量注入到 workflow。

#### 数据模型变更

##### a) Execution 模型（后端）

```rust
// execution_service.rs — Execution 结构体新增字段
pub struct Execution {
    // ... 现有字段 ...
    /// 关联的团队 ID（来自工厂台）
    #[serde(default)]
    pub team_id: Option<String>,
    /// 触发来源：factory / api / webhook / cron
    #[serde(default)]
    pub trigger_source: Option<String>,
}
```

##### b) StartExecutionRequest 变更

```rust
// routes/executions.rs
pub struct StartExecutionRequest {
    pub workflow_id: String,
    #[serde(default = "default_variables")]
    pub variables: serde_json::Value,
    /// 新增可选字段
    pub team_id: Option<String>,
    pub trigger_source: Option<String>,
}
```

##### c) 前端 Execution 接口

```typescript
// executionStore.ts
export interface Execution {
  // ... 现有字段 ...
  team_id?: string;
  trigger_source?: string;  // "factory" | "api" | "webhook" | "cron"
}
```

#### 数据流

```
FactoryConsoleTab (用户输入)
  │
  ├─ 读取 currentTeam.id (从 GlobalBar 选择)
  │
  ├─ POST /api/v1/quick-run
  │   {
  │     "prompt": "...",
  │     "team_id": "xxx"         ← 新增
  │   }
  │
  ├─ quick_run.rs handler:
  │   1. match_workflow() → 选择工作流
  │   2. inject_team_variables() ← 新增: 把团队角色 prompt 注入 variables.team_context
  │   3. execute_workflow(team_id=Some(...), trigger_source="factory")
  │
  └─ execution 记录:
      {
        id: "...",
        team_id: "xxx",
        trigger_source: "factory",
        variables: {
          ...
          team_context: {           ← 注入的团队上下文
            team_name: "Solo全栈",
            roles: [
              { name: "架构师", instructions: "...", model: "..." },
              ...
            ]
          }
        }
      }
```

#### Role Overlay 注入机制

```rust
// quick_run.rs 或 execution_service.rs 新增函数
fn inject_team_context(
    team_service: &TeamService,
    team_id: &str,
    variables: &mut serde_json::Value,
) -> Result<(), TeamServiceError> {
    let team_with_roles = team_service.get_team_with_roles(team_id)?;

    let roles_info: Vec<serde_json::Value> = team_with_roles
        .roles
        .iter()
        .map(|r| serde_json::json!({
            "name": r.role.name,
            "instructions": r.role.system_prompt,
            "model": r.role.model_config.as_ref().map(|c| &c.model_id),
            "skills": r.skills.iter().map(|s| &s.skill_name).collect::<Vec<_>>(),
        }))
        .collect();

    variables["team_context"] = serde_json::json!({
        "team_id": team_id,
        "team_name": team_with_roles.team.name,
        "roles": roles_info,
    });

    Ok(())
}
```

#### 执行过滤（按团队）

```
GET /api/v1/executions?team_id={team_id}&trigger_source=factory
```

后端在 `list_executions` handler 中增加可选查询参数过滤。

#### 文件清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `nx_api/src/services/execution_service.rs` | **修改** | Execution 加 team_id/trigger_source；execute_workflow 加参数 |
| `nx_api/src/routes/executions.rs` | **修改** | StartExecutionRequest 加 team_id/trigger_source；list_executions 加过滤 |
| `nx_api/src/routes/quick_run.rs` | **修改** | QuickRunReq 加 team_id；调用 inject_team_context |
| `nx_dashboard/src/stores/executionStore.ts` | **修改** | Execution 接口加 teamId/triggerSource；startExecution 传参 |
| `nx_dashboard/src/components/factory/FactoryConsoleTab.tsx` | **修改** | quick-run 请求带 currentTeam.id |
| `nx_api/src/routes/mod.rs` | **修改** | 路由参数调整 |

#### WebSocket 过滤

WebSocket 推送的 execution 更新事件需携带 `team_id`，前端按当前选中的团队过滤：
- 后端 snapshot 已包含 `team_id`，前端 receiver 按 team_id 过滤显示
- 或在 quick-run 启动时，前端已记住 execution_id，通过单独的 WS 连接跟踪

**推荐方案**：不做全局 WS 过滤，而是沿用现有的 per-execution WS 连接模式。Console 启动 Run 时创建该 execution 的 WS 连接，天然隔离。

---

## 三、API 路由汇总

### 新增路由

| Method | Path | Handler | 说明 |
|--------|------|---------|------|
| POST | `/api/v1/teams/from-template` | `teams::create_team_from_template` | Solo 模板创建（F5） |
| GET | `/api/v1/executions/:id/artifacts/deliverables` | `artifacts::list_deliverables` | 按执行聚合产物（F4） |

### 修改路由

| Method | Path | 变更 |
|--------|------|------|
| POST | `/api/v1/quick-run` | QuickRunReq 新增 `team_id: Option<String>` |
| POST | `/api/v1/executions/start` | StartExecutionRequest 新增 `team_id`, `trigger_source` |
| GET | `/api/v1/executions` | 新增可选查询参数 `team_id`, `trigger_source` |

---

## 四、实现顺序（依赖关系）

```
F6 (team_id 数据模型)
 ├─ 被 F3 依赖 (Console 传 team_id)
 ├─ 被 F4 依赖 (按 team 过滤 executions)
 └─ 被 F5 依赖 (from-template 创建的 team 需要绑定 workflow)

建议执行顺序:
  Day 1-2:  F6 后端 (Execution + team_id/trigger_source)  ← 先行，其他都依赖它
  Day 3:    F6 前端 (Console + store 适配)
  Day 4-5:  F5 后端 (from-template endpoint)
  Day 5-6:  F5 前端 (SoloTeamWizardBanner 适配)
  Day 7:    F4 (Deliverables 增强)
  Day 8:    F1 (ContextPanel)
  Day 9-10: 端到端测试 + Golden Path 验证
```

实际 F6 后端改动量小（约 6 个文件、每个文件 5-20 行），可 1 天内完成。

---

## 五、Golden Path 验证清单

```
☐ 1. 打开 /factory，看到 SoloTeamWizardBanner
☐ 2. 点击"一键创建"，3分钟内创建完成，顶栏团队选择器出现 "Solo 全栈"
☐ 3. 选择 Solo 团队和工作区
☐ 4. 在 Console 输入"给 README 加快速开始章节"
☐ 5. ⌘+Enter 启动 Run → 跳转 Runs Tab 看到进度
☐ 6. WebSocket 实时更新 stage 进度
☐ 7. Run 完成后 Deliverables Tab 显示变更文件
☐ 8. 切换到 Runs Tab → 能看到该 team 的所有 runs
☐ 9. ContextPanel 打开能看 diff 摘要
☐ 0. 全程 0 次需要开终端
```

---

## 六、风险汇总

| 风险 | 严重度 | 影响 | 缓解措施 | 关联任务 |
|------|--------|------|----------|----------|
| team_id 注入改动面太广 | 中 | 波及 6+ 文件，可能遗漏 | F6 先做，其余任务依赖 F6 的结构 |
| from-template 与已有空壳 Solo 团队冲突 | 低 | 前端已创建空壳团队 | from-template 实现迁移模式检测已有团队 |
| Workflow 引擎不消费 team_context | 中 | role overlay 注入后工作流不使用它 | 在 workflow YAML 的 agent prompt 模板中引用 `{{team_context}}` 变量 |
| ContextPanel 与 Allotment 编辑器分屏冲突 | 低 | UI 空间不足 | ContextPanel 仅在编辑器关闭时显示；或悬停面板替代 |
| WebSocket 重连后 team 过滤失效 | 低 | 数据混乱 | 前端 WS 连接时传 execution_id，per-execution 连接天然隔离 |

---

## 七、非功能需求

| 维度 | 要求 | 验证方式 |
|------|------|----------|
| 性能 | Console 启动 Run P95 < 2s | quick-run 响应时间日志 |
| 可靠性 | from-template 事务性保证 | 集成测试覆盖回滚场景 |
| 兼容性 | 现有 API 不破坏 | 全量 `cargo test` + `npx tsc --noEmit` |
| 安全 | team_id 校验：用户只能操作自己创建的团队 | 中间件或 handler 内校验 |
