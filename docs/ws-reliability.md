# WebSocket 可靠性（AF-03）

工厂台 Run 进度通过 `executionStore.connectWebSocket(executionId)` 订阅
`/ws/executions/:id`。断线时自动降级为 REST 轮询，避免 UI 冻结。

## 连接状态

| 状态 | 含义 |
|------|------|
| `connecting` | 首次建立 WS |
| `connected` | WS 正常，实时事件 |
| `reconnecting` | WS 断开后指数退避重连（1s → 32s） |
| `polling` | WS 不可用，每 **3s** `GET /api/v1/executions/:id` |
| `disconnected` | 内部过渡态，用户可见为 reconnecting / polling |

底栏 `FactoryStatusBar` 聚合所有活跃 Run 的 WS 状态：
**已连接 / 重连中 / 轮询中 / 空闲**。

## 事件类型（WS → `executionStore`）

| 事件 | 前端行为 |
|------|----------|
| `snapshot` | 重连 catch-up：`status`、`stage_results`、`current_stage`、`pending_pause`、`output_log` |
| `started` | 追加输出行 |
| `stage_started` | 设置 `execution.current_stage`，状态 `running` |
| `stage_completed` | 追加 `stage_results`，清空 `current_stage` |
| `output` | 追加 CLI 输出行 |
| `workflow_paused` | `pendingPause` + `status=paused`（含 `pause_kind=approval`） |
| `workflow_resumed` | 清除 pause，恢复 `running` |
| `completed` / `failed` | 终态，停止 poll，清理连接 |
| `token_usage` | 更新 token / cost |
| `pong` | 心跳响应，忽略 |

## Poll fallback

触发条件：

- WS `onclose`（Run 未终态）
- WS 连接异常（`catch`）

行为：

1. 立即 `GET /executions/:id` 合并到 `executions` 列表
2. 每 3s 重复，直到 WS 重连成功或 Run 结束
3. WS `onopen` 时停止 poll；服务端发送 `snapshot` 补齐遗漏

Poll 与 WS 使用同一 `Execution` 形状（含 `current_stage`、`pending_pause`、`approval_events`）。

## 谁负责建连

- `ActiveExecutionsPanel`：`running` / `paused` 的 execution 自动 `connectWebSocket`
- `resolveExecution` / quick-run 成功后也会建连

## 手动验证

1. `/factory` 启动 solo-dev Run，观察 Console 卡片 stage 实时变化
2. 开发工具 Network → 禁用 WS 或杀 `nx_api` 几秒：底栏应显示 **轮询中**，stage 仍更新
3. 恢复 API：底栏回到 **已连接**，进度不丢
4. 跑到「交付审批」断线：poll 应仍显示 `paused` + ApprovalPanel
