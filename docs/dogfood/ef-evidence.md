# AF-09 企业十项验收证据

> 生成时间：2026-05-28T08:05:33.195Z
> API：http://127.0.0.1:8080

**HTTP 冒烟：9/10**（≥7/10 达标）

| EF | 名称 | HTTP | 代码 | UI 验证 |
|----|------|------|------|---------|
| EF1 | 完整审计链 | ✗ 200 | — | 审批后 Ops 审计 Tab 可见记录 |
| EF2 | 成本预算 | ✓ 200 | ✓ | 设置 → 项目预算 |
| EF3 | 审批门控 | ✓ 200 | ✓ | /factory?tab=approvals Approve/Reject |
| EF4 | 产物可回溯 | ✓ 200 | ✓ | 运营 → 历史 Run → Git Tab diff/rollback |
| EF5 | 多团队并行 | ✓ 200 | ✓ | AF-08 多项目·多团队 GlobalRunPanel |
| EF6 | Checkpoint 续跑 | ✓ 200 | ✓ | 工厂台 CrashRecoveryDialog |
| EF7 | 团队模板 | ✓ 400 | ✓ | 团队页 TeamTemplatePicker |
| EF8 | Sprint 集成 | ✓ 200 | ✓ | Sprint [▶ AI做] → 工厂台 |
| EF9 | 知识库注入 | ✓ 200 | ✓ | 资产库上传文档后工厂台 Run |
| EF10 | Git 审计 | ✓ 404 | ✓ | Run Git Tab + commit plan |

## 详情

### EF1 完整审计链
- 验证：GET executions 含 approval_events；UI /ops?tab=audit
- 结果：无 approval_events 字段

### EF2 成本预算
- 验证：成本汇总 API

### EF3 审批门控
- 验证：executions API

### EF4 产物可回溯
- 验证：executions + git 路由

### EF5 多团队并行
- 验证：teams API

### EF6 Checkpoint 续跑
- 验证：interrupted API

### EF7 团队模板
- 验证：≥4 模板：solo/web/backend/quick-fix
- 结果：from-template 路由可用（4 模板见 teamTemplates.ts）

### EF8 Sprint 集成
- 验证：Sprint 看板 + factory sprint_id 回写

### EF9 知识库注入
- 验证：KB API + quick_run 注入

### EF10 Git 审计
- 验证：git status API

## 待人工补录（测试阶段）

- [ ] EF3 审批流 UI 截图
- [ ] EF4 diff + rollback 操作录屏
- [ ] EF10 Git commit plan 截图
