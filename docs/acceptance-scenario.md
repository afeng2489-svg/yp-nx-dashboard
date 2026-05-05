# NexusFlow 验收剧本：用系统本身开发一个完整项目

> **给 AI 执行员的指令**：
> 你不是在测试接口，你是在**用 NexusFlow 开发一个真实软件**。
> 按剧本走完全程，最终交付一个可运行的 Todo List 应用。
> 每一步都要用系统的功能完成，不能绕过系统直接写代码。

---

## 目标项目

**名称**：TodoFlow  
**描述**：一个命令行 Todo List，支持增删改查，数据存 SQLite  
**语言**：Python  
**验收标准**：`python todo.py add "买牛奶"` 能运行，数据持久化

---

## 环境启动

```bash
cd /Users/Zhuanz/Desktop/yp-nx-dashboard
cargo run --bin nx_api &
sleep 3
curl -s http://localhost:3000/health
```

---

## ACT 1 · 项目初始化

### 1.1 创建工作区

```bash
mkdir -p /tmp/todoflow && git -C /tmp/todoflow init
curl -s -X POST http://localhost:3000/api/v1/workspaces \
  -H "Content-Type: application/json" \
  -d '{"name":"todoflow","root_path":"/tmp/todoflow"}'
```

**记录**：`WS_ID=<返回的 id>`

### 1.2 创建项目

```bash
curl -s -X POST http://localhost:3000/api/v1/projects \
  -H "Content-Type: application/json" \
  -d '{"name":"TodoFlow","description":"命令行 Todo List，Python + SQLite"}'
```

**记录**：`PJ_ID=<返回的 id>`

### 1.3 上传需求文档到知识库

```bash
cat > /tmp/todoflow-spec.md << 'EOF'
# TodoFlow 需求规格

## 功能需求
- add <text>：添加一条 todo
- list：列出所有未完成 todo
- done <id>：标记完成
- delete <id>：删除

## 技术要求
- Python 3.8+
- SQLite 存储，文件名 todo.db
- 单文件实现 todo.py
- 无第三方依赖

## 验收标准
python todo.py add "买牛奶"  → Added: [1] 买牛奶
python todo.py list          → [1] 买牛奶
python todo.py done 1        → Done: [1] 买牛奶
python todo.py list          → (empty)
EOF

curl -s -X POST http://localhost:3000/api/v1/knowledge \
  -F "file=@/tmp/todoflow-spec.md"
```

**记录**：`KB_ID=<返回的 id>`

---

## ACT 2 · 任务分解

### 2.1 创建任务列表

```bash
# 任务 1：设计数据库 schema
T1=$(curl -s -X POST http://localhost:3000/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{"title":"设计 SQLite schema","description":"设计 todos 表结构","status":"pending"}')

# 任务 2：实现核心逻辑
T2=$(curl -s -X POST http://localhost:3000/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{"title":"实现 CRUD 逻辑","description":"add/list/done/delete 四个命令","status":"pending"}')

# 任务 3：实现 CLI 入口
T3=$(curl -s -X POST http://localhost:3000/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{"title":"实现 CLI 入口","description":"argparse 解析命令行参数","status":"pending"}')

# 任务 4：编写测试
T4=$(curl -s -X POST http://localhost:3000/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{"title":"编写单元测试","description":"pytest 测试 CRUD 逻辑","status":"pending"}')
```

**记录**：`T1_ID, T2_ID, T3_ID, T4_ID`

---

## ACT 3 · 创建 Workflow

### 3.1 创建生产线 Workflow

```bash
curl -s -X POST http://localhost:3000/api/v1/workflows \
  -H "Content-Type: application/json" \
  -d '{
    "name": "todoflow-build",
    "description": "TodoFlow 完整构建流水线",
    "stages": [
      {
        "name": "design",
        "prompt": "根据需求设计 SQLite schema，输出建表 SQL"
      },
      {
        "name": "implement",
        "prompt": "实现 todo.py，包含 add/list/done/delete 命令，使用 SQLite"
      },
      {
        "name": "test-gen",
        "prompt": "为 todo.py 生成 pytest 单元测试文件 test_todo.py"
      },
      {
        "name": "review",
        "prompt": "代码审查：检查 todo.py 的错误处理、边界条件、SQL 注入风险"
      }
    ]
  }'
```

**记录**：`WF_ID=<返回的 id>`

---

## ACT 4 · 执行生产线

### 4.1 启动执行

```bash
curl -s -X POST http://localhost:3000/api/v1/executions \
  -H "Content-Type: application/json" \
  -d "{\"workflow_id\":\"$WF_ID\",\"input\":{\"working_dir\":\"/tmp/todoflow\"}}"
```

**记录**：`EX_ID=<返回的 id>`

### 4.2 监控执行进度（每 10 秒轮询一次）

```bash
watch -n 10 "curl -s http://localhost:3000/api/v1/executions/$EX_ID | \
  python3 -c \"import sys,json; d=json.load(sys.stdin)['data']; \
  print('status:', d['status'], '| stages:', len(d.get('stage_results',[])), 'done')\""
```

**等待直到 status = completed 或 failed**

### 4.3 查看执行日志

```bash
curl -s http://localhost:3000/api/v1/executions/$EX_ID/logs 2>/dev/null | \
  python3 -m json.tool | head -50
```

---

## ACT 5 · 产物收集

### 5.1 查看产物列表

```bash
curl -s http://localhost:3000/api/v1/executions/$EX_ID | \
  python3 -c "import sys,json; d=json.load(sys.stdin)['data']; \
  [print(s['stage_name'], ':', s.get('output','')[:100]) for s in d.get('stage_results',[])]"
```

### 5.2 从产物中提取代码，写入工作区

将 AI 生成的 `todo.py` 内容写入 `/tmp/todoflow/todo.py`：

```bash
# 从执行产物中获取 implement 阶段的输出
curl -s http://localhost:3000/api/v1/executions/$EX_ID | \
  python3 -c "
import sys, json, re
d = json.load(sys.stdin)['data']
for s in d.get('stage_results', []):
    if s['stage_name'] == 'implement':
        output = str(s.get('output', ''))
        # 提取代码块
        match = re.search(r'\`\`\`python\n(.*?)\`\`\`', output, re.DOTALL)
        if match:
            with open('/tmp/todoflow/todo.py', 'w') as f:
                f.write(match.group(1))
            print('Written todo.py')
        else:
            print('No code block found, raw output:')
            print(output[:500])
"
```

---

## ACT 6 · 验收测试

### 6.1 运行生成的代码

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
[1] 买牛奶
[2] 写代码
Done: [1] 买牛奶
[2] 写代码
(empty or no items)
```

### 6.2 运行生成的测试

```bash
cd /tmp/todoflow
pip install pytest -q
python3 -m pytest test_todo.py -v 2>/dev/null || echo "测试文件未生成或测试失败"
```

---

## ACT 7 · Git 提交（通过系统）

### 7.1 查看 Git Diff

```bash
curl -s http://localhost:3000/api/v1/workspaces/$WS_ID/diffs
```

**预期**：`todo.py` 和 `test_todo.py` 出现在 diff 列表

### 7.2 通过工作区保存文件（验证文件写入 API）

```bash
# 读取生成的文件内容
CONTENT=$(cat /tmp/todoflow/todo.py | python3 -c "import sys,json; print(json.dumps(sys.stdin.read()))")

curl -s -X PUT "http://localhost:3000/api/v1/workspaces/$WS_ID/file?path=todo.py" \
  -H "Content-Type: application/json" \
  -d "{\"content\": $CONTENT}"
```

---

## ACT 8 · 知识沉淀

### 8.1 将本次经验写入 Wisdom

```bash
curl -s -X POST http://localhost:3000/api/v1/wisdom \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Python CLI 工具开发模式",
    "content": "单文件 + argparse + SQLite 是最简 Python CLI 工具的标准模式，无需第三方依赖，适合快速原型。",
    "category": "engineering"
  }'
```

---

## 验收结果表

| 步骤 | 功能模块 | 预期 | 实际 | 状态 |
|------|---------|------|------|------|
| 1.1 | 工作区创建 | WS_ID 非空 | | ⬜ |
| 1.2 | 项目创建 | PJ_ID 非空 | | ⬜ |
| 1.3 | 知识库上传 | 分块数 > 0 | | ⬜ |
| 2.1 | 任务分解 | 4 个任务创建成功 | | ⬜ |
| 3.1 | Workflow 创建 | WF_ID 非空 | | ⬜ |
| 4.1 | 执行启动 | EX_ID 非空，status=running | | ⬜ |
| 4.2 | 执行完成 | status=completed | | ⬜ |
| 5.1 | 产物收集 | 4 个 stage 有输出 | | ⬜ |
| 5.2 | 代码提取 | todo.py 写入成功 | | ⬜ |
| 6.1 | 代码可运行 | `python3 todo.py add` 成功 | | ⬜ |
| 6.2 | 测试通过 | pytest 全绿 | | ⬜ |
| 7.1 | Git Diff 可见 | todo.py 在 diff 列表 | | ⬜ |
| 8.1 | 知识沉淀 | Wisdom 创建成功 | | ⬜ |

**通过标准**：6.1（代码可运行）必须通过，其余为加分项。

---

## 失败处理

| 失败点 | 原因 | 处理方式 |
|--------|------|---------|
| 执行卡在 running | CLI 工具未安装 | `which claude` 检查，安装后重试 |
| stage_results 为空 | dispatcher 未触发 | 检查 `execution_service.rs` 日志 |
| todo.py 无代码块 | AI 输出格式不对 | 手动从 stage output 提取 |
| pytest 失败 | 生成的测试有 bug | 记录为"测试生成质量问题" |
