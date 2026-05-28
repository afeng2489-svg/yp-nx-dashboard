# AF-06 50 人内测 Kit（模板）

> 依赖 **GATE-2** 通过后启动。代码与文档就绪后，由运营填本表。

## 招募（B1）

| 项 | 内容 |
|----|------|
| 目标人数 | 50 |
| 渠道 | 朋友圈 / 开发者社群 / Discord |
| 安装包 | GitHub Release `.dmg` 或内测链接 |
| 准入 | macOS + Claude CLI |

## W1 — Golden Path（B2）

每人完成 [GOLDEN-PATH.md](../GOLDEN-PATH.md)，记录：

```bash
curl http://localhost:8080/api/v1/factory/metrics > w1-metrics-$(date +%F).json
```

目标：`golden_path_success ≥ 70%`

## W2 — 真实小任务（B3）

每人 1 个真实小改动（非 README 演示），目标 `run_completion ≥ 60%`。

## Top5 修复（B4）

从反馈汇总 Top5 → 开 issue → 修完再扩量。

## Go / No-Go

| 条件 | 通过 |
|------|------|
| W1 ≥70% | ☐ |
| W2 ≥60% | ☐ |
| 无 P0 崩溃 | ☐ |
| → 进入 AF-09 公测招募 | ☐ |
