# AF-P5 治理手册（规划 + 实现）

> **规划真相源：** [`AF-P5-unified-capabilities.yaml`](./AF-P5-unified-capabilities.yaml) **v3.0**  
> **规划变更日志：** [`AF-P5-DECISIONS.md`](./AF-P5-DECISIONS.md)  
> **实现验收：** `./scripts/gate-check.sh AF-UX-XX` + `e2e/journey-af-p5.spec.ts`

---

## A. 规划防跑偏（迭代软件时，保证 AI/人 不改规划）

### A1. 三层文档

| 层 | 文件 | 谁能改 |
|----|------|--------|
| **宪法** | yaml `product_principles` P1–P11 | 仅 @qinyu + 写入 DECISIONS |
| **路线图** | yaml `execution_order_v3` / epics | 提案 → DECISIONS 批准 → 改 yaml |
| **实现** | 代码 PR | 不得反向改路线图，除非走 A2 |

### A2. 规划变更四步

1. **动机一句** — 解决什么用户问题  
2. **改 yaml 哪节** — Epic 顺序 / scope / 新增 Epic  
3. **写 DECISIONS** — 用模板，标 `批准：@qinyu`  
4. **你回复「批准改 yaml」** — 之后 AI/开发才可按新版执行  

**口头聊天 ≠ 规划变更。**

### A3. 新 Cursor 会话锚点（规划用）

```
规划锚点（只读）：
- yaml: docs/sprints/AF-P5-unified-capabilities.yaml v3.0
- decisions: docs/sprints/AF-P5-DECISIONS.md（最近 3 条）
- 当前 Epic: 见 DECISIONS「当前执行焦点」WIP=1
- 若建议新方向 → 先写 DECISIONS「待决提案」，不要直接扩 scope
- 宪法 P1–P11 本季度不改
```

### A4. 规划复盘（每周 15min，只问 4 题）

1. 楔子承诺还能描述本周交付吗？  
2. 当前 Phase 退出标准达到了吗？  
3. 有没有未经 DECISIONS 批准就做的 Epic？  
4. 下周仍 **WIP=1** — 只 pick 下一个 Epic  

### A5. 规划偏差红灯

| 红灯 | 处理 |
|------|------|
| AI 另起一套完整新规划 | 要求合并进 yaml + DECISIONS diff |
| 未批准就调整 Epic 优先级 | 停止实现，先走 A2 |
| 宪法 P1–P11 被实现细节否定 | 先 DECISIONS，否则 revert 方向 |
| yaml 与 DECISIONS 矛盾 | 以 DECISIONS 已批准条目为准，修 yaml |

---

## B. 实现防跑偏（按规划做出来的东西要对）

### B1. 三层防护

```
L1 契约 — journey-af-p5.spec.ts + Epic acceptance
L2 机器 — gate-check.sh
L3 人工 — PR-CHECKLIST-AF-P5.md + 15min dogfood
```

没有 L1 测试（含 skip 占位）的 Epic **不准开写 UI**。

### B2. Epic 开工流程

1. 锁 scope（yaml + PR 顶部）  
2. 先红后绿（journey e2e）  
3. `./scripts/gate-check.sh AF-UX-XX`  
4. PR checklist + dogfood  

### B3. 实现偏差红灯

| 红灯 | 违反 |
|------|------|
| 首屏 6+ 产线卡片 | P3 |
| 必选 workflow 名 | P1 |
| Run 结束无 CTA | P5 |
| 启动前零预览（v3 后） | P8 |
| 失败只能看日志无重试 | P10 |

### B4. 发布 Phase 闸门

| Phase | Epic | 用户标准 |
|-------|------|----------|
| Alpha-UX-1 | UX-01,02 + AF-12,16 | 30s 启动 Run |
| Alpha-UX-2 | UX-03,07,08,09 + AF-13,05 | 敢点启动、不盯着、挂了能救 |
| Alpha-UX-3 | UX-04a,06 + AF-14 | @ 团队 |
| Beta | UX-04b,10,11,12 + AF-15 | 叙事连续、Cursor 共生 |

Phase 未达标 → 不开下一 Phase 功能 PR。

---

## C. Feature Flags

| Flag | Epic |
|------|------|
| `p5_first_run_wizard` | AF-UX-01 |
| `p5_intent_console` | AF-UX-02 |
| `p5_run_next_step` | AF-UX-03 |
| `p5_launch_preview` | AF-UX-07 |
| `p5_approval_policy` | AF-UX-08 |
| `p5_failure_recovery` | AF-UX-09 |
| `p5_team_chat_at` | AF-UX-04a |
| `p5_team_chat_unified` | AF-UX-04b |

---

## D. 产品原则快查（v3）

| ID | 一句话 |
|----|--------|
| P1 | 意图优先，非 workflow |
| P2 | 上下文优先路由 |
| P3 | 首屏 ≤3 + 更多 |
| P4 | 进度可见 |
| P5 | 无死胡同 CTA |
| P6 | @ 优先 |
| P7 | 工厂主场 |
| P8 | **启动前 5 秒：时间/范围/成本可见** |
| P9 | **审批可减负（信任质量门）** |
| P10 | **失败可恢复（阶段重试）** |
| P11 | **异步厂长（离开 App 能闭环）** |

---

## E. AI 开发提示词（实现会话）

```
当前 Epic: AF-UX-XX（WIP=1，见 DECISIONS）
yaml v3.0 + GOVERNANCE + DECISIONS 为准
先 journey e2e，再实现；扩 scope 先 DECISIONS 批准
```
