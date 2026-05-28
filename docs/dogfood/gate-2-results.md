# GATE-2 朋友测记录（模板）

> 10 人手动 dogfood，每人跑一遍 [GOLDEN-PATH.md](../GOLDEN-PATH.md)。  
> 通过标准见 [GATE-2-friend-test.yaml](../sprints/GATE-2-friend-test.yaml)。

## 汇总

| 指标 | 目标 | 实际 | 通过 |
|------|------|------|------|
| Golden Path 成功率 | ≥80% | _/_ | ☐ |
| time_to_first_diff 中位 | ≤15min | _ min | ☐ |
| 致命 bug 数 | 0 | _ | ☐ |
| terminal_fallback 比例 | ≤30% | _% | ☐ |

## 参与者记录

| # | 姓名 | 日期 | 成功 | 首 diff 耗时 | 用终端? | 备注 |
|---|------|------|------|-------------|---------|------|
| 1 | | | ☐ | | ☐ | |
| 2 | | | ☐ | | ☐ | |
| … | | | | | | |

## 指标导出

```bash
curl http://localhost:8080/api/v1/factory/metrics > gate-2-metrics.json
```

## Checklist 证据

- [ ] G2-1 macOS 包 Golden Path ≥80%
- [ ] G2-2 time_to_first_diff ≤15min
- [ ] G2-3 0 致命 bug
- [ ] G2-4 /factory 每人 ≥1 成功 Run
- [ ] G2-5 `cargo test -p nexus_workflow --lib` 通过
