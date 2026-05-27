# 三轮交叉验证报告

**日期**：2026-05-27  
**方案**：AI 工厂 AF 计划（300–500 用户，Golden Path 杀手场景）

## 评审方式

- 原计划：Claude + GPT（Codex CLI）交叉验证三轮
- 实际：**Codex CLI 未安装**（`omc ask codex` 失败）
- 替代：Claude CLI 独立扮演「GPT 评审角色」；方案作者每轮回应并收敛

## 三轮评分

| 轮次 | 焦点 | 评分 | 成功概率 | 核心变化 |
|------|------|------|----------|----------|
| R1 | 严厉评审原方案 + 代码库 | 2.6/10 | 12% | v0–v4「假完成」、skip-permissions、Claude-only |
| R2 | 作者回应 + AF 计划澄清 | 4.2/10 | 25% | 消解 SQLite/多模型误解；计划未入库 |
| R3 | 三分歧对齐 + 最终 12 卡 | **7.2/10** | **62%** | 收敛为可执行方案 |

**R3 判决**：现在能开干（有条件）。前 5 张卡（AF-00 → AF-05）是可控区。

## R1 主要批评及 R3 处置

| 批评 | R3 状态 |
|------|---------|
| `--dangerously-skip-permissions` 硬编码 | 纳入 AF-00 |
| progress.json 过度宣称 completed | 改为 GATE 机器验证 |
| orchestrator `#![allow(dead_code)]` | 纳入 AF-00b |
| PageGenerate 硬编码 React | 不重构，标记 deprecated |
| 模型无关是空话 | Phase1 只承诺 Claude CLI；AF-04b 为后端 executor |
| v4.3 画布 80h 资源错配 | 已归档，不进 AF 计划 |
| SQLite vs 300 用户 | 误解已澄清：300=累计试用，单机合理 |

## R2/R3 三大分歧决议

### 1. 先修旧代码 vs 先做工厂台

**决议**：AF-00b 必须先做（约 2 天，锁文件清单），再 AF-01。不做全库 lint 归零。

### 2. GATE 是流程还是证据

**决议**：必须机器验证：

- GATE-1: `e2e/golden-path.spec.ts` 绿灯
- GATE-2: `cargo test -p nexus_workflow --lib` 通过
- GATE-3: `scripts/gate-check.sh` 验证 artifact 存在

`progress.json` 不再手改 completed。

### 3. Scope 控制

**决议**：MASTER-PLAN 写入 explicit_not_doing（10 条）+ 活跃卡上限 12 张。AF-06 为容器卡。

## 16 卡草案 → R3 最终方案

| 原 16 卡草案 | R3 收敛后 |
|--------------|-----------|
| AF-01 Team↔Workflow | 合并进 AF-01 工厂台 MVP（绑定在 Console 启动链） |
| AF-02 Factory Shell | 合并进 AF-01 |
| AF-03 5 项导航 | 合并进 AF-01 |
| AF-04 工厂台 4 Tab | 合并进 AF-01 |
| AF-05 产物 Tab | 合并进 AF-01（MVP 级） |
| AF-06 团队工作室 | 合并进 AF-01 Solo 模板 |
| AF-07 资产库+运营 | 延后 → AF-07（内测后） |
| AF-08 审批 harness | 提前 → AF-02 |
| AF-09 Golden Path | → AF-04 |
| AF-10 macOS 安装包 | → AF-05 |
| AF-11/12 50 人内测 | → AF-06 容器卡 |
| AF-13 ⌘K+响应式 | deferred_after_300 |
| AF-14 多团队+Sprint | → AF-08 |
| AF-15 300 人公测 | → AF-09 |
| AF-16 企业十项 | → AF-09（7/10 项） |
| （无） | **AF-00 / AF-00b** 新增前置 |
| （无） | **AF-04b** post-R3 增补（executor 双车道） |

## 仍须警惕的 Top 3 风险

1. **AF-00b scope creep** — 必须锁文件清单，2 天后强制进 AF-01
2. **GATE-3 人因漏洞** — yaml 填 true 不够，要验证 e2e artifact/日志
3. **AF-06 内测反馈爆炸** — 必须用容器卡机制

## AF-01 三条不可拆分验收（R2/R3 强制）

1. 顶栏可切换项目 + 团队，GlobalRunPanel 可见 Run
2. `/factory` 输入任务 → 启动 quick-fix/solo-dev → 15min 内看到 diff
3. Solo 模板 3 分钟创建团队，Golden Path 全程 0 次必须开终端

## 第四轮（可选）

若需真·GPT/Codex 评审：安装 Codex CLI 后运行 `omc ask codex`。
