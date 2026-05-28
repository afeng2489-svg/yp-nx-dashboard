# Golden Path — 一句话 → 15min diff → 审批

AF-04 定义的 killer 场景：用 **solo-dev** 工作流 + **solo-fullstack** 团队，在本地工作区完成一个小改动并走审批。

## 前置条件

| 项 | 要求 |
|---|---|
| Claude Code CLI | 已安装并在 **设置 → AI** 中检测通过 |
| 工作区 | 顶栏已选择目标项目文件夹（Claude 在此读写代码） |
| 团队 | 工厂台首次进入时创建 **Solo 全栈** 模板（或已有团队） |
| 网络 | 本地 `nx_api` 运行中（Tauri 桌面或 `cargo run --bin nx_api`） |

## 步骤（约 15 分钟）

1. 打开 **工厂台** → `/factory` → **Console** 标签
2. 点击快捷入口 **「试用演示」**（或手动输入任务）  
   预填任务：`给 README.md 增加「快速开始」安装步骤`
3. 确认底栏 WS 为 **已连接**（断线时会 **轮询中**，见 [ws-reliability.md](./ws-reliability.md)）
4. 在 **进行中的 Run** 卡片观察 stage：规划 → 实现 → 自测 → **交付审批** → 审查
5. 到 **交付审批** 时，右侧 **ContextPanel** 或 **Approvals** 标签出现 Approve / Reject
6. 点击 **Approve**，等待 Run 完成
7. 打开 **Deliverables** 标签，确认有 README 相关 diff / 产物
8. （可选）`GET /api/v1/factory/metrics` 查看 6 项本地指标

## 成功标准

- Run 状态变为 **completed**
- 工作区 `README.md`（或等价文件）有实质改动（非空 diff）
- 审批阶段可 Approve 且流程继续
- `golden_path_success` 指标记录为成功（见下方指标说明）

## 失败排查

| 现象 | 处理 |
|---|---|
| 「未绑定 CLI」 | 设置 → AI → 检测 Claude Code 路径 |
| 「请选择工作区」 | 顶栏选文件夹；Tauri 需对该目录有读写权限 |
| Run 一直 pending | 重启 `tauri:dev`；确认 `nx_api` 日志无 panic |
| WS 空闲但 Run 在跑 | 刷新页面；断线时应显示 **轮询中** |
| 405 on from-template | 重建 `cargo build --bin nx_api` 并重启 Tauri |
| 审批不出现 | 确认 `solo-dev.yaml` 含 `stage_type: approval` 阶段 |
| 15min 无 diff | 查看 Run 日志；CLI 权限 mode 是否过严 |

## 自动化验证

```bash
# API 冒烟（~4s，不跑完整 CLI）
cd nx_dashboard && npm run test:e2e:af-p1

# Golden Path（无 CLI 时自动 skip）
npm run test:e2e:golden-path

# Sprint 门禁片段
./scripts/gate-check.sh AF-04   # 检查本文档 + workflow 单测
```

## 6 项本地指标

埋点写入 SQLite `factory_events`，查询 `GET /api/v1/factory/metrics`：

| 指标 | 含义 | 目标（MASTER-PLAN） |
|---|---|---|
| `activation` | 首次打开工厂台 24h 内完成 1 次 Run | ≥60% |
| `golden_path_success` | Golden Path Run 完成率 | ≥80% |
| `time_to_first_diff` | Run 启动 → 首个产物（中位分钟） | ≤15min |
| `run_completion` | 所有 Run 完成率 | ≥70% |
| `terminal_fallback` | Run 期间打开终端占比 | ≤20% |
| `w2_retention` | 首次使用 14 天后再次打开工厂台 | ≥50% |

## G6：10 人手动 dogfood（AF-04 内）

招募 10 人（含非作者）按本文执行，记录：

- 成功 / 失败人数 → `golden_path_success`
- 是否中途打开 **终端** 页 → `terminal_fallback`
- 首个 diff 耗时 → `time_to_first_diff`

结果汇总到团队 wiki 或 Issue，AF-06 前复测。
