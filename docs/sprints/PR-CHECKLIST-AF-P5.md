## AF-P5 PR Checklist

**Epic ID:** `AF-UX-__` / `AF-12` / …  
**Plan:** [AF-P5-unified-capabilities.yaml](./AF-P5-unified-capabilities.yaml)

### Scope

- [ ] 本 PR 只做一个 Epic（或一个 Epic 内明确子任务）
- [ ] `in_scope` 已满足；`out_of_scope` 未触碰
- [ ] 扩 scope 已先更新 yaml（如有）

### 产品原则（动 UI 必填）

- [ ] P1 用户不需选 workflow 名即可启动（或本 PR 不碰启动路径）
- [ ] P3 引导模式首屏主操作仍 ≤3（或本 PR 不碰 Factory 首屏）
- [ ] P5 若动 Run 完成态，有明确「下一步」CTA
- [ ] 未向用户暴露 Tier / workflow 内部名（除非工作室折叠区）

### 自动化

- [ ] `./scripts/gate-check.sh <EPIC_ID>` PASS
- [ ] `npx playwright test e2e/journey-af-p5.spec.ts -g "<EPIC_ID>"` PASS
- [ ] `npm run test:unit` PASS（若改 nx_dashboard 逻辑）

### Dogfood（15 分钟）

**路径：**

1. …
2. …

**结果：** 通过 / 阻塞（issue #）

### 截图 / 录屏

- [ ] 附引导模式 Factory 首屏（若相关）
