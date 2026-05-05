# NexusFlow 全功能端到端测试卡片

> **AI 执行指令**：按卡片顺序执行，每张卡片独立。状态：⬜未测 / ✅通过 / ❌失败 / ⚠️部分通过

---

## 环境准备

```bash
cargo build --workspace 2>&1 | tail -3
cargo run --bin nx_api &
sleep 3
curl -s http://localhost:3000/health
```

---

## CARD-01 · 健康检查 ⬜
**路由**：`GET /health`
```bash
curl -s http://localhost:3000/health
```
**预期**：`{"status":"ok","service":"nexusflow-api"}`

---

## CARD-02 · Workflow CRUD ⬜
**路由**：`/api/v1/workflows`
```bash
# 创建
WF=$(curl -s -X POST http://localhost:3000/api/v1/workflows \
  -H "Content-Type: application/json" \
  -d '{"name":"e2e-wf","description":"test","stages":[{"name":"s1","prompt":"hello"}]}')
WF_ID=$(echo $WF | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

# 查询
curl -s http://localhost:3000/api/v1/workflows/$WF_ID

# 更新
curl -s -X PUT http://localhost:3000/api/v1/workflows/$WF_ID \
  -H "Content-Type: application/json" -d '{"name":"e2e-wf-updated"}'

# 列表
curl -s http://localhost:3000/api/v1/workflows

# 删除
curl -s -X DELETE http://localhost:3000/api/v1/workflows/$WF_ID
```
**预期**：CRUD 全部返回 2xx，列表包含新建项

---

## CARD-03 · Execution 执行生命周期 ⬜
**路由**：`/api/v1/executions`
```bash
# 先创建 workflow（复用 CARD-02 的 WF_ID）
EX=$(curl -s -X POST http://localhost:3000/api/v1/executions \
  -H "Content-Type: application/json" \
  -d "{\"workflow_id\":\"$WF_ID\",\"input\":{}}")
EX_ID=$(echo $EX | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

# 轮询状态
for i in 1 2 3 4 5; do
  STATUS=$(curl -s http://localhost:3000/api/v1/executions/$EX_ID | \
    python3 -c "import sys,json;print(json.load(sys.stdin)['data']['status'])")
  echo "[$i] status: $STATUS"
  [ "$STATUS" = "completed" ] || [ "$STATUS" = "failed" ] && break
  sleep 5
done

# 列表
curl -s http://localhost:3000/api/v1/executions
```
**预期**：状态从 `pending`→`running`→`completed`

---

## CARD-04 · Execution Git 信息 ⬜
**路由**：`GET /api/v1/executions/:id/git`
```bash
curl -s http://localhost:3000/api/v1/executions/$EX_ID/git
```
**预期**：返回 commit hash 或 diff 信息（需工作区为 git 仓库）

---

## CARD-05 · Session 生命周期 ⬜
**路由**：`/api/v1/sessions`
```bash
# 创建
SS=$(curl -s -X POST http://localhost:3000/api/v1/sessions \
  -H "Content-Type: application/json" \
  -d "{\"workflow_id\":\"$WF_ID\"}")
SS_ID=$(echo $SS | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

# 获取消息
curl -s http://localhost:3000/api/v1/sessions/$SS_ID/messages

# 暂停
curl -s -X POST http://localhost:3000/api/v1/sessions/$SS_ID/pause

# 同步
curl -s -X POST http://localhost:3000/api/v1/sessions/$SS_ID/sync

# 删除
curl -s -X DELETE http://localhost:3000/api/v1/sessions/$SS_ID
```
**预期**：各操作返回 2xx，状态变更正确

---

## CARD-06 · Session 消息响应 ⬜
**路由**：`POST /api/v1/sessions/:id/messages/:msg_id/respond`
```bash
# 获取消息列表，取第一条 msg_id
MSG_ID=$(curl -s http://localhost:3000/api/v1/sessions/$SS_ID/messages | \
  python3 -c "import sys,json;msgs=json.load(sys.stdin)['data'];print(msgs[0]['id'] if msgs else 'none')")

curl -s -X POST http://localhost:3000/api/v1/sessions/$SS_ID/messages/$MSG_ID/respond \
  -H "Content-Type: application/json" \
  -d '{"response":"approved","content":"looks good"}'
```
**预期**：返回 2xx，消息状态更新为已响应

---

## CARD-07 · Group Sessions 群组会话 ⬜
**路由**：`/api/v1/group-sessions`
```bash
# 创建群组会话
GS=$(curl -s -X POST http://localhost:3000/api/v1/group-sessions \
  -H "Content-Type: application/json" \
  -d '{"name":"e2e-group","members":["agent-1","agent-2"]}')
GS_ID=$(echo $GS | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

# 获取
curl -s http://localhost:3000/api/v1/group-sessions/$GS_ID

# 列表
curl -s http://localhost:3000/api/v1/group-sessions
```
**预期**：创建成功，列表包含新建项

---

## CARD-08 · Teams 团队管理 ⬜
**路由**：`/api/v1/teams`
```bash
# 创建团队
TM=$(curl -s -X POST http://localhost:3000/api/v1/teams \
  -H "Content-Type: application/json" \
  -d '{"name":"e2e-team","description":"test team"}')
TM_ID=$(echo $TM | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

# 获取团队角色
curl -s http://localhost:3000/api/v1/teams/$TM_ID/roles

# 列表
curl -s http://localhost:3000/api/v1/teams
```
**预期**：团队创建成功，角色列表可查询

---

## CARD-09 · Roles 角色管理 ⬜
**路由**：`/api/v1/roles`
```bash
# 创建角色
RL=$(curl -s -X POST http://localhost:3000/api/v1/roles \
  -H "Content-Type: application/json" \
  -d '{"name":"developer","description":"code role","team_id":"'$TM_ID'"}')
RL_ID=$(echo $RL | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

# 获取角色技能
curl -s http://localhost:3000/api/v1/roles/$RL_ID/skills

# 获取角色所属团队
curl -s http://localhost:3000/api/v1/roles/$RL_ID/team

# 执行角色
curl -s -X POST http://localhost:3000/api/v1/roles/$RL_ID/execute \
  -H "Content-Type: application/json" \
  -d '{"prompt":"write hello world"}'
```
**预期**：角色 CRUD 正常，执行返回 AI 输出

---

## CARD-10 · Tasks 任务管理 ⬜
**路由**：`/api/v1/tasks`
```bash
# 创建任务
TK=$(curl -s -X POST http://localhost:3000/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{"title":"e2e-task","description":"test","status":"pending"}')
TK_ID=$(echo $TK | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

# 更新状态
curl -s -X PUT http://localhost:3000/api/v1/tasks/$TK_ID \
  -H "Content-Type: application/json" -d '{"status":"in_progress"}'

# 统计
curl -s http://localhost:3000/api/v1/tasks/stats

# 列表
curl -s http://localhost:3000/api/v1/tasks
```
**预期**：任务状态流转正常，stats 返回各状态计数

---

## CARD-11 · Projects 项目管理 ⬜
**路由**：`/api/v1/projects`
```bash
PJ=$(curl -s -X POST http://localhost:3000/api/v1/projects \
  -H "Content-Type: application/json" \
  -d '{"name":"e2e-project","description":"test"}')
PJ_ID=$(echo $PJ | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

curl -s http://localhost:3000/api/v1/projects/$PJ_ID
curl -s http://localhost:3000/api/v1/projects
curl -s -X DELETE http://localhost:3000/api/v1/projects/$PJ_ID
```
**预期**：CRUD 全部 2xx

---

## CARD-12 · Issues 问题追踪 ⬜
**路由**：`/api/v1/issues`
```bash
IS=$(curl -s -X POST http://localhost:3000/api/v1/issues \
  -H "Content-Type: application/json" \
  -d '{"title":"e2e-issue","description":"test bug","severity":"medium"}')
IS_ID=$(echo $IS | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

curl -s -X PUT http://localhost:3000/api/v1/issues/$IS_ID \
  -H "Content-Type: application/json" -d '{"status":"resolved"}'

curl -s http://localhost:3000/api/v1/issues
```
**预期**：Issue 创建、状态更新、列表查询正常

---

## CARD-13 · Skills 技能系统 ⬜
**路由**：`/api/v1/skills`
```bash
# 列表（预置技能）
curl -s http://localhost:3000/api/v1/skills

# 分类
curl -s http://localhost:3000/api/v1/skills/categories

# 标签
curl -s http://localhost:3000/api/v1/skills/tags

# 统计
curl -s http://localhost:3000/api/v1/skills/stats

# 搜索
curl -s "http://localhost:3000/api/v1/skills/search?query=code"

# 按标签筛选
curl -s http://localhost:3000/api/v1/skills/tag/code

# 执行（取第一个技能 id）
SK_ID=$(curl -s http://localhost:3000/api/v1/skills | \
  python3 -c "import sys,json;d=json.load(sys.stdin)['data'];print(d[0]['id'] if d else '')")
curl -s -X POST http://localhost:3000/api/v1/skills/$SK_ID/execute \
  -H "Content-Type: application/json" -d '{"params":{}}'
```
**预期**：预置技能列表非空，执行返回 `success:true`

---

## CARD-14 · Skills 导入 ⬜
**路由**：`POST /api/v1/skills/import`
```bash
curl -s -X POST http://localhost:3000/api/v1/skills/import \
  -H "Content-Type: application/json" \
  -d '{"source":"paste","content":"{\"id\":\"test-skill\",\"name\":\"Test\",\"description\":\"e2e\",\"category\":\"test\",\"code\":\"echo hello\"}"}'
```
**预期**：返回导入的技能详情，技能列表新增该项

---

## CARD-15 · AI Config — CLI 管理 ⬜
**路由**：`/api/v1/ai/clis`
```bash
# 列出可用 CLI
curl -s http://localhost:3000/api/v1/ai/clis

# 更新 CLI 配置
curl -s -X PUT http://localhost:3000/api/v1/ai/clis/config \
  -H "Content-Type: application/json" \
  -d '{"cli":"claude","enabled":true}'
```
**预期**：CLI 列表返回 claude/gemini 等，配置更新 2xx

---

## CARD-16 · AI Config — 模型管理 ⬜
**路由**：`/api/v1/ai/models`
```bash
# 列出模型
curl -s http://localhost:3000/api/v1/ai/models

# 当前选中模型
curl -s http://localhost:3000/api/v1/ai/selected

# 设置默认模型
curl -s -X PUT http://localhost:3000/api/v1/ai/default \
  -H "Content-Type: application/json" \
  -d '{"model_id":"claude-sonnet-4-5","provider":"anthropic"}'

# 刷新模型列表
curl -s -X POST http://localhost:3000/api/v1/ai/models/refresh
```
**预期**：模型列表非空，selected 返回当前模型

---

## CARD-17 · AI Provider V2 ⬜
**路由**：`/api/v1/ai/v2/providers`
```bash
# 列出 providers
curl -s http://localhost:3000/api/v1/ai/v2/providers

# 创建 provider
PV=$(curl -s -X POST http://localhost:3000/api/v1/ai/v2/providers \
  -H "Content-Type: application/json" \
  -d '{"name":"test-provider","provider_key":"openai","api_key":"sk-test","base_url":"https://api.openai.com"}')
PV_ID=$(echo $PV | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

# 删除
curl -s -X DELETE http://localhost:3000/api/v1/ai/v2/providers/$PV_ID
```
**预期**：Provider CRUD 正常

---

## CARD-18 · AI 直接执行 ⬜
**路由**：`POST /api/v1/ai/execute`、`POST /api/v1/ai/chat`
```bash
# 执行 CLI
curl -s -X POST http://localhost:3000/api/v1/ai/execute \
  -H "Content-Type: application/json" \
  -d '{"prompt":"say hello","cli":"claude"}'

# 对话
curl -s -X POST http://localhost:3000/api/v1/ai/chat \
  -H "Content-Type: application/json" \
  -d '{"message":"hello","context":[]}'
```
**预期**：返回 AI 输出文本（需 CLI 工具已安装）

---

## CARD-19 · API Keys 管理 ⬜
**路由**：`/api/v1/ai/api-keys`
```bash
curl -s http://localhost:3000/api/v1/ai/api-keys
```
**预期**：返回已配置的 API key 列表（key 值脱敏）

---

## CARD-20 · Model Routing 多模型路由 ⬜
**路由**：`/api/v1/ai/cli-model`、model_routing 路由
```bash
# 获取当前 CLI 模型绑定
curl -s http://localhost:3000/api/v1/ai/cli-model

# 获取路由配置
curl -s http://localhost:3000/api/v1/model-routing/config 2>/dev/null || \
  curl -s http://localhost:3000/api/v1/ai/providers
```
**预期**：返回路由规则配置

---

## CARD-21 · Costs 费用统计 ⬜
**路由**：`/api/v1/costs/summary`、`/api/v1/costs/by-day`
```bash
curl -s http://localhost:3000/api/v1/costs/summary
curl -s "http://localhost:3000/api/v1/costs/by-day?days=7"
```
**预期**：summary 返回总 token/费用，by-day 返回按天分组数据

---

## CARD-22 · Workspaces 工作区 ⬜
**路由**：`/api/v1/workspaces`
```bash
mkdir -p /tmp/e2e-ws && git -C /tmp/e2e-ws init 2>/dev/null

WS=$(curl -s -X POST http://localhost:3000/api/v1/workspaces \
  -H "Content-Type: application/json" \
  -d '{"name":"e2e-ws","root_path":"/tmp/e2e-ws"}')
WS_ID=$(echo $WS | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

# 文件浏览
curl -s "http://localhost:3000/api/v1/workspaces/$WS_ID/browse"

# Git diff
echo "test" > /tmp/e2e-ws/new.txt
curl -s http://localhost:3000/api/v1/workspaces/$WS_ID/diffs

# Git status
curl -s http://localhost:3000/api/v1/workspaces/$WS_ID/git/status

# 删除工作区
curl -s -X DELETE http://localhost:3000/api/v1/workspaces/$WS_ID
```
**预期**：文件浏览返回目录列表，diffs 包含 `new.txt`

---

## CARD-23 · Knowledge 知识库 ⬜
**路由**：`/api/v1/knowledge`（knowledge.rs）
```bash
# 上传文档
echo "NexusFlow 是 AI 软件工厂" > /tmp/kb-doc.txt
curl -s -X POST http://localhost:3000/api/v1/knowledge \
  -F "file=@/tmp/kb-doc.txt"

# 列表
curl -s http://localhost:3000/api/v1/knowledge

# 搜索
curl -s "http://localhost:3000/api/v1/knowledge/search?q=AI软件工厂"
```
**预期**：上传后分块数 > 0，搜索返回相关 chunks

---

## CARD-24 · Triggers 触发器 ⬜
**路由**：`/api/v1/triggers`（triggers.rs）
```bash
TR=$(curl -s -X POST http://localhost:3000/api/v1/triggers \
  -H "Content-Type: application/json" \
  -d "{\"name\":\"e2e-trigger\",\"type\":\"cron\",\"cron\":\"* * * * *\",\"workflow_id\":\"$WF_ID\"}")
TR_ID=$(echo $TR | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

curl -s http://localhost:3000/api/v1/triggers
sleep 65
# 验证触发器创建了新执行
curl -s http://localhost:3000/api/v1/executions | \
  python3 -c "import sys,json;d=json.load(sys.stdin)['data'];print(len(d),'executions')"

curl -s -X DELETE http://localhost:3000/api/v1/triggers/$TR_ID
```
**预期**：65 秒后执行列表新增 1 条触发器创建的记录

---

## CARD-25 · Scheduler 调度器 ⬜
**路由**：`/api/v1/scheduler`（scheduler.rs）
```bash
curl -s http://localhost:3000/api/v1/scheduler/jobs 2>/dev/null || \
  curl -s http://localhost:3000/api/v1/scheduler/status
```
**预期**：返回当前调度任务列表及状态

---

## CARD-26 · Templates 模板 ⬜
**路由**：`/api/v1/templates`
```bash
TP=$(curl -s -X POST http://localhost:3000/api/v1/templates \
  -H "Content-Type: application/json" \
  -d '{"name":"e2e-template","content":"# Template\n{{prompt}}","category":"code"}')
TP_ID=$(echo $TP | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

curl -s http://localhost:3000/api/v1/templates/$TP_ID
curl -s http://localhost:3000/api/v1/templates
curl -s -X DELETE http://localhost:3000/api/v1/templates/$TP_ID
```
**预期**：模板 CRUD 正常

---

## CARD-27 · Search 全局搜索 ⬜
**路由**：`/api/v1/search`
```bash
# 搜索
curl -s "http://localhost:3000/api/v1/search?q=workflow"

# 获取搜索模式
curl -s http://localhost:3000/api/v1/search/modes

# 重建索引
curl -s -X POST http://localhost:3000/api/v1/search/index
```
**预期**：搜索返回跨模块结果，modes 返回可用搜索模式

---

## CARD-28 · Wisdom 知识沉淀 ⬜
**路由**：`/api/v1/wisdom`
```bash
WD=$(curl -s -X POST http://localhost:3000/api/v1/wisdom \
  -H "Content-Type: application/json" \
  -d '{"title":"e2e-wisdom","content":"test insight","category":"engineering"}')
WD_ID=$(echo $WD | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

curl -s http://localhost:3000/api/v1/wisdom/categories
curl -s "http://localhost:3000/api/v1/wisdom/search?q=insight"
curl -s -X DELETE http://localhost:3000/api/v1/wisdom/$WD_ID
```
**预期**：Wisdom CRUD + 搜索正常

---

## CARD-29 · Test Gen 测试生成 ⬜
**路由**：`/api/v1/test-gen`
```bash
# 生成测试
curl -s -X POST http://localhost:3000/api/v1/test-gen \
  -H "Content-Type: application/json" \
  -d '{"code":"fn add(a: i32, b: i32) -> i32 { a + b }","language":"rust"}'

# 生成单元测试
curl -s -X POST http://localhost:3000/api/v1/test-gen/unit \
  -H "Content-Type: application/json" \
  -d '{"function":"add","language":"rust"}'
```
**预期**：返回生成的测试代码（需 CLI 工具）

---

## CARD-30 · Plugins 插件 ⬜
**路由**：`/api/v1/plugins`
```bash
curl -s http://localhost:3000/api/v1/plugins
```
**预期**：返回已安装插件列表（可为空）

---

## CARD-31 · Processes 进程管理 ⬜
**路由**：`GET /api/v1/processes`
```bash
curl -s http://localhost:3000/api/v1/processes
```
**预期**：返回当前运行中的进程列表（PTY/CLI 进程）

---

## CARD-32 · Execution Logs 执行日志 ⬜
**路由**：execution_logs.rs
```bash
# 获取执行日志（复用 CARD-03 的 EX_ID）
curl -s http://localhost:3000/api/v1/executions/$EX_ID/logs 2>/dev/null || \
  curl -s "http://localhost:3000/api/v1/execution-logs?execution_id=$EX_ID"
```
**预期**：返回日志数组，每条含 `timestamp`、`level`、`message`

---

## CARD-33 · WebSocket — 执行实时推送 ⬜
**路由**：`WS /ws/executions/:id`
```bash
# 需要 websocat：brew install websocat
websocat ws://localhost:3000/ws/executions/$EX_ID &
WS_PID=$!
sleep 10
kill $WS_PID
```
**预期**：连接建立后收到执行状态变更消息

---

## CARD-34 · WebSocket — Session 流 ⬜
**路由**：`WS /ws/sessions/:id`、`WS /ws/claude-stream`
```bash
websocat ws://localhost:3000/ws/sessions/$SS_ID &
WS_PID=$!
sleep 5
kill $WS_PID
```
**预期**：连接建立，收到 session 消息推送

---

## CARD-35 · WebSocket — 终端 & 命令执行 ⬜
**路由**：`WS /ws/terminal`、`WS /ws/run-command`
```bash
# 测试终端 WS 连接
websocat ws://localhost:3000/ws/terminal &
WS_PID=$!
sleep 3
kill $WS_PID
```
**预期**：WS 连接建立，不立即断开

---

## CARD-36 · Tauri 桌面应用启动 ⬜
**类型**：桌面应用手动验证
```bash
cd /Users/Zhuanz/Desktop/yp-nx-dashboard/nx_dashboard
cargo tauri dev
```
**验证清单**：
- [ ] 应用窗口正常打开，无崩溃
- [ ] 侧边栏所有菜单项可点击，无白屏
- [ ] 技能页面正常加载（已修复 `error` 未解构 bug）
- [ ] DevTools 控制台无 `Error` 级别报错
- [ ] 执行页面实时更新（WS 连接正常）

---

## 汇总表（36 张卡片）

| 卡片 | 功能模块 | 路由/类型 | 状态 |
|------|---------|-----------|------|
| CARD-01 | 健康检查 | `GET /health` | ⬜ |
| CARD-02 | Workflow CRUD | `/api/v1/workflows` | ⬜ |
| CARD-03 | Execution 生命周期 | `/api/v1/executions` | ⬜ |
| CARD-04 | Execution Git 信息 | `/executions/:id/git` | ⬜ |
| CARD-05 | Session 生命周期 | `/api/v1/sessions` | ⬜ |
| CARD-06 | Session 消息响应 | `/sessions/:id/messages/:id/respond` | ⬜ |
| CARD-07 | Group Sessions | `/api/v1/group-sessions` | ⬜ |
| CARD-08 | Teams 团队 | `/api/v1/teams` | ⬜ |
| CARD-09 | Roles 角色 | `/api/v1/roles` | ⬜ |
| CARD-10 | Tasks 任务 | `/api/v1/tasks` | ⬜ |
| CARD-11 | Projects 项目 | `/api/v1/projects` | ⬜ |
| CARD-12 | Issues 问题追踪 | `/api/v1/issues` | ⬜ |
| CARD-13 | Skills 技能系统 | `/api/v1/skills` | ⬜ |
| CARD-14 | Skills 导入 | `/skills/import` | ⬜ |
| CARD-15 | AI CLI 管理 | `/api/v1/ai/clis` | ⬜ |
| CARD-16 | AI 模型管理 | `/api/v1/ai/models` | ⬜ |
| CARD-17 | AI Provider V2 | `/api/v1/ai/v2/providers` | ⬜ |
| CARD-18 | AI 直接执行 | `/ai/execute`、`/ai/chat` | ⬜ |
| CARD-19 | API Keys | `/api/v1/ai/api-keys` | ⬜ |
| CARD-20 | 多模型路由 | `/ai/cli-model` | ⬜ |
| CARD-21 | Costs 费用统计 | `/api/v1/costs/*` | ⬜ |
| CARD-22 | Workspaces 工作区 | `/api/v1/workspaces` | ⬜ |
| CARD-23 | Knowledge 知识库 | `/api/v1/knowledge` | ⬜ |
| CARD-24 | Triggers 触发器 | `/api/v1/triggers` | ⬜ |
| CARD-25 | Scheduler 调度器 | `/api/v1/scheduler` | ⬜ |
| CARD-26 | Templates 模板 | `/api/v1/templates` | ⬜ |
| CARD-27 | Search 全局搜索 | `/api/v1/search` | ⬜ |
| CARD-28 | Wisdom 知识沉淀 | `/api/v1/wisdom` | ⬜ |
| CARD-29 | Test Gen 测试生成 | `/api/v1/test-gen` | ⬜ |
| CARD-30 | Plugins 插件 | `/api/v1/plugins` | ⬜ |
| CARD-31 | Processes 进程 | `/api/v1/processes` | ⬜ |
| CARD-32 | Execution Logs | execution_logs | ⬜ |
| CARD-33 | WS 执行推送 | `WS /ws/executions/:id` | ⬜ |
| CARD-34 | WS Session 流 | `WS /ws/sessions/:id` | ⬜ |
| CARD-35 | WS 终端/命令 | `WS /ws/terminal` | ⬜ |
| CARD-36 | Tauri 桌面应用 | 手动验证 | ⬜ |

**通过标准**：36/36 全绿 = 全功能端到端跑通

---

## 已知风险

| 风险 | 影响卡片 | 说明 |
|------|----------|------|
| Checkpoint 未写入 | CARD-03 | 已知问题 #5，断点续跑可能失效 |
| `.expect()` 崩溃 | 全部 | 已知问题 #1，特定操作可能 panic |
| CLI 工具未安装 | CARD-18/29 | 需本机有 `claude` 或 `gemini` |
| websocat 未安装 | CARD-33~35 | `brew install websocat` |
