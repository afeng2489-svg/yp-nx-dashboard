# AI 软件工厂 + 数字员工平台 — 32 周迭代路线图

> 基于当前系统（NexusFlow / yp-nx-dashboard）演进到企业级 AI 数字员工平台的完整可执行方案。
>
> 核心原则：**所有改动都让 AI 自己实现，模型无关（claude / gpt / qwen / glm 都能跑）**。

---

## 📑 目录

- [总览](#-总览)
- [让 AI 自己改自己的核心机制](#-让-ai-自己改自己的核心机制)
- [v0.x 自用工厂（4 周）](#-v0x-自用工厂4-周)
- [v1.x 第一个企业客户（6 周）](#-v1x-第一个企业客户6-周)
- [v2.x 多租户 SaaS（8 周）](#-v2x-多租户-saas8-周)
- [v3.x 规模化平台（8 周）](#-v3x-规模化平台8-周)
- [让 AI 跑这套方案的工程实践](#-让-ai-跑这套方案的工程实践)
- [立即开始的 5 步](#-立即开始的-5-步)
- [风险点与应对](#-风险点与应对)
- [成功指标](#-成功指标)

---

## 🎯 总览

### 起点（v0.0）

| 模块 | 状态 |
|------|------|
| Workflow 引擎（线性 stages） | ✅ |
| PTY 终端 + 团队对话 | ✅ |
| Workspace + 项目管理 | ⚠️ 部分 |
| Skill / Plugin 框架 | ⚠️ 半残 |
| SQLite + 单用户 | ⚠️ |
| Claude CLI 集成 | ✅ |
| Git / 质量门 / 触发器 / RAG | ❌ |
| 多租户 / SSO / RBAC / 审计 | ❌ |

### 目标（32 周后）

```
最终：企业数字员工平台 + 自用 AI 工厂
  ├─ 给企业卖（按席位 + 私有化部署）
  └─ 自用 dogfood（团队效率工具）
```

### 时间线

| 阶段 | 版本 | 时间 | 里程碑 |
|------|------|------|--------|
| Phase 1 | **v0.x 自用工厂** | 0-4 周 | 团队 dogfood 可用 |
| Phase 2 | **v1.x 第一个客户** | 4-12 周 | 单租户私有部署 |
| Phase 3 | **v2.x 多租户 SaaS** | 12-24 周 | 商业化运营 |
| Phase 4 | **v3.x 规模化平台** | 24-32 周 | 多 agent + DAG + 智能调度 |

---

## 🤖 让 AI 自己改自己的核心机制

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

## 📦 v0.x 自用工厂（4 周）

### v0.1：产物管理（1 周）

#### Sprint 1.1：执行后看到生成了什么文件（3 天）

**核心改动**：

后端新建 `artifact_tracker.rs`：

```rust
pub fn snapshot_workdir(path: &Path) -> WorkdirSnapshot {
    // 递归扫描所有文件，记录 path / size / mtime / sha256
}

pub fn diff_snapshots(before: &Snapshot, after: &Snapshot) -> ArtifactDiff {
    // 输出 added / modified / deleted 列表
}
```

workflow_engine 在每个 stage **前** 拍 snapshot，**后** 算 diff，存入 `artifacts` 表。

**新数据表**：

```sql
CREATE TABLE artifacts (
    id TEXT PRIMARY KEY,
    execution_id TEXT NOT NULL,
    stage_name TEXT,
    relative_path TEXT NOT NULL,
    change_type TEXT,  -- added/modified/deleted
    size_bytes INT,
    sha256 TEXT,
    mime_type TEXT,
    created_at TEXT,
    FOREIGN KEY (execution_id) REFERENCES executions(id)
);
```

**验收**：跑一次"内容创作"工作流，产物 tab 列出 `PRD.md`、`outline.md` 等。

#### Sprint 1.2：产物 zip 导出 + 大文件支持（2 天）

导出选中文件为 zip，>5MB 文件不预览只显示元信息。

#### Sprint 1.3：产物按 stage 分组 + 版本对比（2 天）

按 stage 分组；同一文件被多 stage 修改时显示版本链；点击查看 diff。

---

### v0.2：Git 集成（1 周）

#### Sprint 2.1：每 stage 自动 commit（3 天）

```yaml
behavior:
  - workflow 开始前: 在 working_dir 创建分支 ai-exec-{execution_id}
  - 每个 stage 完成: git add . && git commit -m "stage: {stage_name}"
  - workflow 结束: 输出"本次执行涉及 N 个 commit"
```

#### Sprint 2.2：失败回滚 + 分支管理（2 天）

失败时让用户选择"回滚到执行前 / 保留当前 / 创建新分支"。

#### Sprint 2.3：自动生成 PR 描述（2 天）

执行完成后，AI 根据 commit 历史生成一段 PR 描述，复制到剪贴板可直接贴 GitHub。

---

### v0.3：质量门（1 周）

#### Sprint 3.1：stage 级 quality gate（4 天）

**Schema 变化**：

```yaml
quality_gate:
  checks:
    - cmd: "cargo test"
      timeout: 300
    - cmd: "cargo clippy -- -D warnings"
  on_fail: retry  # retry | block | continue
  max_retries: 3
```

**核心机制**：

```rust
async fn execute_stage_with_verify(...) {
    for attempt in 0..max_retries {
        let agent_output = run_agent(...).await?;
        let gate_result = run_quality_gate(&stage.quality_gate).await?;

        if gate_result.passed {
            return Ok(agent_output);
        }

        // 失败：把错误回喂给下一次执行
        let retry_prompt = format!(
            "上次执行失败，错误：\n{}\n请修复",
            gate_result.errors.join("\n")
        );
    }
    Err(WorkflowError::QualityGateFailed)
}
```

#### Sprint 3.2：质量门可视化（2 天）

执行详情页每个 stage 显示"通过/失败"chip，hover 看具体哪条 check 挂了。

#### Sprint 3.3：内置 5 套 quality gate 模板（1 天）

rust / typescript / python / go / docker 五个常用模板，工作流定义里 `quality_gate: rust_default` 可直接复用。

---

### v0.4：触发器系统（1 周）

#### Sprint 4.1：Cron 触发（3 天）

```yaml
workflow.triggers:
  - type: cron
    expression: "0 9 * * MON"  # 每周一 9 点
    timezone: "Asia/Shanghai"
```

后端用 `tokio-cron-scheduler` crate，启动时加载所有 active workflow 的 trigger。

#### Sprint 4.2：Webhook 触发（2 天）

```
POST /api/v1/triggers/webhook/:workflow_id?secret=xxx
body 作为 input variables 传入
```

#### Sprint 4.3：手动 + 链式触发（2 天）

工作流 A 完成时自动触发工作流 B（链式），用 `on_complete: trigger_workflow_id`。

---

### v0.5：Token / Cost 监控（1 周）

#### Sprint 5.1：Token 计数 + 实时显示（3 天）

- 用 `claude --output-format stream-json` 拿 usage
- 累加到 `execution.total_tokens / total_cost_usd`
- 前端右上角实时显示当次执行 cost

#### Sprint 5.2：Token 预算 + 告警（2 天）

每个 workflow 可设 `budget_limit_usd`，超过 80% 告警，超过 100% 自动暂停。

#### Sprint 5.3：Cost dashboard（2 天）

新页面 `/cost`，按天/工作流/agent 分组显示 token 用量曲线（用已装的 recharts）。

---

> **v0.x 完成后**：你团队自用一个真正可用的 AI 工厂，能跑闭环。

---

## 🏢 v1.x 第一个企业客户（6 周）

### v1.0：邮件接入 + 邮件 Agent（2 周）

#### Sprint 1.0.1：IMAP 接入（4 天）

```yaml
backend:
  - 新 service: email_service.rs
  - lettre 或 imap crate 收 IMAP
  - 配置：host, port, username, password, ssl
  - 触发器类型加 "email_received"

trigger_payload: |
  当收到新邮件时，触发 workflow，input vars:
    sender, subject, body_text, body_html, attachments[]
```

#### Sprint 1.0.2：SMTP 发件 + 邮件回复 node（3 天）

工作流 stage 类型 `email_reply`，可回复触发邮件 / 发新邮件。

#### Sprint 1.0.3：第一个邮件 agent demo（3 天）

内置模板"客户咨询自动分类 + 回复"。流程：

```
收到邮件 → 分类（咨询/投诉/订单）→ 查 KB → 草稿回复 → 等管理员 approve → 发出
```

#### Sprint 1.0.4：邮件 agent 控制台（2 天）

UI 列出所有未处理邮件 + 状态 + 一键 approve/reject。

---

### v1.1：HTTP Node + 工具调用（1 周）

#### Sprint 1.1.1：HTTP node（3 天）

```yaml
- type: http
  method: POST
  url: "{api_base}/customers/{customer_id}"
  headers:
    Authorization: "Bearer {api_token}"
  body: { ... }
  output_var: customer_data
```

#### Sprint 1.1.2：内置 5 个常用集成（4 天）

- Slack 发消息
- 钉钉 / 企业微信 webhook
- Notion 写入页面
- GitHub 创建 issue
- Google Sheets 读写

每个一个独立 plugin，复用 `core/plugin` 已有 trait。

---

### v1.2：RAG 知识库（2 周）

#### Sprint 1.2.1：文档上传 + 向量化（4 天）

```yaml
backend:
  - 新建 knowledge_base 表 + documents 表 + chunks 表（带 vector 列）
  - 用 sqlite-vec extension 做向量检索（不需要单独 vector DB）
  - upload API: 支持 PDF/Word/Markdown，自动 chunk + embedding
  - embedding 用：OpenAI text-embedding-3-small / BGE-M3 / 本地 ollama

upload_flow:
  上传 → 解析 → 切块（500 tokens/块）→ embedding → 入库
```

#### Sprint 1.2.2：检索 + 注入 prompt（3 天）

```yaml
- id: customer_support
  rag:
    knowledge_base_id: kb_001
    top_k: 5
    threshold: 0.7
  prompt: |
    使用以下知识库上下文回答客户问题：
    {rag_context}

    问题：{user_question}
```

#### Sprint 1.2.3：知识库管理 UI（3 天）

`/knowledge-base` 页面：上传 / 删除 / 重新索引 / 测试检索。

#### Sprint 1.2.4：增量更新 + 文件监听（2 天）

监听文件夹，新增/修改自动重新索引。

---

### v1.3：结构化审批节点（1 周）

#### Sprint 1.3.1：human_review node（3 天）

升级现有 `user_input`：

```yaml
- type: human_review
  reviewers: ["pm@company.com"]
  required_approvals: 1
  artifacts_to_review: ["draft.md"]
  questions: ["内容是否合规？"]
  timeout_hours: 24
  on_timeout: escalate  # block / auto_approve / escalate
  on_reject:
    feedback_var: rejection_reason
    retry_stage: 内容生成
```

#### Sprint 1.3.2：审批 UI + 通知（4 天）

- Dashboard 顶部红点提示待审批
- 审批页面：产物预览 + 同意/驳回 + 评论
- Telegram/邮件通知（复用现有 telegram_service）

---

### v1.4：私有化打包（1 周）

#### Sprint 1.4.1：docker-compose（3 天）

一键 `docker-compose up` 起整套（nx_api + 前端 + sqlite volume）。

#### Sprint 1.4.2：客户配置导入导出（2 天）

工作流 / 知识库 / agent 配置一键导出 zip，新环境一键导入。

#### Sprint 1.4.3：第一个客户 onboarding doc（2 天）

写一份 30 页 PDF 教客户怎么部署 + 配置 + 运维。

---

> **v1.x 完成后**：能给一个具体客户私有部署一套"邮件客服员工"，按席位收费。

---

## 🌐 v2.x 多租户 SaaS（8 周）

### v2.0：SSO + 多租户隔离（2 周）

#### Sprint 2.0.1：tenant_id 改造（5 天）

```yaml
schema_change: |
  所有业务表加 tenant_id 字段:
    workflows / executions / agents / knowledge_bases / artifacts ...

  middleware:
    所有请求从 JWT 拿 tenant_id，自动注入查询 WHERE tenant_id=...
```

#### Sprint 2.0.2：SSO 接入（5 天）

oauth2 flow 接钉钉 / 企微 / Google Workspace。

#### Sprint 2.0.3：用户邀请 / 团队管理（4 天）

管理员能邀请同事，分配角色。

---

### v2.1：RBAC + 审计日志（2 周）

#### Sprint 2.1.1：role / permission 模型（4 天）

```sql
CREATE TABLE roles (id, tenant_id, name, permissions JSON);
CREATE TABLE user_roles (user_id, role_id);
CREATE TABLE audit_logs (
    id, tenant_id, user_id, action, resource_type, resource_id,
    before_state JSON, after_state JSON, ip, timestamp
);
```

#### Sprint 2.1.2：权限中间件 + UI（5 天）

所有 mutation 检查权限；前端按权限隐藏入口。

#### Sprint 2.1.3：审计日志 UI + 导出（5 天）

`/audit` 页面，按时间/用户/操作过滤；导出 CSV 给合规。

---

### v2.2：管理后台（2 周）

#### Sprint 2.2.1：tenant 管理 / 用量监控 / 健康检查（10 天）

超管能看所有 tenant 状态、用量、活跃度、错误率。

---

### v2.3：计费系统（2 周）

#### Sprint 2.3.1：计费模型（5 天）

```yaml
plans:
  - id: starter
    price_monthly: 99
    quotas:
      max_users: 5
      max_workflows: 10
      max_tokens_per_month: 1_000_000
  - id: pro
    ...
  - id: enterprise
    ... (custom)
```

#### Sprint 2.3.2：Stripe / 微信支付接入（5 天）

账单 + 自动续费 + 超量提醒。

---

## 🚀 v3.x 规模化平台（8 周）

### v3.0：DAG 编排（2 周）

升级 workflow 引擎：

- 节点不只是顺序
- 支持条件分支：`if {condition} then stage_a else stage_b`
- 支持循环：`while {condition}`
- 支持并行+合并：`parallel: [a, b, c] then merge`
- 支持子工作流：`call: workflow_id`

### v3.1：Multi-Agent 对话（2 周）

新节点类型：

```yaml
- type: agent_conversation
  participants:
    - product_manager
    - architect
    - tech_lead
  topic: "讨论如何实现 X 功能"
  speaking_strategy: round_robin / free / debate
  max_turns: 10
  consensus: majority
  output_artifact: design_doc.md
```

> 复用现有 `group_chat_service`！

### v3.2：智能调度（2 周）

- agent 池（同 type 多个并发跑）
- 负载均衡
- 优先级队列
- 失败自动迁移到备用 agent

### v3.3：监控告警（2 周）

- Prometheus exporter
- Grafana dashboard 模板
- Sentry 错误告警
- 自动 RCA（AI 看异常日志生成根因报告）

---

## 🛠 让 AI 跑这套方案的工程实践

### 1. 任务卡片库（每个 sprint 一个 yaml）

建立 `docs/sprints/` 目录：

```
docs/sprints/
  v0.1-S1-artifact-management.yaml
  v0.1-S2-zip-export.yaml
  v0.1-S3-stage-grouping.yaml
  v0.2-S1-git-auto-commit.yaml
  ...
```

每个 yaml 是一个完整任务卡片（前文模板）。

### 2. 元工作流（"改自己"）

定义 `meta-workflow.yaml`，输入是任意 sprint 卡片，跑：

```
plan → research → design → backend_dev || frontend_dev → test → review → commit → demo
```

### 3. 模型路由（让任何模型都能跑）

在 `agent` 定义里抽象：

```yaml
agents:
  - id: planner
    model: ${MODEL_PLANNER}    # 环境变量决定，可换
    fallback_model: ${MODEL_FALLBACK}
```

环境变量示例：

```bash
MODEL_PLANNER=claude-opus-4-7        # 高质量计划
MODEL_DEV=claude-sonnet-4-6          # 中等成本编码
MODEL_REVIEWER=gpt-4.1               # 不同视角 review
MODEL_TESTER=qwen-max                # 国产模型成本低
MODEL_FALLBACK=glm-4
```

调用层抽象（已有 `nexus_ai` trait），加几个 adapter。

### 4. 进度透明 — 每天一份报告

新建 cron：每天 9 点生成"昨日 sprint 进度报告"邮件给你：

```
昨日完成：v0.1.S1 artifact-management ✅
本日进行：v0.1.S2 zip-export
阻塞：无
本周累计 commit：23
本周 token 消耗：$45.2
```

### 5. 失败兜底 — 永远可回滚

每个 sprint 跑前自动 `git tag pre-{sprint_id}`。失败：

```bash
git reset --hard pre-{sprint_id}
```

### 6. Dogfood loop

每个 sprint 完，**让 AI 自己评测**：

- 这次改动是否满足 acceptance？
- 有没有引入新 bug？
- 代码质量打几分？

输出加到 `docs/sprints/{sprint_id}-retro.md`。

---

## ⚡ 立即开始的 5 步

1. **本周一**：建 `docs/sprints/`，把 v0.1.S1 卡片 yaml 写好
2. **本周二**：写 `meta-workflow.yaml`，跑通"AI 自己改 ScreenEmu 增加一个测试用例"作为最小验证
3. **本周三**：跑通 v0.1.S1 全流程，产出 `artifact_tracker.rs`
4. **本周四 - 周五**：手动 review + 调 prompt + 重跑直到 quality_gate 全过
5. **下周一**：v0.1.S2 由 AI 全自动完成（你只 approve）

> 成功的标志：**第二周开始你不再写代码，只 approve PR**。

---

## ⚠️ 风险点与应对

| 风险 | 应对 |
|------|------|
| AI 生成代码质量不稳定 | quality_gate 是底线，加 reviewer agent 二次验证 |
| Token 成本失控 | budget_limit + 每 sprint 预算 + 每天报告盯紧 |
| 大改 break 现有功能 | 每 sprint 跑全量 e2e；pre-{sprint} tag 兜底 |
| Prompt 写得不准 | 第一次手动迭代 5 轮 prompt，存模板复用 |
| 复杂任务 AI hold 不住 | 任务卡片必须切到 1-2 天粒度，太大就拆 |
| 销售周期长 | 第一个 friendly customer 走 PoC，3 个月内出 ROI |
| 合规审查 | 等第一个客户给压力再做 SOC2 / ISO27001 |

---

## 📊 成功指标（每周看）

| 指标 | 目标 |
|------|------|
| 累计完成 sprint 数 | ≥ 1 / 周 |
| 自动测试通过率 | > 90% |
| 平均一个 sprint 工时 | < 8 h |
| 每个 sprint token 消耗 | < $5 |
| 每周引入的新 bug | = 0 |

---

## 🎯 商业化对标

| 公司 | 模式 | 估值 / 状态 |
|------|------|-------------|
| Lindy AI（YC W23） | AI 员工，按席位卖 | 估值 1B+ |
| Beam AI | 企业 agent 流程自动化 | 估值 1.5B |
| Adept | 多模态 enterprise agent | 被 Amazon 收购，4B+ |
| 智谱 GLM Agent / 字节豆包企业版 / 腾讯灵聚 | 国内同类 | 估值各异 |

商业模式：**按席位（agent 数）+ token 用量**，私有化部署另收维护费。

---

## 🔑 关键技术决策

### 1. 数据隔离架构

| 方案 | 优点 | 缺点 | 推荐 |
|------|------|------|------|
| 共享 DB + tenant_id 字段 | 简单、迁移容易 | 数据泄漏风险高 | ❌ |
| 每 tenant 独立 schema | 平衡好 | 中等复杂度 | ✅ MVP |
| 每 tenant 独立 DB instance | 最安全 | 运维成本高 | ⚠️ 大客户 |

### 2. 私有化 vs SaaS

```
SaaS（你托管）：客户上传数据 → 你提供服务
  ✅ 部署一份养所有客户
  ❌ 企业不接受，数据合规过不去

私有化（客户自部署）：docker-compose 一键起
  ✅ 数据不出客户网络，企业必选
  ❌ 每个客户单独维护版本

混合（推荐）：
  - 中小企业用 SaaS
  - 大企业（年付 50w+）私有化
```

### 3. 模型策略

```
默认：OpenAI / Anthropic API（用客户自己的 key）
可选：Azure OpenAI（合规友好）
高端：私有部署 Llama / Qwen / GLM（数据完全不出网）
```

---

## 📂 附录：项目目录结构演进

```
yp-nx-dashboard/
├─ docs/
│  ├─ sprints/              # 每个 sprint 的任务卡片 yaml（v0.x 起）
│  └─ retro/                # 每个 sprint 的复盘报告
├─ workflows/
│  ├─ meta/
│  │  └─ ai-self-improvement.yaml   # 元工作流
│  └─ templates/
│     ├─ email-customer-support.yaml  # v1.0 起
│     ├─ rag-faq-bot.yaml              # v1.2 起
│     └─ full-sdlc.yaml                # v3.x 起
├─ core/
│  ├─ workflow/             # 现有，逐步增强
│  ├─ artifact/             # v0.1 新增
│  ├─ trigger/              # v0.4 新增
│  ├─ rag/                  # v1.2 新增
│  └─ tenant/               # v2.0 新增
├─ nx_api/                  # 现有
├─ nx_dashboard/            # 现有
└─ deploy/
   ├─ docker-compose.yml    # v1.4 新增
   ├─ k8s/                  # v2.x 新增
   └─ helm/                 # v3.x 新增（大客户）
```

---

## 📝 文档维护

本路线图按版本节奏更新：

- 每完成一个 sprint，标记 ✅ + 链接到 retro 报告
- 每月底 review 路线图，根据实际进展调整
- 收到客户反馈时插入新 sprint，标记 `[customer-driven]`

---

**最后更新**：2026-04-29  
**当前版本**：v0.0（基线）  
**下一个里程碑**：v0.1 自用工厂 — 产物管理（4 周内）
