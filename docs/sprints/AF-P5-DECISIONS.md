# AF-P5 规划决策日志

> **用途：** 记录「规划本身」的变更，防止多轮对话口头漂移。  
> **规则：** 未写入本文件、未经 `@qinyu` 批准的变更，不算正式规划。  
> **真相源：** [`AF-P5-unified-capabilities.yaml`](./AF-P5-unified-capabilities.yaml)

---

## 如何使用

| 动作 | 怎么做 |
|------|--------|
| 调整 Epic 顺序 | 本文件新增一条 → 改 yaml → 你回复「批准」 |
| AI 提新方向 | 先写「提案」小节，不直接改 yaml |
| 新会话对齐 | `@AF-P5-unified-capabilities.yaml` + 读本文件最近 3 条 |

---

## 决策模板（复制使用）

```markdown
### YYYY-MM-DD · 标题
- **类型：** 宪法 | 路线图 | Epic scope | 优先级 | 砍 scope
- **决定：**
- **原因：** （用户问题 / 数据 / dogfood）
- **影响 Epic：**
- **不变：** （仍遵守的原则或 explicit_not_doing）
- **批准：** @qinyu
```

---

## 已批准决策

### 2026-05-27 · v2.0 意图优先 + 单一输入框
- **类型：** 宪法 / 路线图
- **决定：** 工厂主路径改为意图输入框；workflow 内部路由；首屏 ≤3 chip
- **原因：** 6 张产线卡片对新用户认知负担过重
- **影响 Epic：** AF-UX-02，砍首屏卡片墙
- **不变：** 不取代 Cursor；引导+工作室双模式
- **批准：** @qinyu（对话确认）

### 2026-05-27 · v3.0 信任 / 异步 / 失败 / 审批负担
- **类型：** 路线图升级 v2.0 → v3.0
- **决定：**
  - 新增 Epic **AF-UX-07~09**（启动预览、审批策略+通知、失败恢复）
  - 新增 Epic **AF-UX-10~12**（时间线、Cmd+K 厂长台、Cursor 共生、轻量个性化）为 Beta
  - **Alpha-UX-2** 在 UX-03 后紧接 UX-07/08/09，再开团队向 Epic
  - **AF-UX-04 拆分：** 04a（@+角色名）→ 04b（合并 Tab）
  - **楔子承诺** 写入 north_star 副句（15min 可跑骨架 or 可 merge diff，否则可重试）
- **原因：** v2.0 解决「入口路径」，未解决「敢不敢点启动、要不要盯着、挂了怎么办」
- **影响 Epic：** execution_order_v3、release_phases、resource_priority_v3
- **不变：** dev-workflow 不进首屏；团队 Beta 前不阻塞 J1–J4
- **批准：** @qinyu（「可以 按 PM v3 这套走」）

### 2026-05-27 · 规划防跑偏机制
- **类型：** 治理
- **决定：** 建立 AF-P5-GOVERNANCE.md + journey e2e 契约 + gate-check；规划变更走本 DECISIONS 文件
- **原因：** 区分「软件实现跑偏」与「多轮 AI 规划跑偏」
- **批准：** @qinyu

---

### 2026-05-28 · 零用户阶段 — 战略重排
- **类型：** 优先级 / Phase 门槛
- **决定：**
  - **当前产品阶段：** `pre_launch`（0 付费/外测用户；目标 = 创始人 dogfood + GATE-2 十人朋友测）
  - **Alpha-UX-1 完成标准改为：** 创始人 15min 内 Golden Path or greenfield 跑通，可录屏演示
  - **Alpha-UX-2 收缩：** 仅保留 AF-UX-03（下一步 CTA）+ AF-UX-09（失败重试）；**AF-UX-07/08 后移** 至 GATE-2 通过后
  - **整包后移 Beta：** AF-UX-08（异步通知）、AF-UX-12（个性化）、AF-UX-10/11
  - **AF-UX-04 团队相关：** 全部后移至 **有 10 个朋友测用户之后**；0 用户阶段工厂单路径即可
  - **指标：** 暂不考核 w2_retention / approval_via_notification；只盯 `founder_golden_success` + `friend_test_3_of_10`
- **原因：** 用户原话「现在没有任何用户」— 留存/异步/审批减负优化无样本；先把楔子跑通给人看
- **影响 Epic：** execution_order 见 yaml `pre_launch_priority`；WIP 建议 AF-UX-02 或 AF-12
- **不变：** 楔子承诺、P1–P7 宪法、意图优先
- **批准：** @qinyu（对话确认）

---

### 2026-05-28 · 双引擎 — Claude Code + 多模型 API
- **类型：** 路线图新增 Epic 带 AF-MM-01~04
- **决定：**
  - 产品叙事 **「双引擎工厂」**，非「模型超市」
  - **代码车道** = Claude CLI（Golden Path 必须）；**文本车道** = API 多 provider
  - **MASTER-PLAN「多模型 UI Phase1 不做」** 收窄解释为：不做首屏选模型；**设置 + Text Lane 产品化允许**
  - **pre_launch：** AF-MM 默认 **GATE-2 前** 至少 AF-MM-01；若朋友无 CLI 则 **提前**
  - **AF-MM-03**（无 CLI 降级）post GATE-2；**AF-MM-04** 成本路由 Beta
- **原因：** 用户要求；且仅依赖 Claude Code 会阻塞朋友测；后端 AF-04b 已有，缺主路径产品化
- **不变：** API 不假装能 Edit/Bash；楔子改代码仍靠 Claude
- **批准：** @qinyu（对话确认）

---

## 待决提案（未批准 — 不得按此开发）

_（空：AI 新想法先写这里）_

<!-- 示例
### 提案 · 暂缓 dev-workflow 整个 Epic
- **建议：** AF-14 移至 Post-Beta
- **原因：** solo-dev 覆盖 80% 日常
- **等待：** @qinyu 批准/驳回
-->

---

## 宪法（本季度不轻易改）

1. **P1–P7** 见 yaml `product_principles`
2. **P8–P11** v3 新增（信任、异步、失败可恢复、审批可减负）
3. **楔子：** 15min 内可跑骨架 or 可 merge diff；否则明确卡点 + 一键重试
4. **不做：** 替代 Cursor、画布、模板市场、首屏 workflow 名

---

## 当前执行焦点（WIP=1）

| 字段 | 值 |
|------|-----|
| **产品阶段** | `pre_launch` — 0 外测用户 |
| **Phase** | Alpha-UX-1 |
| **North star 此刻** | 你能 15min 跑通 Golden Path，并能录屏给第一个朋友看 |
| **当前 Epic** | **AF-P5 五项收尾** — MM-02/03/04、UX-08/09、04b、dogfood、e2e 契约 |
| **Feature flag** | 全部 `p5_*` flags 默认 refined 模式开启 |
| **刻意不做** | API 模拟 Edit/Bash |
| **GATE-2 前视情况** | **AF-MM-01**（朋友若无 Claude CLI） |
| **下一里程碑** | GATE-2 十人朋友测（评估 Claude CLI 覆盖率） |

_（每完成一个 Epic 或阶段变更，更新此行 + 上方 DECISIONS 条目）_
