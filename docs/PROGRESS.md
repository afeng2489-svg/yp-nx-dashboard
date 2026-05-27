# NexusFlow — AI 软件工厂 开发进度

> **AI 恢复指令**：读 [`docs/sprints/MASTER-PLAN.yaml`](sprints/MASTER-PLAN.yaml) + [`docs/progress.json`](progress.json) → 找 `current_sprint` → 读对应 `docs/sprints/AF-*.yaml`。

---

## 产品定位

**目标**：300–500 独立开发者觉得好用 — 工厂台一句话 → 15min 内看到 diff → 审批 → 合入。

**定位**：本地 AI 开发部 — 不取代 Cursor，取代终端里的 claude 编排。

**单机优先**：累计试用 / MAU 目标，非多租户 SaaS。

三轮验证结论见 [`docs/sprints/VALIDATION-3ROUND.md`](sprints/VALIDATION-3ROUND.md)（R3：**7.2/10**，62% 成功率）。

---

## 快速恢复 SOP

```
1. 读 docs/sprints/MASTER-PLAN.yaml → execution_order
2. 读 docs/progress.json → current_sprint
3. 读 docs/sprints/{sprint_id}-*.yaml → 第一个 planned 的 remaining_task
4. 执行该任务
5. 完成后仅通过 scripts/gate-check.sh 更新 completed（禁止手改）
6. 继续下一 sprint
```

---

## 当前状态

| 字段 | 值 |
|------|-----|
| **当前 Sprint** | **AF-00b** — Code hygiene |
| **Sprint 状态** | `planned` |
| **刚完成** | AF-00 Security（permissions_mode + workspace + blocklist） |
| **Tracker** | v5.0 |
| **Master Plan** | [`MASTER-PLAN.yaml`](sprints/MASTER-PLAN.yaml) |
| **归档** | v0–v4 见 [`sprints/_archive/`](sprints/_archive/) |
| **最后更新** | 2026-05-27 |

---

## AF 计划进度

```
AF-P0 安全与卫生     ░░░░░░░░░░░░░░░░░░░░   0%   AF-00, AF-00b
AF-P1 工厂台 MVP     ░░░░░░░░░░░░░░░░░░░░   0%   AF-01, AF-02, AF-03
AF-P2 Golden Path    ░░░░░░░░░░░░░░░░░░░░   0%   AF-04, AF-04b, AF-05, GATE-2
AF-P3 内测           ░░░░░░░░░░░░░░░░░░░░   0%   AF-06
AF-P4 公测           ░░░░░░░░░░░░░░░░░░░░   0%   AF-07, AF-08, AF-09
```

| Sprint | 标题 | 状态 |
|--------|------|------|
| AF-00 | Security — permissions + workspace | planned |
| AF-00b | Hygiene — dead_code + deprecated | planned |
| AF-01 | 工厂台 MVP | planned |
| AF-02 | 审批 Harness MVP | planned |
| AF-03 | WS 可靠性 | planned |
| AF-04 | Golden Path + dogfood | planned |
| AF-04b | Executor 双车道（CLI vs API） | planned |
| AF-05 | macOS 安装包 | planned |
| GATE-2 | 10 人朋友测 | planned |
| AF-06 | 50 人内测（容器卡） | planned |
| AF-07 | 资产库 + 运营中心 | planned |
| AF-08 | 多团队 + Sprint | planned |
| AF-09 | 300 人公测 | planned |

---

## 已归档（v0–v4）

Phase 0–4 共 24 sprint 已归档，**勿再执行**。能力清单与 AF 映射见 [`sprints/_archive/README.md`](sprints/_archive/README.md)。

---

## GATE 规则

| Gate | 验证 |
|------|------|
| GATE-1 | `e2e/golden-path.spec.ts`（AF-04） |
| GATE-2 | `cargo test -p nexus_workflow --lib` |
| GATE-3 | `scripts/gate-check.sh`（须验证 artifact） |

---

## 执行命令

```bash
cargo build && cargo test -p nexus_workflow --lib
cd nx_dashboard && npx tsc --noEmit
./scripts/gate-check.sh AF-00   # 占位，完整实现见 AF-04/AF-05
```
