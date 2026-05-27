# 已归档 Sprint（v0–v4）

共 21 张 sprint 卡（progress.json 曾标记 24 completed，含 v4.4–v4.6 无独立 yaml）。

**勿再执行**。能力已并入产品或映射到新 AF 计划。

新计划从 **AF-00** 开始。读 [`../MASTER-PLAN.yaml`](../MASTER-PLAN.yaml)。

## 归档索引

| Sprint | 文件 | 能力摘要 |
|--------|------|----------|
| v0.0.1 | v0.0.1-critical-bug-fixes.yaml | expect 崩溃、Mutex 统一 |
| v0.0.2 | v0.0.2-deploy-auth-api.yaml | A2UI 路由、API 信封、Docker |
| v0.0.3 | v0.0.3-unified-scheduler.yaml | 接入 core/orchestrator |
| v0.1-S1 | v0.1-S1-artifact-management.yaml | 产物 tracker/repository |
| v0.1-S2 | v0.1-S2-zip-export.yaml | zip 导出 |
| v0.1-S3 | v0.1-S3-stage-grouping.yaml | stage 分组视图 |
| v1.1 | v1.1-pipeline-execution.yaml | Pipeline dispatch 回调 |
| v1.2 | v1.2-checkpoint-resume.yaml | 断点续跑 |
| v1.3 | v1.3-quality-gate.yaml | 质量门自动化 |
| v1.4 | v1.4-observability.yaml | 进度/成本/产物看板 |
| v2.1 | v2.1-chat-experience.yaml | CLI 优先团队对话 |
| v2.3 | v2.3-project-awareness.yaml | 项目状态感知 |
| v2.4 | v2.4-skill-system-internalization.yaml | 技能内置化 |
| v3.1 | v3.1-git-integration.yaml | Git 集成 |
| v3.2 | v3.2-triggers.yaml | Cron/Webhook/链式触发 |
| v3.3 | v3.3-token-cost.yaml | Token/Cost 监控 |
| v3.4 | v3.4-rag-knowledge-base.yaml | RAG 知识库 |
| v4.1 | v4.1-multi-model-routing.yaml | 多模型路由（部分：仅 CLI --model） |
| v4.2 | v4.2-failure-recovery.yaml | 失败自愈 |
| v4.3 | v4.3-visual-canvas.yaml | 低代码画布 |
| v4.7 | v4.7-ai-project-tracking.yaml | Sprint 看板 / AI 项目追踪 |

## 能力 → 新 AF 计划映射

| 能力 | 来源 Sprint | 在新计划中的位置 |
|------|-------------|------------------|
| Pipeline 回调 | v1.1 | 已有；AF-01 统一 Run 展示 |
| Checkpoint | v1.2 | 已有；AF-09 企业验收 |
| Quality Gate | v1.3 | 已有；Golden Path 沿用 |
| 产物/成本/进度 | v1.4 | AF-01 工厂台、AF-07 运营中心 |
| CLI 对话 | v2.1 | AF-01 团队 Tab |
| Git 合入 | v3.1 | AF-01 产物操作 |
| Cost | v3.3 | AF-07 ops Tab |
| RAG | v3.4 | AF-07 知识库、AF-09 验收 |
| Canvas | v4.3 | AF-01 Runs 详情（只读） |
| Sprint 看板 | v4.7 | AF-07、AF-08 |
| 多模型 executor 分流 | v4.1 部分 | **AF-04b** |

## v4.1 备注

`multi-model routing` 在代码中仅为 Claude CLI 的 `--model` 切换；真正的 executor 双车道（CLI vs API）见 **AF-04b**。
