# NexusFlow 验收测试 — 目标：开发电商后台 + 商城前台

> 读这一个文件就够了。三个场景，同一个目标项目，不同使用方式。
> 环境：`cargo run --bin nx_api`（后端 8080）+ `cd nx_dashboard && npm run dev`（前端 5173）

**目标项目**：ShopFlow — 电商系统
- 后台管理：商品管理、订单管理、用户管理、数据看板
- 商城前台：商品列表、购物车、下单、支付流程
- 技术栈：Python FastAPI + SQLite + HTML/JS 前端

---

## 场景一：从零开始，手动驱动

> 模拟一个开发者第一次用 NexusFlow，不依赖任何预设，自己一步步搭起来。

### S1-1 建立工作区

```bash
mkdir -p /tmp/shopflow && git -C /tmp/shopflow init

WS=$(curl -s -X POST http://localhost:3000/api/v1/workspaces \
  -H "Content-Type: application/json" \
  -d '{"name":"shopflow","root_path":"/tmp/shopflow"}')
WS_ID=$(echo $WS | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")
echo "WS_ID=$WS_ID"
```

### S1-2 上传需求到知识库

```bash
cat > /tmp/shopflow-req.md << 'EOF'
# ShopFlow 需求

## 后台管理 API（FastAPI）
- GET/POST/PUT/DELETE /api/products  商品 CRUD
- GET/POST/PUT /api/orders           订单管理
- GET/POST /api/users                用户管理
- GET /api/dashboard                 统计数据（总销售额、订单数、用户数）

## 商城前台（HTML+JS）
- index.html：商品列表，支持搜索
- cart.html：购物车，增删改数量
- checkout.html：下单表单

## 技术要求
- Python 3.8+，FastAPI，SQLite，无需认证
- 单文件 main.py + static/ 目录
- 启动：uvicorn main:app --port 8000
EOF

curl -s -X POST http://localhost:3000/api/v1/knowledge \
  -F "file=@/tmp/shopflow-req.md"
```

### S1-3 手动创建任务拆解

```bash
for TITLE in \
  "设计数据库 schema（products/orders/users 三张表）" \
  "实现 FastAPI 后端 main.py（CRUD + dashboard）" \
  "实现商城前台 index.html（商品列表+搜索）" \
  "实现购物车 cart.html + checkout.html" \
  "编写 pytest 测试 test_api.py" \
  "代码审查：SQL注入、错误处理、边界条件"; do
  curl -s -X POST http://localhost:3000/api/v1/tasks \
    -H "Content-Type: application/json" \
    -d "{\"title\":\"$TITLE\",\"status\":\"pending\"}" | \
    python3 -c "import sys,json;d=json.load(sys.stdin)['data'];print(d['id'],d['title'])"
done
```

### S1-4 手动创建 Workflow（逐步构建）

```bash
WF=$(curl -s -X POST http://localhost:3000/api/v1/workflows \
  -H "Content-Type: application/json" \
  -d '{
    "name":"shopflow-from-scratch",
    "description":"从零构建电商系统",
    "stages":[
      {"name":"db-design","prompt":"设计 SQLite schema，包含 products(id,name,price,stock,description)、orders(id,user_id,total,status,created_at)、order_items(id,order_id,product_id,qty,price)、users(id,name,email,created_at) 四张表，输出完整建表 SQL"},
      {"name":"backend","prompt":"基于上一步的 schema，用 FastAPI 实现 main.py。包含：商品CRUD、订单CRUD、用户列表、dashboard统计接口。使用 SQLite，启动时自动建表。代码要完整可运行。"},
      {"name":"frontend-list","prompt":"实现 static/index.html，纯 HTML+JS，调用 /api/products 展示商品列表，支持按名称搜索，每个商品有「加入购物车」按钮（存 localStorage）"},
      {"name":"frontend-cart","prompt":"实现 static/cart.html，读取 localStorage 购物车，显示商品列表、数量调整、删除、总价计算，「去结算」跳转 checkout.html"},
      {"name":"frontend-checkout","prompt":"实现 static/checkout.html，填写姓名邮箱，提交后调用 POST /api/orders 创建订单，成功后清空购物车并显示订单号"},
      {"name":"test","prompt":"为 main.py 编写 test_api.py，用 pytest + httpx 测试：创建商品、查询商品列表、创建订单、查询 dashboard。"},
      {"name":"review","prompt":"审查 main.py 的安全性：检查 SQL 注入风险（是否用参数化查询）、错误处理是否完整、边界条件（库存为0时能否下单）"}
    ]
  }')
WF_ID=$(echo $WF | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")
echo "WF_ID=$WF_ID"
```

### S1-5 启动执行，监控进度

```bash
EX=$(curl -s -X POST http://localhost:3000/api/v1/executions \
  -H "Content-Type: application/json" \
  -d "{\"workflow_id\":\"$WF_ID\",\"input\":{\"working_dir\":\"/tmp/shopflow\"}}")
EX_ID=$(echo $EX | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")
echo "EX_ID=$EX_ID"

# 监控（每15秒）
while true; do
  STATUS=$(curl -s http://localhost:3000/api/v1/executions/$EX_ID | \
    python3 -c "import sys,json;d=json.load(sys.stdin)['data'];print(d['status'],'|',len(d.get('stage_results',[])),'stages done')")
  echo "$(date +%H:%M:%S) $STATUS"
  echo $STATUS | grep -q "completed\|failed" && break
  sleep 15
done
```

### S1-6 提取产物，写入文件

```bash
curl -s http://localhost:3000/api/v1/executions/$EX_ID | python3 << 'EOF'
import sys, json, re, os

d = json.load(sys.stdin)['data']
os.makedirs('/tmp/shopflow/static', exist_ok=True)

file_map = {
    'db-design': None,
    'backend': '/tmp/shopflow/main.py',
    'frontend-list': '/tmp/shopflow/static/index.html',
    'frontend-cart': '/tmp/shopflow/static/cart.html',
    'frontend-checkout': '/tmp/shopflow/static/checkout.html',
    'test': '/tmp/shopflow/test_api.py',
}

for stage in d.get('stage_results', []):
    name = stage['stage_name']
    path = file_map.get(name)
    if not path:
        continue
    output = str(stage.get('output', ''))
    # 提取代码块
    for lang in ['python', 'html', '']:
        m = re.search(rf'```{lang}\n(.*?)```', output, re.DOTALL)
        if m:
            with open(path, 'w') as f:
                f.write(m.group(1))
            print(f'✅ Written {path}')
            break
    else:
        print(f'⚠️  No code block for {name}, check manually')
EOF
```

### S1-7 验收运行

```bash
cd /tmp/shopflow
pip install fastapi uvicorn httpx pytest -q

# 启动后端
uvicorn main:app --port 8000 &
sleep 2

# 验收接口
curl -s -X POST http://localhost:8000/api/products \
  -H "Content-Type: application/json" \
  -d '{"name":"iPhone 15","price":5999,"stock":100,"description":"苹果手机"}'

curl -s http://localhost:8000/api/products
curl -s http://localhost:8000/api/dashboard

# 运行测试
python3 -m pytest test_api.py -v

# 验收前台（检查文件存在）
ls -la static/
```

### S1-8 Git 提交 + 知识沉淀

```bash
# 查看 diff
curl -s http://localhost:3000/api/v1/workspaces/$WS_ID/diffs

# 沉淀经验
curl -s -X POST http://localhost:3000/api/v1/wisdom \
  -H "Content-Type: application/json" \
  -d '{
    "title":"FastAPI 电商系统标准结构",
    "content":"单文件 main.py + static/ 目录是 FastAPI 小型电商的最简结构。SQLite 参数化查询防注入，启动时 CREATE TABLE IF NOT EXISTS 自动建表。",
    "category":"architecture"
  }'
```

**场景一验收标准**：
- [ ] `curl http://localhost:8000/api/products` 返回商品列表
- [ ] `curl http://localhost:8000/api/dashboard` 返回统计数据
- [ ] `static/index.html` 文件存在
- [ ] `pytest test_api.py` 通过率 ≥ 60%

---

---

## 场景二：复用现有工作流（模板驱动开发）

> **目标**：用场景一创建的 `shopflow-from-scratch` workflow 作为模板，快速启动第二个电商项目（多商户版），验证模板复用、Skills 预设、Workflow clone 功能。

### S2-1 查找并克隆现有 Workflow

```bash
# 列出所有 workflow，找到场景一创建的
WF_LIST=$(curl -s http://localhost:3000/api/v1/workflows)
echo $WF_LIST | python3 -c "
import sys, json
wfs = json.load(sys.stdin)
for w in wfs:
    print(w['id'], w['name'])
"

# 找到 shopflow-from-scratch 的 ID
ORIG_WF_ID="<从上面输出中复制>"

# 克隆为新 workflow（多商户版）
WF2_ID=$(curl -s -X POST http://localhost:3000/api/v1/workflows \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"shopflow-multitenant\",
    \"description\": \"多商户电商系统，基于 shopflow-from-scratch 模板扩展\",
    \"stages\": [
      {\"name\":\"db-design\",\"prompt\":\"设计多商户 SQLite schema：merchants/products/orders/users 表，products 和 orders 含 merchant_id 外键\"},
      {\"name\":\"backend\",\"prompt\":\"实现 FastAPI 多商户后台：/api/merchants CRUD，/api/products?merchant_id= 过滤，JWT 认证区分商户\"},
      {\"name\":\"frontend-admin\",\"prompt\":\"实现商户管理后台 HTML：商户列表、商品管理、订单查看，fetch API 调用后端\"},
      {\"name\":\"frontend-store\",\"prompt\":\"实现买家商城 HTML：按商户浏览商品、加购物车、结算，localStorage 存购物车\"},
      {\"name\":\"test\",\"prompt\":\"生成 pytest 测试：多商户隔离测试（商户A不能看商户B的数据）、JWT 认证测试\"},
      {\"name\":\"review\",\"prompt\":\"安全审查：多租户数据隔离、SQL 注入、JWT 过期处理、CORS 配置\"}
    ]
  }" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])")

echo "WF2_ID=$WF2_ID"
```

### S2-2 查看并激活预设 Skills

```bash
# 列出所有内置 skills
curl -s http://localhost:3000/api/v1/skills | python3 -c "
import sys, json
skills = json.load(sys.stdin)
for s in skills:
    print(s['id'], s['name'], '| enabled:', s.get('enabled', False))
"

# 启用 Python 代码生成 skill（如果存在）
SKILL_ID=$(curl -s http://localhost:3000/api/v1/skills | python3 -c "
import sys, json
skills = json.load(sys.stdin)
for s in skills:
    if 'python' in s['name'].lower() or 'code' in s['name'].lower():
        print(s['id'])
        break
")

if [ -n "$SKILL_ID" ]; then
  curl -s -X PATCH http://localhost:3000/api/v1/skills/$SKILL_ID \
    -H "Content-Type: application/json" \
    -d '{"enabled": true}'
  echo "Skill $SKILL_ID enabled"
fi
```

### S2-3 创建新工作区并启动执行

```bash
# 新工作区
mkdir -p /tmp/shopflow-v2 && git -C /tmp/shopflow-v2 init

WS2_ID=$(curl -s -X POST http://localhost:3000/api/v1/workspaces \
  -H "Content-Type: application/json" \
  -d '{"name":"shopflow-v2","root_path":"/tmp/shopflow-v2"}' \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])")

# 复用场景一上传的知识库文档（直接引用 KB_ID）
# 如果 KB_ID 已失效，重新上传
curl -s http://localhost:3000/api/v1/knowledge | python3 -c "
import sys, json
docs = json.load(sys.stdin)
for d in docs:
    print(d['id'], d.get('filename',''), d.get('chunk_count',0), 'chunks')
"

# 启动执行
EX2_ID=$(curl -s -X POST http://localhost:3000/api/v1/executions \
  -H "Content-Type: application/json" \
  -d "{\"workflow_id\":\"$WF2_ID\",\"input\":{\"working_dir\":\"/tmp/shopflow-v2\"}}" \
  | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('id') or d.get('data',{}).get('id',''))")

echo "EX2_ID=$EX2_ID"
```

### S2-4 监控 + 断点续跑测试

```bash
# 监控执行（每 15 秒）
for i in $(seq 1 20); do
  STATUS=$(curl -s http://localhost:3000/api/v1/executions/$EX2_ID | \
    python3 -c "import sys,json; d=json.load(sys.stdin); \
    data=d.get('data',d); print(data.get('status','?'), len(data.get('stage_results',[])), 'stages done')")
  echo "[$(date +%H:%M:%S)] $STATUS"
  echo "$STATUS" | grep -q "completed\|failed" && break
  sleep 15
done

# 测试断点续跑：如果执行中途失败，检查 checkpoints
curl -s http://localhost:3000/api/v1/executions/$EX2_ID | \
  python3 -c "
import sys, json
d = json.load(sys.stdin)
data = d.get('data', d)
results = data.get('stage_results', [])
print('Completed stages:')
for r in results:
    print(' -', r['stage_name'], ':', r.get('status','?'))
failed = [r for r in results if r.get('status') == 'failed']
if failed:
    print('Failed stages:', [r['stage_name'] for r in failed])
    print('Can resume from checkpoint if supported')
"
```

### S2-5 提取代码 + 验证多商户隔离

```bash
# 提取所有 stage 的代码
curl -s http://localhost:3000/api/v1/executions/$EX2_ID | python3 -c "
import sys, json, re, os
d = json.load(sys.stdin)
data = d.get('data', d)
os.makedirs('/tmp/shopflow-v2', exist_ok=True)
for s in data.get('stage_results', []):
    output = str(s.get('output', ''))
    name = s['stage_name']
    match = re.search(r'\`\`\`python\n(.*?)\`\`\`', output, re.DOTALL)
    if match and name == 'backend':
        with open('/tmp/shopflow-v2/main.py', 'w') as f:
            f.write(match.group(1))
        print('Written main.py')
    match_html = re.search(r'\`\`\`html\n(.*?)\`\`\`', output, re.DOTALL)
    if match_html and 'frontend' in name:
        fname = 'admin.html' if 'admin' in name else 'store.html'
        os.makedirs('/tmp/shopflow-v2/static', exist_ok=True)
        with open(f'/tmp/shopflow-v2/static/{fname}', 'w') as f:
            f.write(match_html.group(1))
        print(f'Written static/{fname}')
    match_test = re.search(r'\`\`\`python\n(.*?)\`\`\`', output, re.DOTALL)
    if match_test and name == 'test':
        with open('/tmp/shopflow-v2/test_api.py', 'w') as f:
            f.write(match_test.group(1))
        print('Written test_api.py')
"

# 启动服务
cd /tmp/shopflow-v2
pip install fastapi uvicorn python-jose -q
uvicorn main:app --port 8001 --reload &
sleep 3

# 验证多商户隔离
curl -s -X POST http://localhost:8001/api/merchants \
  -H "Content-Type: application/json" \
  -d '{"name":"商户A","email":"a@test.com"}' | python3 -m json.tool

curl -s -X POST http://localhost:8001/api/merchants \
  -H "Content-Type: application/json" \
  -d '{"name":"商户B","email":"b@test.com"}' | python3 -m json.tool

# 验证商户A的商品不出现在商户B的列表
MERCHANT_A_ID=1
curl -s "http://localhost:8001/api/products?merchant_id=$MERCHANT_A_ID"
```

### S2-6 搜索功能验证

```bash
# 测试全局搜索（跨 workflow/execution/knowledge）
curl -s "http://localhost:3000/api/v1/search?q=shopflow" | python3 -c "
import sys, json
results = json.load(sys.stdin)
print('Search results:', len(results) if isinstance(results, list) else results)
"

# 查看费用统计（两次执行的 token 消耗）
curl -s http://localhost:3000/api/v1/costs | python3 -c "
import sys, json
costs = json.load(sys.stdin)
data = costs.get('data', costs)
print('Total cost:', data.get('total_cost', 'N/A'))
print('Total tokens:', data.get('total_tokens', 'N/A'))
"
```

**场景二验收标准**：
- [ ] Workflow 克隆成功，WF2_ID 非空
- [ ] Skills 列表可查，至少 1 个 skill 可启用
- [ ] 多商户 API 返回正确隔离数据
- [ ] 搜索 `shopflow` 能找到相关记录
- [ ] 费用统计显示两次执行的 token 消耗

---




## 场景三：AI 团队协作开发（桌面应用 UI 操作）

> **目标**：通过桌面应用 UI 完成 ShopFlow 开发，覆盖「团队 CLI」模块的完整流程：创建团队 → 配置角色 → 发送任务 → 查看终端输出 → 收集产物。

> **关键区别**：
> - `/teams-v2`（团队 CLI）= CLI 优先路径，流式文本输出，适合代码生成
> - `/teams`（团队）= PTY dispatch，每个角色独立 claude 终端会话，适合长任务
> - `/group-chat`（群组讨论）= 多角色同时参与一个对话，适合讨论和决策

---

### S3-1 启动桌面应用 + 选择工作区

**操作步骤（UI）**：

1. 启动后端：`cargo run --bin nx_api`（端口 8080）
2. 启动前端：`cd nx_dashboard && npm run dev`（端口 5173）
3. 打开浏览器访问 `http://localhost:5173`
4. 左上角工作区选择器 → 点击「+ 新建工作区」
5. 填写：名称 `shopflow-v3`，路径 `/tmp/shopflow-v3`
6. 点击确认，工作区切换到 shopflow-v3

**验证**：顶部工作区显示 `shopflow-v3`

---

### S3-2 创建团队（/teams-v2 页面）

**操作步骤（UI）**：

1. 左侧导航 → **AI 团队** → **团队 CLI**（`/teams-v2`）
2. 点击右上角「+ 新建团队」
3. 填写：名称 `shopflow-team`，描述 `ShopFlow 电商系统开发团队`
4. 点击创建

**验证**：团队卡片出现在列表中

---

### S3-3 创建角色（在团队内）

**操作步骤（UI）**：

1. 点击 `shopflow-team` 卡片进入团队详情
2. 点击「+ 添加角色」→「新建角色」
3. 依次创建 4 个角色：

**角色 1 — 架构师**
- 名称：`architect`
- System Prompt：`你是资深系统架构师。收到任务后，输出 SQLite 建表 SQL 和 RESTful API 端点列表。`
- 触发关键词：`数据库, schema, 设计, 架构`

**角色 2 — 后端开发**
- 名称：`backend-dev`
- System Prompt：`你是 Python 后端工程师，专注 FastAPI + SQLite。输出完整可运行代码，用 \`\`\`python 包裹。`
- 触发关键词：`后端, API, FastAPI, 接口, python`

**角色 3 — 前端开发**
- 名称：`frontend-dev`
- System Prompt：`你是前端工程师，专注原生 HTML/CSS/JS。输出完整 HTML 文件，用 \`\`\`html 包裹。`
- 触发关键词：`前端, HTML, 商城, 页面, UI`

**角色 4 — QA**
- 名称：`qa-engineer`
- System Prompt：`你是 QA 工程师，专注 pytest 测试和安全审查。输出完整测试文件，用 \`\`\`python 包裹。`
- 触发关键词：`测试, pytest, 安全, 审查`

**验证**：团队详情面板显示 4 个角色，每个角色有触发关键词

---

### S3-4 发送任务（对话框）

**操作步骤（UI）**：

1. 在团队详情页，点击「对话 (CLI)」按钮（或直接点击团队卡片上的对话图标）
2. 进入 ConversationView，底部输入框发送以下任务：

**任务 1（只触发架构师）**：
```
请设计 ShopFlow 电商系统的数据库 schema：users/products/orders/order_items 四张表，输出建表 SQL
```
- 观察：只有 `architect` 角色响应（parallel_count=1）
- 等待流式输出完成

**任务 2（同时触发后端+前端）**：
```
请同时开始：后端实现 FastAPI /api/products 和 /api/orders 接口；前端实现商城商品列表页面 HTML
```
- 观察：`backend-dev` 和 `frontend-dev` 并行响应（parallel_count=2）
- 右上角切换到「终端」Tab，可看到两个并行 PTY 会话

**任务 3（触发 QA）**：
```
请对 ShopFlow 进行安全审查和测试：检查 SQL 注入风险，生成 pytest 测试覆盖 /api/products
```
- 观察：`qa-engineer` 响应

**验证**：
- 对话历史显示每条消息的角色来源
- 终端 Tab 显示对应的 claude 会话输出

---

### S3-5 查看终端输出（PTY 会话）

**操作步骤（UI）**：

1. 在团队对话页，点击右上角「终端」Tab
2. 可看到每个角色的独立终端面板（TerminalGrid）
3. 观察 claude CLI 的实时流式输出
4. 如果出现「Bypass Permissions」对话框，系统会自动处理（↓ + Enter 选 Yes）
5. 如果出现「trust this folder」对话框，系统自动发 Enter 确认

**验证**：终端面板显示 claude 的实时输出，无卡死

---

### S3-6 群组讨论（/group-chat 页面）

**操作步骤（UI）**：

1. 左侧导航 → **AI 团队** → **群组讨论**（`/group-chat`）
2. 创建群组：名称 `shopflow-review`，选择成员（architect + backend-dev + qa-engineer）
3. 在群组对话框发送：
```
请三位角色共同讨论：ShopFlow 的 JWT 认证方案，架构师给出方案，后端给出实现思路，QA 给出测试要点
```
4. 观察三个角色依次回复

**验证**：群组消息列表显示不同角色的回复，角色名称标注清晰

---

### S3-7 Pipeline 视图（团队详情）

**操作步骤（UI）**：

1. 回到 `/teams-v2` → 点击 `shopflow-team`
2. 在团队详情面板，切换到「Pipeline」Tab
3. 如果已有项目工作区，可看到 Pipeline 执行状态
4. 点击「+ 新建 Pipeline」，配置 stages：
   - Stage 1：`architect` 角色 → 设计 schema
   - Stage 2：`backend-dev` 角色 → 实现后端
   - Stage 3：`qa-engineer` 角色 → 测试

**验证**：Pipeline 视图显示 stage 列表和执行状态

---

### S3-8 进程监测 + 知识沉淀

**操作步骤（UI）**：

1. 左侧导航 → **AI 团队** → **进程监测**（`/processes`）
2. 查看所有活跃的 claude 进程，确认无僵尸进程
3. 左侧导航 → **资源** → **知识库**（`/wisdom`）
4. 点击「+ 新增」，填写：
   - 标题：`AI 团队 CLI 开发模式`
   - 内容：`团队 CLI 路径适合代码生成（流式输出）；PTY 路径适合长任务（独立终端）；群组讨论适合多角色决策。trigger_keywords 要互斥，每角色 3-5 个精准关键词。`
   - 分类：`team-workflow`

**验证**：Wisdom 条目创建成功，可在搜索页找到

---

**场景三验收标准**：
- [ ] 团队 CLI 页面（`/teams-v2`）创建团队成功
- [ ] 4 个角色创建，trigger_keywords 各不相同
- [ ] 单关键词任务只触发 1 个角色
- [ ] 多关键词任务并行触发 2+ 个角色，终端 Tab 显示多个会话
- [ ] 群组讨论（`/group-chat`）多角色回复正常
- [ ] 进程监测（`/processes`）无僵尸进程
- [ ] Wisdom 条目创建成功

---

## 边界条件汇总

| 场景 | 边界条件 | 预期行为 |
|------|---------|---------|
| 所有场景 | `claude` CLI 未安装 | 顶部显示 ClaudeCliMissingBanner，执行失败但不崩溃 |
| S1/S2 | AI 输出无代码块 | 手动从 stage output 提取，记录为"格式问题" |
| S2 | 克隆 workflow 后立即执行 | 与原 workflow 独立，互不影响 |
| S3 | 任务不含任何 trigger_keywords | 路由到团队第一个角色（兜底逻辑） |
| S3 | 角色关键词重叠 | 多个角色同时响应，parallel_count > 预期，属正常行为 |
| S3 | PTY 启动出现 Bypass Permissions 对话框 | 系统自动处理（↓+Enter），无需手动干预 |
| S3 | 群组讨论成员为空 | 创建失败或返回空消息列表，不崩溃 |
| 所有场景 | SQLite 文件已存在 | `CREATE TABLE IF NOT EXISTS` 幂等，不报错 |
| 所有场景 | 端口 8000/8001 被占用 | 换端口重试，与 NexusFlow（3000/5173）不冲突 |
| S2 | checkpoints 表为空 | 断点续跑返回 404 或空列表，不崩溃 |

---

## 场景四：trigger_keywords 路由与并行分发专项测试

> 验证团队核心机制：关键词自动路由 + 多角色并行执行

### S4-1 创建多角色团队（含 trigger_keywords）

```bash
TM_ID=$(curl -s -X POST http://localhost:3000/api/v1/teams \
  -H "Content-Type: application/json" \
  -d '{"name":"shopflow-team","description":"电商开发团队"}' | \
  python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

# 角色1：前端开发，关键词匹配前端任务
curl -s -X POST http://localhost:3000/api/v1/teams/$TM_ID/roles \
  -H "Content-Type: application/json" \
  -d '{"name":"frontend","trigger_keywords":["前端","HTML","CSS","页面","UI"],"model_config":{"model":"claude-haiku-4-5"}}'

# 角色2：后端开发，关键词匹配后端任务
curl -s -X POST http://localhost:3000/api/v1/teams/$TM_ID/roles \
  -H "Content-Type: application/json" \
  -d '{"name":"backend","trigger_keywords":["后端","API","数据库","FastAPI","SQLite"],"model_config":{"model":"claude-haiku-4-5"}}'

# 角色3：测试工程师，关键词匹配测试任务
curl -s -X POST http://localhost:3000/api/v1/teams/$TM_ID/roles \
  -H "Content-Type: application/json" \
  -d '{"name":"tester","trigger_keywords":["测试","pytest","单元测试","test"],"model_config":{"model":"claude-haiku-4-5"}}'
```

### S4-2 单关键词路由（只触发一个角色）

```bash
curl -s -X POST http://localhost:3000/api/v1/teams/$TM_ID/execute \
  -H "Content-Type: application/json" \
  -d '{"task":"实现商品列表的 HTML 页面，包含搜索框和商品卡片"}'
```

**预期**：只有 `frontend` 角色响应（任务含「HTML」「页面」），`parallel_count=1`

### S4-3 多关键词并行分发（触发多个角色）

```bash
curl -s -X POST http://localhost:3000/api/v1/teams/$TM_ID/execute \
  -H "Content-Type: application/json" \
  -d '{"task":"实现商品 API 接口并编写 pytest 单元测试"}'
```

**预期**：`backend`（含「API」）和 `tester`（含「pytest」）同时响应，`parallel_count=2`，两个 PTY 会话并行启动

### S4-4 无关键词兜底路由

```bash
curl -s -X POST http://localhost:3000/api/v1/teams/$TM_ID/execute \
  -H "Content-Type: application/json" \
  -d '{"task":"帮我想一个项目名称"}'
```

**预期**：无关键词匹配，路由到团队第一个角色（`frontend`），`parallel_count=1`

### S4-5 团队 CLI（V2）流式输出验证

1. 桌面应用 → 侧边栏 → **团队 CLI**（`/teams-v2`）
2. 选择 `shopflow-team`
3. 输入：`实现购物车的后端 API，用 FastAPI + SQLite`
4. 观察流式文本输出（CLI 优先路径，干净文本，无 PTY 噪音）

**预期**：`backend` 角色响应，输出实时流式显示，无乱码

### S4-6 trigger_keywords 路由汇总

| 任务内容 | 匹配角色 | parallel_count | 验证点 |
|---------|---------|---------------|--------|
| 含「HTML」「页面」 | frontend | 1 | 单角色路由 |
| 含「API」「pytest」 | backend + tester | 2 | 并行分发 |
| 含「前端」「测试」 | frontend + tester | 2 | 并行分发 |
| 无任何关键词 | 第一个角色 | 1 | 兜底逻辑 |

# NexusFlow 桌面应用使用手册

> 覆盖所有模块的真实操作步骤，基于实际代码实现。
> 启动命令：`cargo run --bin nx_api`（后端 8080）+ `cd nx_dashboard && npm run dev`（前端 5173）

---

## 一、仪表盘（Dashboard）

**功能**：系统总览 + 快速运行

### 操作步骤
1. 打开 `http://localhost:5173`，默认进入仪表盘
2. 查看活跃执行面板（ActiveExecutionsPanel）：显示正在运行的 workflow 执行
3. 快速运行：点击「Quick Run」输入框，输入任务描述，点击发送
   - 调用 `POST /api/v1/quick-run`
   - 立即在仪表盘看到执行卡片

### 测试场景
- 输入 `"写一个 hello world Python 脚本"` → 观察执行卡片出现
- 输入空字符串 → 按钮应禁用或提示错误

---

## 二、工作流（Workflows）

**功能**：创建/编辑/运行 AI 生产线，每个 workflow 由多个 stage 组成

### 操作步骤
1. 左侧导航 → **工作流**
2. 点击「+ 新建工作流」
3. 填写名称、描述
4. 添加 Stage：点击「+ 添加阶段」，每个 stage 填写：
   - 名称（如 `design`、`implement`、`test`）
   - Prompt（AI 执行的指令）
5. 点击保存 → 获得 `workflow_id`
6. 点击「▶ 运行」→ 弹出启动对话框，填写 `working_dir`
7. 跳转到执行记录页查看进度

### 技能关联（Skill Gallery）
- 在 workflow 编辑页，点击「技能库」图标
- 选择预设技能（如 Python 代码生成、代码审查）
- 技能会注入到对应 stage 的 prompt 中

### 测试场景
- 创建 3-stage workflow：`design → implement → test`
- 运行后观察每个 stage 的输出
- 删除 workflow → 确认弹窗出现 → 确认删除

---

## 三、执行记录（Executions）

**功能**：查看所有 workflow 执行的详情、日志、产物、Git diff

### 操作步骤
1. 左侧导航 → **执行记录**
2. 列表显示所有执行，状态：`pending / running / completed / failed / paused`
3. 点击某条执行 → 右侧详情面板展开，包含 4 个 Tab：
   - **Stages**：每个 stage 的输出（可展开/折叠）
   - **Logs**：实时日志流（WebSocket 推送）
   - **Artifacts**：AI 生成的文件产物
   - **Git**：工作区的 git diff
4. 运行中的执行：点击「取消」按钮停止
5. 暂停的执行：点击「继续」恢复

### 测试场景
- 运行一个 workflow → 在执行记录页实时观察 stage 进度
- 点击 Logs Tab → 确认日志实时滚动
- 点击 Git Tab → 确认显示 diff（需要工作区有文件变更）
- 运行中点击取消 → 状态变为 `cancelled`

---

## 四、可视化画布（Canvas）

**功能**：拖拽式 workflow 编辑器，节点连线可视化

### 操作步骤
1. 左侧导航 → **可视化画布**
2. 从左侧节点面板拖拽节点到画布
3. 连接节点（拖拽连线）
4. 点击节点 → 右侧属性面板编辑 prompt
5. 点击「YAML」Tab → 查看/编辑对应的 workflow YAML
6. 点击「保存」→ 同步到 workflow 列表
7. 点击「模板」Tab → 从模板库加载预设 workflow

### 测试场景
- 拖入 3 个节点，连线成链 → 保存 → 在工作流页确认出现
- 切换 YAML Tab → 手动修改 YAML → 画布节点同步更新

---

## 五、Sprint 看板（Sprint Board）

**功能**：项目迭代管理，看板式任务追踪

### 操作步骤
1. 左侧导航 → **Sprint 看板**
2. 查看 Sprint 列表（调用 `GET /api/v1/sprints`）
3. 拖拽卡片在列间移动（`pending → in_progress → done`）
4. 点击卡片 → 更新状态（`POST /api/v1/sprints/:id/status`）

### 测试场景
- 创建新 Sprint → 添加任务卡片
- 拖拽卡片到「进行中」→ 刷新页面确认状态持久化

---

## 六、AI 团队 → 团队（Teams）

**功能**：PTY dispatch 模式，每个角色独立 claude 终端会话

### 操作步骤
1. 左侧导航 → **AI 团队** → **团队**
2. 点击「+ 新建团队」，填写名称和描述
3. 进入团队详情，切换到 3 个 Tab：
   - **角色**：管理团队成员角色
   - **Pipeline**：查看/创建 Pipeline 执行
   - **设置**：团队配置
4. 点击「+ 添加角色」→「新建角色」，填写：
   - 名称、描述
   - System Prompt（角色的 AI 指令）
   - 触发关键词（逗号分隔，用于自动路由）
   - 模型配置（provider/model/temperature）
5. 点击团队卡片上的「对话」图标 → 进入 ConversationView
6. 输入任务 → 系统根据触发关键词路由到匹配角色
7. 切换「终端」Tab → 查看 PTY 会话的实时输出

### 触发关键词路由规则
- 任务文本包含角色的关键词 → 该角色响应
- 多个角色同时匹配 → 并行 PTY dispatch
- 无匹配 → 路由到团队第一个角色（兜底）

### 测试场景
- 创建含 `trigger_keywords: ["后端", "API"]` 的角色
- 发送含「后端」的任务 → 只有该角色响应
- 发送含「后端」+「前端」的任务 → 两个角色并行响应
- 终端 Tab 显示多个 PTY 会话

---

## 七、AI 团队 → 角色（Roles）

**功能**：独立管理所有角色（跨团队），可分配技能

### 操作步骤
1. 左侧导航 → **AI 团队** → **角色**
2. 查看所有角色列表（`GET /api/v1/roles`）
3. 点击「+ 新建角色」，填写：
   - 名称、描述、System Prompt
   - 触发关键词
   - 模型配置（model_id、provider、max_tokens、temperature）
4. 点击角色卡片 → 进入角色详情
5. 「技能」Tab → 点击「分配技能」→ 选择技能 + 优先级（critical/high/medium/low）
6. 点击「分配到团队」→ 选择目标团队

### 测试场景
- 创建角色，分配 2 个技能
- 将角色分配到已有团队 → 在团队详情确认角色出现
- 删除角色 → 确认从团队中移除

---

## 八、AI 团队 → 技能（Skills）

**功能**：可复用的 AI 能力单元，可导入/执行/分配给角色

### 操作步骤
1. 左侧导航 → **AI 团队** → **技能**（实际路径 `/skills`）
2. 查看技能列表，包含内置预设技能
3. **启用/禁用**：点击技能卡片上的开关
4. **执行技能**：点击「▶ 执行」→ 填写输入参数 → 查看输出
5. **导入技能**（3 种方式）：
   - URL：填写技能定义文件的 URL
   - 文件：上传本地 `.yaml` 或 `.json` 文件
   - 粘贴：直接粘贴技能定义内容
6. **导出技能**：点击「↓ 导出」→ 下载技能定义文件
7. **分配给角色**：在角色详情页的技能 Tab 操作

### 测试场景
- 找到内置「Python 代码生成」技能 → 启用 → 执行，输入 `"写一个排序函数"`
- 导入一个自定义技能（粘贴模式）→ 确认出现在列表
- 将技能分配给角色，优先级设为 `high`

---

## 九、AI 团队 → 群组讨论（Group Chat）

**功能**：多角色同时参与一个对话，适合讨论和决策

### 操作步骤
1. 左侧导航 → **AI 团队** → **群组讨论**
2. 点击「+ 新建群组会话」，填写：
   - 名称
   - 成员（从角色列表多选）
3. 点击会话 → 进入群组对话界面
4. 底部输入框发送消息
5. 点击「开始讨论轮次」→ 调用 `POST /api/v1/group-sessions/:id/execute-round`
   - 所有成员角色依次生成回复
6. 查看讨论记录（按角色区分）
7. **Pipeline 审批**：如果讨论触发了 Pipeline 暂停，可在此页面点击「批准」或「拒绝」

### WebSocket 实时推送
- 群组讨论通过 WebSocket 接收实时消息
- 连接断开时自动重连

### 测试场景
- 创建含 3 个角色的群组会话
- 发送「请讨论 ShopFlow 的技术选型」→ 观察 3 个角色依次回复
- 点击「执行轮次」→ 所有角色自动生成一轮回复

---

## 十、AI 团队 → 会话（Sessions）

**功能**：管理单个 AI 会话，支持消息历史和人工回复

### 操作步骤
1. 左侧导航 → **AI 团队** → **会话**
2. 查看所有会话列表（`GET /api/v1/sessions`）
3. 点击会话 → 右侧展开消息历史（每 3 秒自动刷新）
4. 如果会话处于「等待确认」状态：
   - 查看 AI 的问题
   - 点击「回复」→ 填写确认内容 → 提交（`POST /api/v1/sessions/:id/messages/:msgId/respond`）
5. 暂停/恢复会话

### 测试场景
- 运行一个需要人工确认的 workflow（stage 含 `auto_confirm: false`）
- 在会话页找到等待确认的消息 → 回复「继续」
- 观察 workflow 恢复执行

---

## 十一、AI 团队 → 进程监测（Processes）

**功能**：查看所有活跃的 claude 进程，防止僵尸进程

### 操作步骤
1. 左侧导航 → **AI 团队** → **进程监测**
2. 查看进程列表（`GET /api/v1/processes`）
3. 每个进程显示：PID、角色、状态、启动时间
4. 点击「终止」→ 停止指定进程
5. 点击「清理临时文件」→ 调用 `POST /api/v1/temp-cleanup`

### 测试场景
- 启动一个团队任务 → 在进程监测页确认 claude 进程出现
- 任务完成后 → 确认进程自动消失
- 手动点击「清理」→ 确认临时文件清除

---

## 十二、AI 团队 → 团队 CLI（Teams V2）

**功能**：CLI 优先路径，流式文本输出，适合代码生成任务

### 与「团队」的区别
| 特性 | 团队（/teams） | 团队 CLI（/teams-v2） |
|------|--------------|---------------------|
| 执行路径 | PTY dispatch | CLI 直接调用 |
| 输出方式 | 终端 UI | 流式文本 |
| 适合场景 | 长任务、交互式 | 代码生成、快速响应 |

### 操作步骤
1. 左侧导航 → **AI 团队** → **团队 CLI**
2. 创建团队（同「团队」模块）
3. 添加角色（含 trigger_keywords）
4. 点击团队卡片上的「对话 (CLI)」按钮
5. 在 ConversationViewV2 中发送任务
6. 观察流式文本输出（无终端 UI，直接显示文本）
7. 切换「终端」Tab → 仍可查看底层 PTY 会话

### 测试场景
- 发送「写一个 FastAPI hello world」→ 观察流式代码输出
- 发送含多个关键词的任务 → 确认 parallel_count ≥ 2

---

## 十三、资源 → 项目（Projects）

**功能**：项目管理，关联团队和工作区，追踪模块状态

### 操作步骤
1. 左侧导航 → **资源** → **项目**
2. 点击「+ 新建项目」，填写：
   - 名称、描述
   - 关联团队（从团队列表选择）
   - 关联工作区（从工作区列表选择）
3. 点击项目 → 展开模块列表
4. 点击「+ 添加模块」→ 填写模块名称和状态
5. 更新模块状态（`pending / in_progress / completed / blocked`）
6. 模块状态会注入到 AI prompt 中（项目状态感知）

### 测试场景
- 创建项目，关联 shopflow-team
- 添加模块：`backend(in_progress)`, `frontend(pending)`, `tests(pending)`
- 在团队对话中发送任务 → 观察 AI 是否感知到模块状态

---

## 十四、资源 → 模板（Templates）

**功能**：预设 workflow 模板库，快速启动标准项目

### 操作步骤
1. 左侧导航 → **资源** → **模板**
2. 浏览模板列表（按分类筛选）
3. 点击模板卡片 → 查看详情（stages、描述）
4. 点击「使用此模板」→ 基于模板创建新 workflow
5. 修改 workflow 名称和参数 → 保存

### 测试场景
- 找到「Python 项目」模板 → 使用 → 在工作流页确认出现
- 修改模板的某个 stage prompt → 保存 → 运行

---

## 十五、资源 → 知识库（Wisdom）

**功能**：经验沉淀库，存储开发过程中的最佳实践

### 操作步骤
1. 左侧导航 → **资源** → **知识库**
2. 查看条目列表（按分类筛选）
3. 点击「+ 新增」，填写：
   - 标题
   - 内容（Markdown 支持）
   - 分类（如 `architecture`、`engineering`、`team-workflow`）
4. 搜索：顶部搜索框输入关键词 → 实时过滤
5. 删除条目 → 确认弹窗

### 测试场景
- 创建条目：标题「FastAPI 最佳实践」，分类 `engineering`
- 搜索「FastAPI」→ 确认条目出现
- 删除条目 → 确认从列表消失

---

## 十六、资源 → RAG 知识库（Knowledge Base）

**功能**：文档向量化存储，支持语义搜索，为 AI 提供上下文

### 操作步骤
1. 左侧导航 → **资源** → **RAG 知识库**
2. **配置 Embedding**：点击「Embedding 配置」→ 填写 API Key 和模型
3. **创建知识库**：点击「+ 新建知识库」→ 填写名称
4. **上传文档**：
   - 点击「上传文档」→ 选择文件（支持 `.md`、`.txt`、`.pdf`）
   - 文档自动分块（chunking）并向量化
5. **语义搜索**：顶部搜索框输入问题 → 返回相关文档片段
6. 查看文档列表：每个文档显示 chunk 数量

### 测试场景
- 上传 `shopflow-spec.md` → 确认 chunk_count > 0
- 搜索「购物车功能」→ 确认返回相关片段
- 在 workflow stage 中引用知识库 → AI 输出包含文档内容

---

## 十七、工具 → 终端（Terminal）

**功能**：多窗口终端，支持 claude CLI 会话

### 操作步骤
1. 左侧导航 → **工具** → **终端**
2. 点击「+ 新建终端」→ 打开新终端窗口
3. 输入命令执行（标准 shell）
4. 多窗口布局：拖拽调整大小
5. 点击「Claude 流」Tab → 查看 claude CLI 的流式输出

### 测试场景
- 打开终端 → 运行 `python3 --version`
- 打开 Claude 流面板 → 确认 WebSocket 连接（`/ws/claude-stream`）

---

## 十八、工具 → 搜索（Search）

**功能**：全局搜索，跨 workflow/execution/knowledge/wisdom

### 操作步骤
1. 左侧导航 → **工具** → **搜索**
2. 输入关键词 → 实时搜索
3. 结果按类型分组显示（workflow、execution、wisdom 等）
4. 点击结果 → 跳转到对应页面

### 测试场景
- 搜索「shopflow」→ 确认返回相关 workflow 和 execution
- 搜索不存在的词 → 显示空结果，不报错

---

## 十九、工具 → 成本（Cost）

**功能**：Token 消耗统计和费用分析

### 操作步骤
1. 左侧导航 → **工具** → **成本**
2. 查看总 token 消耗和费用
3. 按时间范围筛选
4. 按 workflow/team 分组查看

### 测试场景
- 运行几个 workflow 后 → 在成本页确认 token 数增加
- 查看各 workflow 的费用对比

---

## 二十、系统 → 设置（Settings / AI Settings）

**功能**：配置 AI Provider、模型路由、API Key

### 操作步骤
1. 左侧导航 → **系统** → **设置**
2. **AI 配置**（`/ai-settings`）：
   - 添加 Provider（Anthropic/OpenAI/自定义）
   - 填写 API Key
   - 配置模型路由规则（哪类任务用哪个模型）
3. **CLI 配置**：设置 claude CLI 路径
4. **工作区配置**：默认工作区路径

### 测试场景
- 添加 Anthropic Provider，填写 API Key → 保存
- 配置模型路由：代码生成用 `claude-sonnet-4-6`，快速任务用 `claude-haiku-4-5`
- 运行 workflow → 确认使用了正确的模型

---

## 二十一、工具 → 编辑器（Editor）

**功能**：可视化 Workflow 编辑器，拖拽节点构建 Pipeline，支持模板库

### 操作步骤
1. 左侧导航 → **编辑器**（`/editor`）
2. 从左侧节点面板拖拽节点到画布（Prompt、Code、Condition 等类型）
3. 连接节点，配置每个节点的 prompt/参数
4. 顶部输入 Workflow 名称，点击「保存」→ 调用 `/api/v1/workflows` 创建
5. 点击「模板库」→ 选择预置模板一键加载

### 测试场景
- 拖拽 3 个节点连成链 → 保存 → 在工作流页面确认出现
- 加载模板 → 修改 prompt → 另存为新 workflow
- 从工作流页面点击「编辑」跳转 → 修改后保存

---

## 二十二、工具 → 内置浏览器（Browser）

**功能**：Tauri 内置 WebView 浏览器，支持多标签、截图、查看源码（仅桌面应用有效）

### 操作步骤
1. 左侧导航 → **浏览器**（`/browser`）
2. 地址栏输入 URL → 回车导航
3. 点击「+」新建标签页，多标签切换
4. 点击截图按钮（Camera 图标）→ 截图保存
5. 点击「源码」按钮 → 查看页面 HTML

### 测试场景
- 导航到 `http://localhost:5173` → 确认前端页面加载
- 导航到 `http://localhost:3000/health` → 确认 API 响应显示
- 多标签切换 → 确认各标签独立

---

## 二十三、工具 → UI 设计（UI Design）

**功能**：从截图/URL 提取设计风格，AI 生成前端代码，4步流程：提取→生成→代码化→同步

### 操作步骤
1. 左侧导航 → **UI 设计**（`/ui-design`）
2. **Step 1 提取**：上传截图（文件模式）或输入网页地址（URL 模式）→ 点击「提取风格」
3. **Step 2 生成**：查看提取的设计规范，点击「生成组件」
4. **Step 3 代码化**：查看生成的 HTML/CSS/JS 代码
5. **Step 4 同步**：选择工作区目录 → 写入文件

### 测试场景
- 上传 ShopFlow 商城截图 → 提取设计风格 → 生成商品卡片组件
- URL 模式输入 `http://localhost:5173` → 提取当前系统风格
- 生成代码后点击「同步到工作区」→ 确认文件写入 `/tmp/shopflow/static/`

---

## 快速上手路径（推荐顺序）

```
1. 设置 → 配置 AI Provider + API Key
2. 工作流 → 创建第一个 workflow（3 stages）
3. 工作流 → 运行 → 跳转执行记录查看输出
4. RAG 知识库 → 上传项目需求文档
5. AI 团队 → 团队 CLI → 创建团队 + 角色
6. 团队 CLI → 发送开发任务 → 查看流式输出
7. 知识库（Wisdom）→ 沉淀本次经验
8. 成本 → 查看 token 消耗
```

---

## 附录 A：API 端到端测试卡片（36 张）

> 按顺序执行，状态：⬜未测 / ✅通过 / ❌失败

### 环境准备

```bash
cd /Users/Zhuanz/Desktop/yp-nx-dashboard
cargo run --bin nx_api &
sleep 3
curl -s http://localhost:3000/health
```

---

### CARD-01 · 健康检查 ⬜
```bash
curl -s http://localhost:3000/health
```
**预期**：`{"status":"ok","service":"nexusflow-api"}`

---

### CARD-02 · Workflow CRUD ⬜
```bash
WF=$(curl -s -X POST http://localhost:3000/api/v1/workflows \
  -H "Content-Type: application/json" \
  -d '{"name":"e2e-wf","description":"test","stages":[{"name":"s1","prompt":"hello"}]}')
WF_ID=$(echo $WF | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

curl -s http://localhost:3000/api/v1/workflows/$WF_ID
curl -s -X PUT http://localhost:3000/api/v1/workflows/$WF_ID \
  -H "Content-Type: application/json" -d '{"name":"e2e-wf-updated"}'
curl -s http://localhost:3000/api/v1/workflows
curl -s -X DELETE http://localhost:3000/api/v1/workflows/$WF_ID
```
**预期**：CRUD 全部返回 2xx

---

### CARD-03 · Execution 生命周期 ⬜
```bash
EX=$(curl -s -X POST http://localhost:3000/api/v1/executions \
  -H "Content-Type: application/json" \
  -d "{\"workflow_id\":\"$WF_ID\",\"input\":{}}")
EX_ID=$(echo $EX | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

for i in 1 2 3 4 5; do
  STATUS=$(curl -s http://localhost:3000/api/v1/executions/$EX_ID | \
    python3 -c "import sys,json;print(json.load(sys.stdin)['data']['status'])")
  echo "[$i] status: $STATUS"
  [ "$STATUS" = "completed" ] || [ "$STATUS" = "failed" ] && break
  sleep 5
done
```
**预期**：状态从 `pending`→`running`→`completed`

---

### CARD-04 · Execution Git 信息 ⬜
```bash
curl -s http://localhost:3000/api/v1/executions/$EX_ID/git
```
**预期**：返回 commit hash 或 diff 信息

---

### CARD-05 · Session 生命周期 ⬜
```bash
SS=$(curl -s -X POST http://localhost:3000/api/v1/sessions \
  -H "Content-Type: application/json" \
  -d "{\"workflow_id\":\"$WF_ID\"}")
SS_ID=$(echo $SS | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

curl -s http://localhost:3000/api/v1/sessions/$SS_ID/messages
curl -s -X POST http://localhost:3000/api/v1/sessions/$SS_ID/pause
curl -s -X POST http://localhost:3000/api/v1/sessions/$SS_ID/sync
curl -s -X DELETE http://localhost:3000/api/v1/sessions/$SS_ID
```
**预期**：各操作返回 2xx，状态变更正确

---

### CARD-06 · Session 消息响应 ⬜
```bash
MSG_ID=$(curl -s http://localhost:3000/api/v1/sessions/$SS_ID/messages | \
  python3 -c "import sys,json;msgs=json.load(sys.stdin)['data'];print(msgs[0]['id'] if msgs else 'none')")

curl -s -X POST http://localhost:3000/api/v1/sessions/$SS_ID/messages/$MSG_ID/respond \
  -H "Content-Type: application/json" \
  -d '{"response":"approved","content":"looks good"}'
```
**预期**：返回 2xx，消息状态更新为已响应

---

### CARD-07 · Group Sessions 群组会话 ⬜
```bash
GS=$(curl -s -X POST http://localhost:3000/api/v1/group-sessions \
  -H "Content-Type: application/json" \
  -d '{"name":"e2e-group","members":["agent-1","agent-2"]}')
GS_ID=$(echo $GS | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

curl -s http://localhost:3000/api/v1/group-sessions/$GS_ID
curl -s http://localhost:3000/api/v1/group-sessions
```
**预期**：创建成功，列表包含新建项

---

### CARD-08 · Teams 团队管理 ⬜
```bash
TM=$(curl -s -X POST http://localhost:3000/api/v1/teams \
  -H "Content-Type: application/json" \
  -d '{"name":"e2e-team","description":"test team"}')
TM_ID=$(echo $TM | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

curl -s http://localhost:3000/api/v1/teams/$TM_ID/roles
curl -s http://localhost:3000/api/v1/teams
```
**预期**：团队创建成功，角色列表可查询

---

### CARD-09 · Roles 角色管理 ⬜
```bash
RL=$(curl -s -X POST http://localhost:3000/api/v1/roles \
  -H "Content-Type: application/json" \
  -d '{"name":"developer","description":"code role","team_id":"'$TM_ID'"}')
RL_ID=$(echo $RL | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

curl -s http://localhost:3000/api/v1/roles/$RL_ID/skills
curl -s -X POST http://localhost:3000/api/v1/roles/$RL_ID/execute \
  -H "Content-Type: application/json" \
  -d '{"prompt":"write hello world"}'
```
**预期**：角色 CRUD 正常，执行返回 AI 输出

---

### CARD-10 · Tasks 任务管理 ⬜
```bash
TK=$(curl -s -X POST http://localhost:3000/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{"title":"e2e-task","description":"test","status":"pending"}')
TK_ID=$(echo $TK | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

curl -s -X PUT http://localhost:3000/api/v1/tasks/$TK_ID \
  -H "Content-Type: application/json" -d '{"status":"in_progress"}'
curl -s http://localhost:3000/api/v1/tasks/stats
```
**预期**：任务状态流转正常，stats 返回各状态计数

---

### CARD-11 · Projects 项目管理 ⬜
```bash
PJ=$(curl -s -X POST http://localhost:3000/api/v1/projects \
  -H "Content-Type: application/json" \
  -d '{"name":"e2e-project","description":"test"}')
PJ_ID=$(echo $PJ | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

curl -s http://localhost:3000/api/v1/projects/$PJ_ID
curl -s -X DELETE http://localhost:3000/api/v1/projects/$PJ_ID
```
**预期**：CRUD 全部 2xx

---

### CARD-12 · Issues 问题追踪 ⬜
```bash
IS=$(curl -s -X POST http://localhost:3000/api/v1/issues \
  -H "Content-Type: application/json" \
  -d '{"title":"e2e-issue","description":"test bug","severity":"medium"}')
IS_ID=$(echo $IS | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

curl -s -X PUT http://localhost:3000/api/v1/issues/$IS_ID \
  -H "Content-Type: application/json" -d '{"status":"resolved"}'
```
**预期**：Issue 创建、状态更新正常

---

### CARD-13 · Skills 技能系统 ⬜
```bash
curl -s http://localhost:3000/api/v1/skills
curl -s http://localhost:3000/api/v1/skills/categories
curl -s "http://localhost:3000/api/v1/skills/search?query=code"

SK_ID=$(curl -s http://localhost:3000/api/v1/skills | \
  python3 -c "import sys,json;d=json.load(sys.stdin)['data'];print(d[0]['id'] if d else '')")
curl -s -X POST http://localhost:3000/api/v1/skills/$SK_ID/execute \
  -H "Content-Type: application/json" -d '{"params":{}}'
```
**预期**：预置技能列表非空，执行返回 `success:true`

---

### CARD-14 · Skills 导入 ⬜
```bash
curl -s -X POST http://localhost:3000/api/v1/skills/import \
  -H "Content-Type: application/json" \
  -d '{"source":"paste","content":"{\"id\":\"test-skill\",\"name\":\"Test\",\"description\":\"e2e\",\"category\":\"test\",\"code\":\"echo hello\"}"}'
```
**预期**：返回导入的技能详情

---

### CARD-15 · AI CLI 管理 ⬜
```bash
curl -s http://localhost:3000/api/v1/ai/clis
curl -s -X PUT http://localhost:3000/api/v1/ai/clis/config \
  -H "Content-Type: application/json" \
  -d '{"cli":"claude","enabled":true}'
```
**预期**：CLI 列表返回 claude/gemini 等

---

### CARD-16 · AI 模型管理 ⬜
```bash
curl -s http://localhost:3000/api/v1/ai/models
curl -s http://localhost:3000/api/v1/ai/selected
curl -s -X PUT http://localhost:3000/api/v1/ai/default \
  -H "Content-Type: application/json" \
  -d '{"model_id":"claude-sonnet-4-5","provider":"anthropic"}'
```
**预期**：模型列表非空，selected 返回当前模型

---

### CARD-17 · AI Provider V2 ⬜
```bash
PV=$(curl -s -X POST http://localhost:3000/api/v1/ai/v2/providers \
  -H "Content-Type: application/json" \
  -d '{"name":"test-provider","provider_key":"openai","api_key":"sk-test","base_url":"https://api.openai.com"}')
PV_ID=$(echo $PV | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")
curl -s -X DELETE http://localhost:3000/api/v1/ai/v2/providers/$PV_ID
```
**预期**：Provider CRUD 正常

---

### CARD-18 · AI 直接执行 ⬜
```bash
curl -s -X POST http://localhost:3000/api/v1/ai/execute \
  -H "Content-Type: application/json" \
  -d '{"prompt":"say hello","cli":"claude"}'
```
**预期**：返回 AI 输出文本（需 CLI 工具已安装）

---

### CARD-19 · API Keys 管理 ⬜
```bash
curl -s http://localhost:3000/api/v1/ai/api-keys
```
**预期**：返回已配置的 API key 列表（key 值脱敏）

---

### CARD-20 · 多模型路由 ⬜
```bash
curl -s http://localhost:3000/api/v1/ai/cli-model
curl -s http://localhost:3000/api/v1/model-routing/config 2>/dev/null || \
  curl -s http://localhost:3000/api/v1/ai/providers
```
**预期**：返回路由规则配置

---

### CARD-21 · Costs 费用统计 ⬜
```bash
curl -s http://localhost:3000/api/v1/costs/summary
curl -s "http://localhost:3000/api/v1/costs/by-day?days=7"
```
**预期**：summary 返回总 token/费用，by-day 返回按天分组数据

---

### CARD-22 · Workspaces 工作区 ⬜
```bash
mkdir -p /tmp/e2e-ws && git -C /tmp/e2e-ws init 2>/dev/null
WS=$(curl -s -X POST http://localhost:3000/api/v1/workspaces \
  -H "Content-Type: application/json" \
  -d '{"name":"e2e-ws","root_path":"/tmp/e2e-ws"}')
WS_ID=$(echo $WS | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

curl -s "http://localhost:3000/api/v1/workspaces/$WS_ID/browse"
echo "test" > /tmp/e2e-ws/new.txt
curl -s http://localhost:3000/api/v1/workspaces/$WS_ID/diffs
curl -s -X DELETE http://localhost:3000/api/v1/workspaces/$WS_ID
```
**预期**：文件浏览返回目录列表，diffs 包含 `new.txt`

---

### CARD-23 · Knowledge 知识库 ⬜
```bash
echo "NexusFlow 是 AI 软件工厂" > /tmp/kb-doc.txt
curl -s -X POST http://localhost:3000/api/v1/knowledge \
  -F "file=@/tmp/kb-doc.txt"
curl -s http://localhost:3000/api/v1/knowledge
curl -s "http://localhost:3000/api/v1/knowledge/search?q=AI软件工厂"
```
**预期**：上传后分块数 > 0，搜索返回相关 chunks

---

### CARD-24 · Triggers 触发器 ⬜
```bash
TR=$(curl -s -X POST http://localhost:3000/api/v1/triggers \
  -H "Content-Type: application/json" \
  -d "{\"name\":\"e2e-trigger\",\"type\":\"cron\",\"cron\":\"* * * * *\",\"workflow_id\":\"$WF_ID\"}")
TR_ID=$(echo $TR | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

curl -s http://localhost:3000/api/v1/triggers
sleep 65
curl -s http://localhost:3000/api/v1/executions | \
  python3 -c "import sys,json;d=json.load(sys.stdin)['data'];print(len(d),'executions')"
curl -s -X DELETE http://localhost:3000/api/v1/triggers/$TR_ID
```
**预期**：65 秒后执行列表新增 1 条触发器创建的记录

---

### CARD-25 · Scheduler 调度器 ⬜
```bash
curl -s http://localhost:3000/api/v1/scheduler/jobs 2>/dev/null || \
  curl -s http://localhost:3000/api/v1/scheduler/status
```
**预期**：返回当前调度任务列表及状态

---

### CARD-26 · Templates 模板 ⬜
```bash
TP=$(curl -s -X POST http://localhost:3000/api/v1/templates \
  -H "Content-Type: application/json" \
  -d '{"name":"e2e-template","content":"# Template\n{{prompt}}","category":"code"}')
TP_ID=$(echo $TP | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

curl -s http://localhost:3000/api/v1/templates/$TP_ID
curl -s -X DELETE http://localhost:3000/api/v1/templates/$TP_ID
```
**预期**：模板 CRUD 正常

---

### CARD-27 · Search 全局搜索 ⬜
```bash
curl -s "http://localhost:3000/api/v1/search?q=workflow"
curl -s http://localhost:3000/api/v1/search/modes
curl -s -X POST http://localhost:3000/api/v1/search/index
```
**预期**：搜索返回跨模块结果，modes 返回可用搜索模式

---

### CARD-28 · Wisdom 知识沉淀 ⬜
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

### CARD-29 · Test Gen 测试生成 ⬜
```bash
curl -s -X POST http://localhost:3000/api/v1/test-gen \
  -H "Content-Type: application/json" \
  -d '{"code":"fn add(a: i32, b: i32) -> i32 { a + b }","language":"rust"}'
```
**预期**：返回生成的测试代码（需 CLI 工具）

---

### CARD-30 · Plugins 插件 ⬜
```bash
curl -s http://localhost:3000/api/v1/plugins
```
**预期**：返回已安装插件列表（可为空）

---

### CARD-31 · Processes 进程管理 ⬜
```bash
curl -s http://localhost:3000/api/v1/processes
```
**预期**：返回当前运行中的进程列表（PTY/CLI 进程）

---

### CARD-32 · Execution Logs ⬜
```bash
curl -s http://localhost:3000/api/v1/executions/$EX_ID/logs 2>/dev/null || \
  curl -s "http://localhost:3000/api/v1/execution-logs?execution_id=$EX_ID"
```
**预期**：返回日志数组，每条含 `timestamp`、`level`、`message`

---

### CARD-33 · WebSocket — 执行实时推送 ⬜
```bash
# 需要 websocat：brew install websocat
websocat ws://localhost:3000/ws/executions/$EX_ID &
WS_PID=$!; sleep 10; kill $WS_PID
```
**预期**：连接建立后收到执行状态变更消息

---

### CARD-34 · WebSocket — Session 流 ⬜
```bash
websocat ws://localhost:3000/ws/sessions/$SS_ID &
WS_PID=$!; sleep 5; kill $WS_PID
```
**预期**：连接建立，收到 session 消息推送

---

### CARD-35 · WebSocket — 终端 & 命令执行 ⬜
```bash
websocat ws://localhost:3000/ws/terminal &
WS_PID=$!; sleep 3; kill $WS_PID
```
**预期**：WS 连接建立，不立即断开

---

### CARD-36 · Tauri 桌面应用启动 ⬜
```bash
cd /Users/Zhuanz/Desktop/yp-nx-dashboard/nx_dashboard
cargo tauri dev
```
**验证清单**：
- [ ] 应用窗口正常打开，无崩溃
- [ ] 侧边栏所有菜单项可点击，无白屏
- [ ] DevTools 控制台无 `Error` 级别报错
- [ ] 执行页面实时更新（WS 连接正常）

---

### CARD-37 · AI Chat 对话 ⬜
```bash
curl -s -X POST http://localhost:3000/api/v1/ai/chat \
  -H "Content-Type: application/json" \
  -d '{"message":"hello","context":[]}'
```
**预期**：返回 AI 回复文本（需 CLI 工具已安装）

---

### CARD-38 · AI 模型刷新 ⬜
```bash
curl -s -X POST http://localhost:3000/api/v1/ai/models/refresh
```
**预期**：返回刷新后的模型列表

---

### CARD-39 · Skills 标签系统 ⬜
```bash
curl -s http://localhost:3000/api/v1/skills/tags
curl -s http://localhost:3000/api/v1/skills/stats
curl -s http://localhost:3000/api/v1/skills/tag/code
```
**预期**：tags 返回标签列表，stats 返回统计数，tag/:tag 返回该标签下的技能

---

### CARD-40 · Test Gen 单元测试 ⬜
```bash
curl -s -X POST http://localhost:3000/api/v1/test-gen/unit \
  -H "Content-Type: application/json" \
  -d '{"function":"add","language":"rust"}'
```
**预期**：返回生成的单元测试代码

---

### CARD-41 · Webhook 触发工作流 ⬜
```bash
# 触发指定 workflow（需先有 WF_ID）
curl -s -X POST "http://localhost:3000/api/v1/triggers/webhook/$WF_ID" \
  -H "Content-Type: application/json" \
  -d '{"event":"push","ref":"refs/heads/main"}'
```
**预期**：返回新建的 execution ID，workflow 开始执行

---

### CARD-42 · 定时任务 CRUD ⬜
```bash
# 创建定时任务
SJ=$(curl -s -X POST http://localhost:3000/api/v1/tasks/schedule \
  -H "Content-Type: application/json" \
  -d "{\"workflow_id\":\"$WF_ID\",\"cron\":\"0 9 * * *\",\"name\":\"daily-build\"}")
SJ_ID=$(echo $SJ | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

# 列表
curl -s http://localhost:3000/api/v1/tasks/scheduled

# 启用/禁用
curl -s -X PUT "http://localhost:3000/api/v1/tasks/scheduled/$SJ_ID/toggle"

# 删除
curl -s -X DELETE "http://localhost:3000/api/v1/tasks/scheduled/$SJ_ID"
```
**预期**：定时任务 CRUD 正常，toggle 切换启用状态

---

### CARD-43 · RAG 知识库完整流程 ⬜
**路由**：`/api/v1/knowledge-bases`（注意：不是 `/api/v1/knowledge`）
```bash
# 创建知识库
KB=$(curl -s -X POST http://localhost:3000/api/v1/knowledge-bases \
  -H "Content-Type: application/json" \
  -d '{"name":"e2e-kb"}')
KB_ID=$(echo $KB | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

# 上传文档
echo "NexusFlow 是 AI 软件工厂" > /tmp/kb-doc.txt
curl -s -X POST http://localhost:3000/api/v1/knowledge-bases/upload \
  -F "file=@/tmp/kb-doc.txt"

# 列出文档
curl -s "http://localhost:3000/api/v1/knowledge-bases/$KB_ID/documents"

# 语义搜索
curl -s -X POST http://localhost:3000/api/v1/knowledge-bases/search \
  -H "Content-Type: application/json" \
  -d '{"query":"AI软件工厂","limit":5}'

# Embedding 配置
curl -s http://localhost:3000/api/v1/knowledge-bases/embedding-config

# 删除知识库
curl -s -X DELETE "http://localhost:3000/api/v1/knowledge-bases/$KB_ID"
```
**预期**：知识库 CRUD 正常，搜索返回相关 chunks

---

### CARD-44 · Sprint 看板 API ⬜
**路由**：`/api/v1/sprints`
```bash
# 创建/更新 Sprint
SP=$(curl -s -X POST http://localhost:3000/api/v1/sprints \
  -H "Content-Type: application/json" \
  -d '{"name":"Sprint-1","goal":"完成商品模块","start_date":"2026-05-01","end_date":"2026-05-14"}')
SP_ID=$(echo $SP | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

# 列表
curl -s http://localhost:3000/api/v1/sprints

# 更新状态
curl -s -X PUT "http://localhost:3000/api/v1/sprints/$SP_ID/status" \
  -H "Content-Type: application/json" \
  -d '{"status":"active"}'

# 添加事件
curl -s -X POST "http://localhost:3000/api/v1/sprints/$SP_ID/events" \
  -H "Content-Type: application/json" \
  -d '{"type":"task_completed","description":"完成商品列表 API"}'

# Sprint 报告
curl -s "http://localhost:3000/api/v1/sprints/$SP_ID/report"
```
**预期**：Sprint CRUD 正常，事件记录，报告生成

---

### CARD-45 · AI 提供商管理 ⬜
**路由**：`/api/v1/ai/providers`、`/api/v1/ai/default`、`/api/v1/ai/selected`
```bash
# 获取所有提供商
curl -s http://localhost:3000/api/v1/ai/providers

# 获取默认提供商
curl -s http://localhost:3000/api/v1/ai/default

# 获取已选提供商
curl -s http://localhost:3000/api/v1/ai/selected
```
**预期**：返回提供商列表及当前选中状态

---

### CARD-46 · AI CLI 配置 ⬜
**路由**：`/api/v1/ai/cli-model`、`/api/v1/ai/clis/config`、`/api/v1/ai/execute`
```bash
# 获取 CLI 模型配置
curl -s http://localhost:3000/api/v1/ai/cli-model

# 获取 CLI 配置
curl -s http://localhost:3000/api/v1/ai/clis/config

# 直接执行 AI 任务
curl -s -X POST http://localhost:3000/api/v1/ai/execute \
  -H "Content-Type: application/json" \
  -d '{"prompt":"Hello","model":"claude-opus-4-6"}'
```
**预期**：CLI 配置可读，AI 执行返回结果

---

### CARD-47 · Browser 自动化 ⬜
**路由**：`/api/v1/browser`
```bash
curl -s -X POST http://localhost:3000/api/v1/browser \
  -H "Content-Type: application/json" \
  -d '{"url":"https://example.com","action":"screenshot"}'
```
**预期**：返回浏览器操作结果

---

### CARD-48 · Search 索引与模式 ⬜
**路由**：`/api/v1/search/index`、`/api/v1/search/modes`
```bash
# 获取搜索模式
curl -s http://localhost:3000/api/v1/search/modes

# 触发索引重建
curl -s -X POST http://localhost:3000/api/v1/search/index
```
**预期**：返回可用搜索模式列表，索引触发成功

---

### CARD-49 · Skills 分类/搜索/导入 ⬜
**路由**：`/api/v1/skills/categories`、`/api/v1/skills/search`、`/api/v1/skills/import`
```bash
# 获取技能分类
curl -s http://localhost:3000/api/v1/skills/categories

# 搜索技能
curl -s "http://localhost:3000/api/v1/skills/search?q=python"

# 导入技能
curl -s -X POST http://localhost:3000/api/v1/skills/import \
  -H "Content-Type: application/json" \
  -d '{"skills":[{"name":"test-skill","description":"test"}]}'
```
**预期**：分类列表正常，搜索返回匹配结果，导入成功

---

### CARD-50 · Tasks 统计 ⬜
**路由**：`/api/v1/tasks/stats`
```bash
curl -s http://localhost:3000/api/v1/tasks/stats
```
**预期**：返回任务统计数据（总数、按状态分布等）

---

### CARD-51 · 临时文件清理 ⬜
**路由**：`/api/v1/temp-cleanup`
```bash
curl -s -X POST http://localhost:3000/api/v1/temp-cleanup
```
**预期**：清理成功，返回清理文件数量

---

### API 测试汇总表（51 张）

| 卡片 | 功能模块 | 路由 | 状态 |
|------|---------|------|------|
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
| CARD-18 | AI 直接执行 | `/ai/execute` | ⬜ |
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
| CARD-37 | AI Chat 对话 | `POST /api/v1/ai/chat` | ⬜ |
| CARD-38 | AI 模型刷新 | `POST /api/v1/ai/models/refresh` | ⬜ |
| CARD-39 | Skills 标签/统计 | `/api/v1/skills/tags`, `/stats`, `/tag/:tag` | ⬜ |
| CARD-40 | Test Gen 单元测试 | `POST /api/v1/test-gen/unit` | ⬜ |
| CARD-41 | Webhook 触发 | `POST /api/v1/triggers/webhook/:workflow_id` | ⬜ |
| CARD-42 | 定时任务 CRUD | `/api/v1/tasks/schedule`, `/tasks/scheduled` | ⬜ |
| CARD-43 | RAG 知识库完整 | `/api/v1/knowledge-bases` | ⬜ |
| CARD-44 | Sprint 看板 API | `/api/v1/sprints` | ⬜ |
| CARD-45 | AI 提供商管理 | `/api/v1/ai/providers`, `/ai/default`, `/ai/selected` | ⬜ |
| CARD-46 | AI CLI 配置 | `/api/v1/ai/cli-model`, `/ai/clis/config`, `/ai/execute` | ⬜ |
| CARD-47 | Browser 自动化 | `/api/v1/browser` | ⬜ |
| CARD-48 | Search 索引与模式 | `/api/v1/search/index`, `/search/modes` | ⬜ |
| CARD-49 | Skills 分类/搜索/导入 | `/api/v1/skills/categories`, `/skills/search`, `/skills/import` | ⬜ |
| CARD-50 | Tasks 统计 | `/api/v1/tasks/stats` | ⬜ |
| CARD-51 | 临时文件清理 | `/api/v1/temp-cleanup` | ⬜ |

**通过标准**：44/44 全绿 = 全功能端到端跑通


---

## 附录 B：UI 端到端验证卡片（14 张）

> 在桌面应用（`cargo tauri dev`）中按顺序操作，状态：⬜未测 / ✅通过 / ❌失败

### 环境准备

```bash
cd /Users/Zhuanz/Desktop/yp-nx-dashboard
cargo run --bin nx_api &
cd nx_dashboard && npm run dev
# 或桌面应用：cargo tauri dev
```

---

### UI-01 · 工作区创建 ⬜

1. 侧边栏 → **工作区**
2. 点击「新建工作区」
3. 填写：名称 `e2e-test-project`，路径 `/tmp/e2e-test`
4. 点击确认

**预期**：工作区出现在列表，文件浏览器显示目录内容

---

### UI-02 · 需求输入与任务分解 ⬜

1. 侧边栏 → **项目**
2. 输入需求：`实现一个 TODO List，支持增删改查，用 Python + SQLite`
3. 点击「分解」或「生成任务」

**预期**：生成 3-6 个子任务，每个任务有标题和描述

---

### UI-03 · Pipeline 执行 ⬜

1. 侧边栏 → **执行**
2. 选择已有 workflow，点击「运行」
3. 观察执行状态变化

**预期**：状态从 `pending`→`running`→`completed`，执行日志实时滚动

---

### UI-04 · 实时观测 ⬜

1. 在 UI-03 执行过程中，观察执行页面
2. 查看：进度条、当前步骤、Token 消耗、耗时

**预期**：数据实时更新，不需要手动刷新（WebSocket 正常）

---

### UI-05 · 断点续跑 ⬜

1. 启动多步骤执行（至少 3 步）
2. 在第 2 步执行中，重启后端
3. 回到前端，找到该执行记录，点击「继续」

**预期**：从第 2 步继续，不从第 1 步重新开始

**已知风险**：Checkpoint 表可能未写入，此功能可能失效

---

### UI-06 · 质量门 ⬜

1. 创建带质量门的 workflow（YAML 中配置 `quality_gate: true`）
2. 故意让某步骤输出包含错误关键词
3. 运行该 workflow

**预期**：质量门检测到失败，执行状态变为 `failed`，不继续后续步骤

---

### UI-07 · 失败自愈 ⬜

1. 配置会失败的步骤，开启 `auto_retry: true`
2. 运行并等待

**预期**：失败后自动重试 1-3 次，日志显示重试记录

---

### UI-08 · Git 集成 ⬜

1. 确保工作区目录是 git 仓库
2. 执行会生成文件的任务
3. 执行完成后，在工作区「Git」面板查看 diff

**预期**：出现新的 commit，diff 显示 AI 生成的文件变更

---

### UI-09 · 技能系统 ⬜

1. 侧边栏 → **技能**
2. 找到预置技能（如代码审查、文档生成）
3. 点击「执行」，填写参数，等待结果

**预期**：技能执行成功，输出结果显示在页面

---

### UI-10 · RAG 知识库 ⬜

1. 侧边栏 → **知识库**
2. 上传一个文档（txt 或 md 文件）
3. 等待索引完成
4. 在搜索框输入问题，查看返回结果

**预期**：文档被分块索引，搜索返回相关片段

---

### UI-11 · Token/Cost 监控 ⬜

1. 完成几次执行后
2. 侧边栏 → **成本**

**预期**：显示 Token 消耗、估算费用、按模型分类

---

### UI-12 · 多模型路由 ⬜

1. 侧边栏 → **AI 设置**
2. 配置路由规则：代码任务 → Claude，文档任务 → 其他模型
3. 运行代码生成任务，观察实际使用的模型

**预期**：执行日志显示使用了路由规则指定的模型

---

### UI-13 · 可视化画布 ⬜

1. 侧边栏 → **画布**
2. 拖拽 2-3 个节点，连接成简单 workflow
3. 点击「导出 YAML」
4. 用该 YAML 创建执行

**预期**：YAML 格式正确，能被 Pipeline 解析并执行

---

### UI-14 · 触发器 ⬜

1. 侧边栏 → **触发器**
2. 创建定时触发器（每分钟触发一次）
3. 等待触发，观察执行列表是否自动新增

**预期**：到达触发时间后，自动创建并开始一次执行

---

### UI 验证汇总表

| 卡片 | 模块 | 状态 | 备注 |
|------|------|------|------|
| UI-01 | 工作区 | ⬜ | |
| UI-02 | 需求分解 | ⬜ | |
| UI-03 | Pipeline 执行 | ⬜ | |
| UI-04 | 实时观测 | ⬜ | 依赖 WebSocket |
| UI-05 | 断点续跑 | ⬜ | 已知风险 |
| UI-06 | 质量门 | ⬜ | |
| UI-07 | 失败自愈 | ⬜ | |
| UI-08 | Git 集成 | ⬜ | |
| UI-09 | 技能系统 | ⬜ | |
| UI-10 | RAG 知识库 | ⬜ | |
| UI-11 | Token/Cost | ⬜ | |
| UI-12 | 多模型路由 | ⬜ | |
| UI-13 | 可视化画布 | ⬜ | |
| UI-14 | 触发器 | ⬜ | |


---

## 附录 C：TodoFlow 验收剧本（用系统开发真实项目）

> 你不是在测试接口，你是在**用 NexusFlow 开发一个真实软件**。
> 按剧本走完全程，最终交付一个可运行的 Todo List 应用。

**目标**：Python 命令行 Todo List，SQLite 存储，验收标准：`python todo.py add "买牛奶"` 能运行

### ACT 1 · 项目初始化

```bash
mkdir -p /tmp/todoflow && git -C /tmp/todoflow init

WS_ID=$(curl -s -X POST http://localhost:3000/api/v1/workspaces \
  -H "Content-Type: application/json" \
  -d '{"name":"todoflow","root_path":"/tmp/todoflow"}' | \
  python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

PJ_ID=$(curl -s -X POST http://localhost:3000/api/v1/projects \
  -H "Content-Type: application/json" \
  -d '{"name":"TodoFlow","description":"命令行 Todo List，Python + SQLite"}' | \
  python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

cat > /tmp/todoflow-spec.md << 'EOF'
# TodoFlow 需求规格
- add <text>：添加一条 todo
- list：列出所有未完成 todo
- done <id>：标记完成
- delete <id>：删除
技术：Python 3.8+，SQLite，单文件 todo.py，无第三方依赖
EOF

KB_ID=$(curl -s -X POST http://localhost:3000/api/v1/knowledge \
  -F "file=@/tmp/todoflow-spec.md" | \
  python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")
```

### ACT 2 · 任务分解

```bash
for task in '{"title":"设计 SQLite schema","description":"设计 todos 表结构"}' \
            '{"title":"实现 CRUD 逻辑","description":"add/list/done/delete 四个命令"}' \
            '{"title":"实现 CLI 入口","description":"argparse 解析命令行参数"}' \
            '{"title":"编写单元测试","description":"pytest 测试 CRUD 逻辑"}'; do
  curl -s -X POST http://localhost:3000/api/v1/tasks \
    -H "Content-Type: application/json" \
    -d "$task" | python3 -c "import sys,json;d=json.load(sys.stdin)['data'];print(d['id'],d['title'])"
done
```

### ACT 3 · 创建 Workflow

```bash
WF_ID=$(curl -s -X POST http://localhost:3000/api/v1/workflows \
  -H "Content-Type: application/json" \
  -d '{
    "name": "todoflow-build",
    "stages": [
      {"name":"design","prompt":"根据需求设计 SQLite schema，输出建表 SQL"},
      {"name":"implement","prompt":"实现 todo.py，包含 add/list/done/delete 命令，使用 SQLite"},
      {"name":"test-gen","prompt":"为 todo.py 生成 pytest 单元测试文件 test_todo.py"},
      {"name":"review","prompt":"代码审查：检查 todo.py 的错误处理、边界条件、SQL 注入风险"}
    ]
  }' | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")
```

### ACT 4 · 执行生产线

```bash
EX_ID=$(curl -s -X POST http://localhost:3000/api/v1/executions \
  -H "Content-Type: application/json" \
  -d "{\"workflow_id\":\"$WF_ID\",\"input\":{\"working_dir\":\"/tmp/todoflow\"}}" | \
  python3 -c "import sys,json;print(json.load(sys.stdin)['data']['id'])")

# 轮询直到完成
watch -n 10 "curl -s http://localhost:3000/api/v1/executions/$EX_ID | \
  python3 -c \"import sys,json; d=json.load(sys.stdin)['data']; \
  print('status:', d['status'], '| stages:', len(d.get('stage_results',[])), 'done')\""
```

### ACT 5 · 提取产物

```bash
curl -s http://localhost:3000/api/v1/executions/$EX_ID | \
  python3 -c "
import sys, json, re
d = json.load(sys.stdin)['data']
for s in d.get('stage_results', []):
    if s['stage_name'] == 'implement':
        output = str(s.get('output', ''))
        match = re.search(r'\`\`\`python\n(.*?)\`\`\`', output, re.DOTALL)
        if match:
            with open('/tmp/todoflow/todo.py', 'w') as f:
                f.write(match.group(1))
            print('Written todo.py')
"
```

### ACT 6 · 验收测试

```bash
cd /tmp/todoflow
python3 todo.py add "买牛奶"
python3 todo.py add "写代码"
python3 todo.py list
python3 todo.py done 1
python3 todo.py list
python3 todo.py delete 2
python3 todo.py list
```

**预期输出**：
```
Added: [1] 买牛奶
Added: [2] 写代码
[1] 买牛奶  [2] 写代码
Done: [1] 买牛奶
[2] 写代码
(empty)
```

### ACT 7 · Git 提交

```bash
curl -s http://localhost:3000/api/v1/workspaces/$WS_ID/diffs
```

### ACT 8 · 知识沉淀

```bash
curl -s -X POST http://localhost:3000/api/v1/wisdom \
  -H "Content-Type: application/json" \
  -d '{"title":"Python CLI 工具开发模式","content":"单文件 + argparse + SQLite 是最简 Python CLI 工具的标准模式","category":"engineering"}'
```

### TodoFlow 验收结果表

| 步骤 | 功能模块 | 预期 | 状态 |
|------|---------|------|------|
| 1 | 工作区+项目+知识库 | ID 非空 | ⬜ |
| 2 | 任务分解 | 4 个任务创建成功 | ⬜ |
| 3 | Workflow 创建 | WF_ID 非空 | ⬜ |
| 4 | 执行完成 | status=completed | ⬜ |
| 5 | 代码提取 | todo.py 写入成功 | ⬜ |
| 6 | 代码可运行 | `python3 todo.py add` 成功 | ⬜ |
| 7 | Git Diff 可见 | todo.py 在 diff 列表 | ⬜ |
| 8 | 知识沉淀 | Wisdom 创建成功 | ⬜ |

**通过标准**：步骤 6（代码可运行）必须通过。


---

## 附录 D：Playwright 自动化测试（19 个 spec）

> 运行命令：`cd nx_dashboard && npx playwright test`

### 环境准备

```bash
cd /Users/Zhuanz/Desktop/yp-nx-dashboard
cargo run --bin nx_api &
sleep 3
cd nx_dashboard
npx playwright test --reporter=html
# 查看报告
npx playwright show-report
```

### 19 个测试文件

| 文件 | 覆盖功能 | 运行命令 |
|------|---------|---------|
| `workflow-crud.spec.ts` | Workflow 增删改查 | `npx playwright test workflow-crud` |
| `workflow-creation.spec.ts` | Workflow 创建流程 | `npx playwright test workflow-creation` |
| `execution-lifecycle.spec.ts` | 执行生命周期 | `npx playwright test execution-lifecycle` |
| `pipeline-lifecycle.spec.ts` | Pipeline 完整流程 | `npx playwright test pipeline-lifecycle` |
| `checkpoint-resume.spec.ts` | 断点续跑 | `npx playwright test checkpoint-resume` |
| `session-lifecycle.spec.ts` | Session 生命周期 | `npx playwright test session-lifecycle` |
| `teams-roles.spec.ts` | 团队与角色管理 | `npx playwright test teams-roles` |
| `group-sessions-processes.spec.ts` | 群组会话+进程 | `npx playwright test group-sessions` |
| `skills.spec.ts` | 技能系统 | `npx playwright test skills` |
| `knowledge-base.spec.ts` | RAG 知识库 | `npx playwright test knowledge-base` |
| `ai-config.spec.ts` | AI 配置 | `npx playwright test ai-config` |
| `tasks-projects-issues.spec.ts` | 任务/项目/问题 | `npx playwright test tasks-projects` |
| `triggers-scheduler.spec.ts` | 触发器+调度器 | `npx playwright test triggers-scheduler` |
| `templates-plugins-testgen.spec.ts` | 模板/插件/测试生成 | `npx playwright test templates-plugins` |
| `workspaces.spec.ts` | 工作区管理 | `npx playwright test workspaces` |
| `costs-search-wisdom.spec.ts` | 成本/搜索/知识 | `npx playwright test costs-search` |
| `websockets.spec.ts` | WebSocket 连接 | `npx playwright test websockets` |
| `terminal-multiwindow.spec.ts` | 终端多窗口 | `npx playwright test terminal` |
| `helpers.ts` | 共享工具函数 | （非测试文件，被其他 spec 引用） |

### 全量运行

```bash
# 全量运行（无头模式）
npx playwright test

# 有头模式（可看到浏览器操作）
npx playwright test --headed

# 并行运行（加速）
npx playwright test --workers=4

# 失败时截图
npx playwright test --screenshot=only-on-failure

# 生成 HTML 报告
npx playwright test --reporter=html && npx playwright show-report
```


---

## 附录 E：全功能覆盖矩阵

> 每个模块对应的测试覆盖情况一览

| 模块 | UI 场景 | API 卡片 | Playwright | 验收剧本 |
|------|---------|---------|-----------|---------|
| 仪表盘 | 场景一~三 | CARD-01 | - | - |
| 工作流 | 场景一 S1-3~S1-6 | CARD-02 | workflow-crud, workflow-creation | ACT 3 |
| 执行记录 | 场景一 S1-7 | CARD-03,04,32 | execution-lifecycle, pipeline-lifecycle | ACT 4 |
| 可视化画布 | UI-13 | - | - | - |
| Sprint 看板 | 场景一 S1-2 | CARD-10,11,12 | tasks-projects-issues | ACT 2 |
| AI 团队（PTY） | 场景三 S3-1~S3-8 | CARD-08,09 | teams-roles | - |
| 团队 CLI（V2） | 场景三 S3-4~S3-6 | CARD-08,09 | teams-roles | - |
| 角色 | 场景三 S3-2 | CARD-09 | teams-roles | - |
| 技能 | UI-09 | CARD-13,14 | skills | - |
| 群组讨论 | 场景三 S3-7 | CARD-07 | group-sessions-processes | - |
| 会话 | 场景二 S2-5 | CARD-05,06 | session-lifecycle | - |
| 进程检测 | - | CARD-31 | group-sessions-processes | - |
| 项目 | 场景一 S1-1 | CARD-11 | tasks-projects-issues | ACT 1 |
| 模板 | 场景二 S2-1 | CARD-26 | templates-plugins-testgen | - |
| 知识库（Wisdom） | 场景一 S1-8 | CARD-28 | costs-search-wisdom | ACT 8 |
| RAG 知识库 | UI-10 | CARD-23 | knowledge-base | ACT 1.3 |
| 终端 | - | CARD-35 | terminal-multiwindow | - |
| 搜索 | - | CARD-27 | costs-search-wisdom | - |
| 成本 | UI-11 | CARD-21 | costs-search-wisdom | - |
| AI 设置 | UI-12 | CARD-15~20 | ai-config | - |
| 触发器 | UI-14 | CARD-24,25 | triggers-scheduler | - |
| 工作区 | UI-01 | CARD-22 | workspaces | ACT 1.1 |
| WebSocket | UI-04 | CARD-33~35 | websockets | - |
| 断点续跑 | UI-05 | - | checkpoint-resume | - |
| 质量门 | UI-06 | - | - | - |
| Git 集成 | UI-08 | CARD-04 | - | ACT 7 |

---

## 附录 F：已知风险与限制

| 风险 | 影响范围 | 说明 | 处理方式 |
|------|---------|------|---------|
| Checkpoint 表未写入 | UI-05, checkpoint-resume | 断点续跑可能失效 | 检查 `checkpoints` 表：`sqlite3 nx_api/nexus.db "SELECT * FROM checkpoints LIMIT 5;"` |
| `.expect()` 崩溃点 | 全部 | 后端在特定操作时可能 panic | 查看 `cargo run` 日志，定位 panic 位置 |
| CLI 工具未安装 | CARD-18,29, 场景一~三 | 需本机有 `claude` 或 `gemini` CLI | `which claude` 检查，安装后重试 |
| websocat 未安装 | CARD-33~35 | WebSocket 命令行测试工具 | `brew install websocat` |
| 端口冲突 | 全部 | 后端默认 3000，前端默认 5173 | 检查 `lsof -i :3000` |
| 场景一~三 端口 | 场景一~三 | 场景使用 8080，API 卡片使用 3000 | 统一使用 `cargo run --bin nx_api` 的实际端口 |

> **注意**：场景一~三中的 `localhost:3000` 与 API 卡片中的 `localhost:3000` 需根据实际启动端口调整。运行 `curl -s http://localhost:3000/health` 确认实际端口。

---

## 总览：测试执行顺序建议

```
第一步：环境验证
  → CARD-01 健康检查
  → CARD-36 Tauri 桌面应用启动

第二步：基础功能（API 层）
  → CARD-02~04 Workflow + Execution
  → CARD-22 工作区
  → CARD-10~12 任务/项目/问题

第三步：AI 核心功能
  → CARD-13~14 技能系统
  → CARD-23 知识库
  → CARD-18 AI 直接执行

第四步：团队协作
  → CARD-08~09 团队+角色
  → CARD-07 群组会话
  → 场景三：AI 团队 UI 操作

第五步：完整项目验收
  → 附录 C：TodoFlow 验收剧本（8 个 ACT）
  → 场景一：ShopFlow 手动驱动
  → 场景二：ShopFlow 复用 Workflow
  → 场景三：ShopFlow AI 团队协作

第六步：自动化回归
  → npx playwright test（19 个 spec）
```
