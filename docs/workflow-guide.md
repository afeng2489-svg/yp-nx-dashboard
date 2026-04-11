# NexusFlow 工作流系统使用指南

---

## 第一部分：系统概述

### 1.1 什么是 NexusFlow？

NexusFlow 是一个**全栈多智能体开发框架**，提供：

- **后端 API**: Rust (Tokio + Axum) 高性能 Web 服务
- **工作流引擎**: 可视化编排 AI 智能体任务
- **执行服务**: 实时跟踪任务执行状态
- **会话管理**: 支持断点续传和状态恢复
- **插件系统**: 灵活扩展功能

### 1.2 系统架构

```
┌─────────────────────────────────────────────────────────┐
│                     前端 (React)                        │
│           nx_dashboard / nx_a2ui                       │
└─────────────────────┬───────────────────────────────────┘
                      │ HTTP/WebSocket
┌─────────────────────▼───────────────────────────────────┐
│                  nx_api (Rust Axum)                     │
│  ┌──────────┬──────────┬──────────┬──────────┬─────────┐ │
│  │ Workflow │Execution │ Session  │Workspace │ Plugins │ │
│  │ Service  │ Service  │ Service  │ Service  │Service  │ │
│  └────┬─────┴────┬─────┴────┬─────┴────┬─────┴────┬────┘ │
│       └──────────┴──────────┴──────────┴──────────┘       │
│                         │                                │
│  ┌──────────────────────▼──────────────────────────────┐ │
│  │              SQLite 持久化层                         │ │
│  └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

---

## 第二部分：API 接口详解

### 2.1 工作流管理

#### 创建工作流
```http
POST /api/v1/workflows
Content-Type: application/json

{
  "name": "我的第一个工作流",
  "version": "1.0.0",
  "description": "示例工作流",
  "definition": {
    "stages": [
      {
        "name": "阶段1",
        "agents": [
          {
            "name": "agent-1",
            "model": "gpt-4",
            "prompt": "你是一个助手"
          }
        ]
      }
    ]
  }
}
```

#### 列出所有工作流
```http
GET /api/v1/workflows
```

#### 获取工作流详情
```http
GET /api/v1/workflows/{id}
```

#### 更新工作流
```http
PUT /api/v1/workflows/{id}
Content-Type: application/json

{
  "name": "更新后的名称",
  "definition": { ... }
}
```

#### 删除工作流
```http
DELETE /api/v1/workflows/{id}
```

### 2.2 执行管理

#### 启动执行
```http
POST /api/v1/executions/start
Content-Type: application/json

{
  "workflow_id": "工作流ID",
  "variables": {
    "input": "用户输入"
  }
}
```

**响应示例:**
```json
{
  "id": "exec-uuid-xxx",
  "workflow_id": "workflow-xxx",
  "status": "running",
  "variables": { "input": "用户输入" },
  "stage_results": [],
  "started_at": "2026-04-06T12:00:00Z"
}
```

#### 查询执行状态
```http
GET /api/v1/executions/{id}
```

**响应示例:**
```json
{
  "id": "exec-uuid-xxx",
  "workflow_id": "workflow-xxx",
  "status": "completed",
  "variables": { "input": "用户输入" },
  "stage_results": [
    {
      "stage_name": "阶段1",
      "outputs": ["输出内容"],
      "completed_at": "2026-04-06T12:05:00Z"
    }
  ],
  "started_at": "2026-04-06T12:00:00Z",
  "finished_at": "2026-04-06T12:05:00Z"
}
```

#### 取消执行
```http
POST /api/v1/executions/{id}/cancel
```

#### 列出所有执行
```http
GET /api/v1/executions
```

### 2.3 WebSocket 实时事件

连接执行事件流：
```http
WS /ws/executions/{id}
```

**服务端事件示例:**
```json
{
  "type": "Started",
  "execution_id": "exec-xxx",
  "workflow_id": "workflow-xxx",
  "timestamp": "2026-04-06T12:00:00Z"
}
```

```json
{
  "type": "StageCompleted",
  "execution_id": "exec-xxx",
  "stage_name": "阶段1",
  "outputs": ["结果"],
  "timestamp": "2026-04-06T12:05:00Z"
}
```

```json
{
  "type": "Completed",
  "execution_id": "exec-xxx",
  "final_state": { "result": "成功" },
  "timestamp": "2026-04-06T12:10:00Z"
}
```

### 2.4 会话管理

#### 创建会话
```http
POST /api/v1/sessions
Content-Type: application/json

{
  "workflow_id": "工作流ID"
}
```

#### 列出所有会话
```http
GET /api/v1/sessions
```

#### 获取会话详情
```http
GET /api/v1/sessions/{id}
```

#### 删除会话
```http
DELETE /api/v1/sessions/{id}
```

### 2.5 工作区管理

#### 创建工作区
```http
POST /api/v1/workspaces
Content-Type: application/json

{
  "name": "我的工作区",
  "owner_id": "user-123",
  "description": "工作区描述"
}
```

#### 列出所有工作区
```http
GET /api/v1/workspaces
```

### 2.6 测试生成

#### 生成测试代码
```http
POST /api/v1/test-gen
Content-Type: application/json

{
  "source_code": "fn add(a: i32, b: i32) -> i32 { a + b }",
  "language": "rust",
  "framework": "rust",
  "file_path": "src/math.rs"
}
```

**响应示例:**
```json
{
  "test_code": "#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn test_add() {\n        assert_eq!(add(2, 3), 5);\n    }\n}",
  "language": "rust",
  "framework": "rust",
  "test_count": 1,
  "warnings": []
}
```

### 2.7 插件管理

#### 列出已加载插件
```http
GET /api/v1/plugins
```

#### 获取插件信息
```http
GET /api/v1/plugins/{id}
```

#### 获取注册表状态
```http
GET /api/v1/plugins/registry
```

---

## 第三部分：工作流定义格式

### 3.1 基本结构

```yaml
name: 工作流名称
version: 1.0.0
description: 工作流描述

stages:
  - name: 阶段名称
    agents:
      - name: 智能体名称
        model: gpt-4
        prompt: 智能体提示词
        skills:
          - skill-name
```

### 3.2 完整示例

```yaml
name: 代码审查工作流
version: 1.0.0
description: 自动审查代码质量

stages:
  - name: 代码分析
    agents:
      - name: analyzer
        model: gpt-4
        prompt: |
          你是一个代码审查专家。
          请分析以下代码并提供改进建议。

  - name: 生成报告
    agents:
      - name: reporter
        model: gpt-4
        prompt: |
          根据分析结果生成审查报告。
```

---

## 第四部分：快速开始

### 4.1 启动服务器

```bash
# 进入项目目录
cd /Users/Zhuanz/Desktop/yp-nx-dashboard

# 启动 API 服务器
cargo run -p nx_api
```

服务器默认运行在 `http://127.0.0.1:8080`

### 4.2 创建第一个工作流

```bash
# 使用 curl 创建工作流
curl -X POST http://localhost:8080/api/v1/workflows \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Hello World",
    "definition": {
      "stages": [
        {
          "name": "greet",
          "agents": [
            {
              "name": "greeter",
              "model": "gpt-4",
              "prompt": "输出 Hello World"
            }
          ]
        }
      ]
    }
  }'
```

### 4.3 启动执行

```bash
# 启动工作流执行
curl -X POST http://localhost:8080/api/v1/executions/start \
  -H "Content-Type: application/json" \
  -d '{
    "workflow_id": "<工作流ID>",
    "variables": {}
  }'
```

### 4.4 查看执行状态

```bash
# 查看执行状态
curl http://localhost:8080/api/v1/executions/<执行ID>
```

---

## 第五部分：错误处理

### 5.1 常见错误码

| 状态码 | 含义 | 说明 |
|--------|------|------|
| 200 | 成功 | 请求成功 |
| 400 | 错误请求 | 请求参数有误 |
| 404 | 未找到 | 资源不存在 |
| 500 | 服务器错误 | 内部错误 |

### 5.2 错误响应格式

```json
{
  "error": "工作流不存在: abc-123"
}
```

---

## 第六部分：使用示例

### 6.1 Python 调用示例

```python
import requests

BASE_URL = "http://localhost:8080"

# 创建工作流
workflow = requests.post(f"{BASE_URL}/api/v1/workflows", json={
    "name": "Test Workflow",
    "definition": {"stages": []}
}).json()

# 启动执行
execution = requests.post(f"{BASE_URL}/api/v1/executions/start", json={
    "workflow_id": workflow["id"],
    "variables": {}
}).json()

# 轮询执行状态
import time
while execution["status"] == "running":
    time.sleep(1)
    execution = requests.get(
        f"{BASE_URL}/api/v1/executions/{execution['id']}"
    ).json()

print(f"执行完成: {execution['status']}")
```

### 6.2 JavaScript 调用示例

```javascript
const BASE_URL = "http://localhost:8080";

// 创建工作流
const workflow = await fetch(`${BASE_URL}/api/v1/workflows`, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ name: "Test", definition: { stages: [] } })
}).then(r => r.json());

// 启动执行
const execution = await fetch(`${BASE_URL}/api/v1/executions/start`, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ workflow_id: workflow.id, variables: {} })
}).then(r => r.json());

console.log(`执行ID: ${execution.id}`);
```

### 6.3 WebSocket 实时监控

```javascript
const ws = new WebSocket("ws://localhost:8080/ws/executions/exec-xxx");

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log("收到事件:", data.type);

  switch (data.type) {
    case "Started":
      console.log("执行开始");
      break;
    case "StageCompleted":
      console.log(`阶段 ${data.stage_name} 完成`);
      break;
    case "Completed":
      console.log("执行完成!");
      ws.close();
      break;
    case "Failed":
      console.error("执行失败:", data.error);
      ws.close();
      break;
  }
};
```

---

## 附录

### A. 环境变量

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| NEXUS_API_HOST | 127.0.0.1 | 监听地址 |
| NEXUS_API_PORT | 8080 | 监听端口 |
| NEXUS_API_KEY | - | API 密钥（可选） |
| NEXUS_DB_PATH | nexus.db | 数据库路径 |

### B. 完整 API 列表

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | /health | 健康检查 |
| GET | /api/v1/workflows | 列出工作流 |
| POST | /api/v1/workflows | 创建工作流 |
| GET | /api/v1/workflows/:id | 获取工作流 |
| PUT | /api/v1/workflows/:id | 更新工作流 |
| DELETE | /api/v1/workflows/:id | 删除工作流 |
| POST | /api/v1/workflows/:id/execute | 执行工作流 |
| GET | /api/v1/executions | 列出执行 |
| GET | /api/v1/executions/:id | 获取执行 |
| POST | /api/v1/executions/:id/cancel | 取消执行 |
| POST | /api/v1/executions/start | 启动执行 |
| GET | /api/v1/sessions | 列出会话 |
| POST | /api/v1/sessions | 创建会话 |
| GET | /api/v1/sessions/:id | 获取会话 |
| DELETE | /api/v1/sessions/:id | 删除会话 |
| GET | /api/v1/workspaces | 列出工作区 |
| POST | /api/v1/workspaces | 创建工作区 |
| GET | /api/v1/workspaces/:id | 获取工作区 |
| PUT | /api/v1/workspaces/:id | 更新工作区 |
| DELETE | /api/v1/workspaces/:id | 删除工作区 |
| POST | /api/v1/test-gen | 生成测试 |
| GET | /api/v1/plugins | 列出插件 |
| GET | /api/v1/plugins/:id | 获取插件 |
| WS | /ws/executions/:id | 执行事件流 |
| WS | /ws/terminal | 终端连接 |

---

*文档版本: 1.0.0*
*最后更新: 2026-04-06*
