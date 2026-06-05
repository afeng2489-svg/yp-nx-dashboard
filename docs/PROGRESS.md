# NexusFlow — AI 软件工厂 开发进度

> 文档地图：[README.md](README.md)  
> **AI 恢复指令**：读 [`docs/sprints/MASTER-PLAN.yaml`](sprints/MASTER-PLAN.yaml) + [`docs/progress.json`](progress.json) → 找 `current_sprint` → 读对应 sprint yaml。

---

## 产品定位

**目标**：300–500 独立开发者 — 工厂台一句话 → 15min diff → 审批 → 合入。

**定位**：本地 AI 开发部 — 不取代 Cursor，取代终端里的 claude 编排。

三轮验证：[`VALIDATION-3ROUND.md`](sprints/VALIDATION-3ROUND.md)（R3 **7.2/10**，62%）。

---

## 当前状态（2026-05-28）

| 字段 | 值 |
|------|-----|
| **代码完成度** | AF-00 ~ AF-10 已实现；**AF-11** P1–P5 完成（三模式 + shadcn 样板三页） |
| **待你测试** | GATE-2、AF-10 验收、**AF-11 布局切换 + 视觉收敛** |
| **当前 Sprint** | AF-11（P6 Shell 打磨 / P7 e2e 待完成） |
| **Master Plan** | [`MASTER-PLAN.yaml`](sprints/MASTER-PLAN.yaml) |

---

## AF 计划进度

```
AF-P0 安全与卫生     ████████████████████  100%  AF-00, AF-00b
AF-P1 工厂台 MVP     ████████████████████  100%  AF-01, AF-02, AF-03
AF-P2 Golden Path    ████████████████░░░░   80%  AF-04/04b/05 ✓ · GATE-2 待测
AF-P3 内测           ░░░░░░░░░░░░░░░░░░░░    0%  AF-06 待 GATE-2
AF-P4 公测扩展       ████████████████░░░░   85%  AF-07/08 ✓ · AF-09 代码就绪
```

| Sprint | 标题 | 代码 | 人工验证 |
|--------|------|------|----------|
| AF-00 ~ AF-04 | 安全 / 工厂台 / Golden Path | ✅ | GATE-1 e2e 可 skip |
| AF-04b, AF-05 | Executor / macOS 包 | ✅ | Release 安装 |
| GATE-2 | 10 人朋友测 | 模板就绪 | ⏸ 待填 gate-2-results |
| AF-06 | 50 人内测 | kit 就绪 | ⏸ 待招募 |
| AF-07, AF-08 | 资产库 / 多团队 Sprint | ✅ | — |
| AF-09 | 300 人公测 | ✅ 代码 | ⏸ 招募 + NPS |

---

## 测试前检查清单

```bash
# 1. 后端 + 桌面
cd nx_dashboard && npm run tauri:dev

# 2. 工作流单测
cargo test -p nexus-workflow --lib

# 3. 企业 EF 冒烟 + 报告
cd nx_dashboard && npm run ef:check:report

# 4. Sprint gate（可选，会更新 progress.json）
./scripts/gate-check.sh AF-09
```

证据文件：

- [dogfood/gate-2-results.md](dogfood/gate-2-results.md) — 朋友测
- [dogfood/ef-evidence.md](dogfood/ef-evidence.md) — 企业十项
- [dogfood/af-06-beta-kit.md](dogfood/af-06-beta-kit.md) — 内测

---

## 恢复 SOP

1. 读 `MASTER-PLAN.yaml` → `execution_order`
2. 读 `progress.json` → `current_sprint`
3. 若 `ready-for-testing` → 按上方检查清单安排测试
4. 测试通过后仅通过 `scripts/gate-check.sh` 更新 `completed` 计数
