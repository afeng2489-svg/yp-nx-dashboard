# NexusFlow — AI 软件工厂 开发进度

> **AI 恢复指令**：切换模型后先读此文件 + `docs/progress.json`，找到 `current_sprint`，读对应 `docs/sprints/*.yaml`，从第一个 `pending` 任务继续执行。

---

## 产品定位（必读，理解为什么这样做）

**目标用户**：一个普通开发者，不需要团队，一个人用这个软件就能完成一个完整软件产品的设计、开发、测试、监控。

**核心价值**：
- 你描述需求，AI 生产线自动分解 → 并行执行 → 自动测试 → 自动修复 → 交付可运行代码
- 每个步骤可见、可控、可回滚，不是黑盒
- 中断后能从断点继续，不从零开始
- 一个人 = 一个 AI 驱动的软件工厂

**单机优先**：当前阶段只考虑单用户单机，不考虑多租户/团队协作。

---

## 快速恢复 SOP（AI 必须遵守）

```
1. 读此文件 → 找 current_sprint
2. 读 docs/progress.json → 找 status=in_progress 的 task
3. 读 docs/sprints/{sprint_id}-*.yaml → 找第一个 pending 的 remaining_task
4. 执行该任务
5. 完成后更新 progress.json + 此文件
6. 继续下一个任务
```

---

## 当前状态

| 字段 | 值 |
|------|-----|
| **当前 Sprint** | v4.7 — AI 项目追踪自动化 |
| **Sprint 状态** | `completed` |
| **上次工作** | v4.7 完成，Phase 0~4 全部完成 |
| **下一步** | 所有规划 Sprint 已完成 |
| **最后更新** | 2026-05-04 |

---

## 总体进度

```
Phase 0: 地基（v0.0.x）          ████████████████████  100%
  v0.0.1 修复关键 Bug             ✅ done
  v0.0.2 部署+认证+API规范        ✅ done
  v0.0.3 接入 core/orchestrator  ✅ done

Phase 1: 生产线核心（v1.x）       ████████████████████  100%
  v1.1 Pipeline 真正跑通          ✅ done
  v1.2 断点续跑                   ✅ done
  v1.3 质量门自动化               ✅ done
  v1.4 可观测性看板               ✅ done

Phase 2: 用户体验（v2.x）         ████████████████████  100%
  v2.1 团队对话体验重构           ✅ done
  v2.2 产物管理完整               ✅ done
  v2.3 项目状态感知               ✅ done
  v2.4 技能系统内置化             ✅ done

Phase 3: 功能扩展（v3.x）         ████████████████████  100%
  v3.1 Git 集成                   ✅ done
  v3.2 触发器系统                 ✅ done
  v3.3 Token/Cost 监控            ✅ done
  v3.4 RAG 知识库                 ✅ done

Phase 4: 智能化升级（v4.x）       ████████████████████  100%
  v4.1 多模型路由                 ✅ done
  v4.2 失败自愈                   ✅ done
  v4.3 低代码可视化画布           ✅ done
  v4.4 浏览器自动化验证           ✅ done
  v4.5 用户需求分解UI             ✅ done
  v4.6 多模态工具链               ✅ done
```

---

## 架构决策（已定论，不重复讨论）

### 三层架构
```
TaskPipeline（大脑）→ PTY/CLI（执行手）→ Git（审计层）
```

### 关键决策
1. **Phase 0 必须先做**：16处崩溃点不修，后面一切都不稳定
2. **Pipeline 是生产线心跳**：dispatch→执行→完成→推进，这条链不通，就不是生产线
3. **团队对话改 CLI 优先**：PTY 字节流抠文本是错误方向，CLI 输出才是干净文本
4. **单机优先**：SQLite 够用，不迁 PostgreSQL
5. **core/orchestrator 已有调度器**：v0.0.3 是接线，不是新建（6h 不是 25h）
6. **PTY 终端保留**：用于"看执行过程"，不用于"提取对话文本"

### 已知问题清单（按优先级）
| # | 问题 | 文件 | 优先级 |
|---|------|------|--------|
| 1 | 16处 `.expect()` 启动崩溃 | `routes/mod.rs` | P0 |
| 2 | `std::sync::Mutex` 混用 | `execution/artifact/issue_repository.rs` | P0 |
| 3 | DateTime 静默 fallback | `models/*.rs` | P0 |
| 4 | Pipeline dispatch 完成后不回调 `on_step_completed` | `routes/pipelines.rs` | P0 |
| 5 | Checkpoint 表存在但从未写入 | `pty_task_watcher.rs` | P0 |
| 6 | 团队对话 PTY 优先（输出质量差） | `routes/teams.rs` | P1 |
| 7 | A2UI 路由未挂载 | `routes/mod.rs` | P1 |
| 8 | PTY 执行不注册进程监控 | `routes/teams.rs` | P1 |
| 9 | 确认机制只支持一次（`take()`） | `agent_team_service.rs` | P1 |
| 10 | 嵌套 tokio runtime | `claude_terminal.rs:197` | P1 |
| 11 | 多任务并发写同一项目无互斥 | 全局 | P2 |
| 12 | `println!` 未替换为 tracing | 全局 | P2 |

---

## 里程碑

| 日期 | 事件 |
|------|------|
| 2026-04-29 | v1.S1 后端完成（artifact_tracker/repository/watcher/routes） |
| 2026-04-30 | 全代码库架构审查，发现 12 类问题 |
| 2026-05-01 | 全局深度分析完成，最终计划制定，文档更新 |
| 2026-05-02 | v0.0.2 完成（A2UI路由+统一API+Docker+Migration Runner） |
| 2026-05-02 | v0.0.3 完成（接入core/orchestrator，删除旧scheduler） |

---

## 执行命令

```bash
# 验证后端
cargo build && cargo test --bin nx_api && cargo clippy -- -D warnings

# 验证前端
cd nx_dashboard && npx tsc --noEmit

# 提交
git add -A && git commit -m "feat(vX.X.X): {描述}"
```
