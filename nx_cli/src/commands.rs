//! CLI 命令实现
//!
//! 提供 /ccw, /ccw-coordinator, /workflow:session:*, /issue/* 等命令。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::Config;

/// /ccw - 自动工作流编排命令
pub async fn run_ccw(
    workflow_name: Option<String>,
    auto_mode: bool,
    parallel: bool,
    max_agents: Option<usize>,
    config: &Config,
) -> anyhow::Result<()> {
    tracing::info!("启动 CCW 自动工作流编排器...");

    if auto_mode {
        println!("🔄 CCW 自动模式已启用");
        println!("   - 自动检测工作流依赖");
        println!("   - 自动规划执行顺序");
        println!("   - 自动分配智能体资源");
    }

    if parallel {
        println!("📋 并行执行模式");
    }

    if let Some(name) = workflow_name {
        println!("🎯 指定工作流: {}", name);
    }

    if let Some(max) = max_agents {
        println!("🤖 最大并发智能体数: {}", max);
    }

    // 列出可用工作流
    println!("\n📁 可用工作流:");
    println!("   - code_review (代码审查)");
    println!("   - test_generation (测试生成)");
    println!("   - documentation (文档生成)");
    println!("   - refactoring (代码重构)");
    println!("   - bug_fixing (Bug 修复)");

    // 显示 CCW 帮助
    println!("\n📖 CCW 子命令:");
    println!("   /ccw list           - 列出所有可用工作流");
    println!("   /ccw run <name>     - 运行指定工作流");
    println!("   /ccw status         - 显示当前状态");
    println!("   /ccw stop           - 停止当前工作流");

    Ok(())
}

/// /ccw-coordinator - 智能编排协调器
pub async fn run_ccw_coordinator(
    project_path: Option<PathBuf>,
    strategy: Option<String>,
    config: &Config,
) -> anyhow::Result<()> {
    tracing::info!("启动 CCW 智能协调器...");

    println!("🔧 CCW 智能协调器");
    println!("==================\n");

    // 策略选项
    let strategy_name = strategy.unwrap_or_else(|| "auto".to_string());
    println!("📊 编排策略: {}", strategy_name);

    if let Some(path) = project_path {
        println!("📂 项目路径: {:?}", path);
    }

    println!("\n🎯 支持的编排策略:");
    println!("   - auto        (自动选择最佳策略)");
    println!("   - sequential  (顺序执行)");
    println!("   - parallel    (完全并行)");
    println!("   - pipeline    (流水线)");
    println!("   - dependent   (依赖驱动)");

    println!("\n🔄 协调流程:");
    println!("   1. 分析项目结构");
    println!("   2. 识别任务依赖");
    println!("   3. 规划执行策略");
    println!("   4. 分配智能体");
    println!("   5. 监控执行进度");
    println!("   6. 汇总执行结果");

    Ok(())
}

/// /workflow:session:* 会话管理命令
pub mod session_commands {
    use super::*;

    /// 列出所有会话
    pub async fn list_sessions(config: &Config) -> anyhow::Result<()> {
        println!("📋 工作流会话列表");
        println!("=================\n");

        println!("ID                                   状态          创建时间");
        println!("--------------------------------------------------------------------");

        // 模拟会话数据
        println!("abc123-def456-ghi789                 🟢 运行中     2026-04-03 10:30");
        println!("def456-ghi789-abc123                 🟡 暂停       2026-04-03 09:15");
        println!("ghi789-abc123-def456                 🟢 运行中     2026-04-03 08:00");
        println!("ijk123-bcd234-efg567                 ⚪ 已完成     2026-04-02 16:45");

        println!("\n共 4 个会话, 2 个运行中");

        Ok(())
    }

    /// 获取会话详情
    pub async fn get_session(session_id: &str, config: &Config) -> anyhow::Result<()> {
        println!("📋 会话详情: {}", session_id);
        println!("==================\n");

        println!("会话 ID: {}", session_id);
        println!("状态: 🟢 运行中");
        println!("创建时间: 2026-04-03 10:30:00");
        println!("当前阶段: 3/5 (代码生成)");
        println!("已用时间: 45 分钟");

        println!("\n📊 阶段进度:");
        println!("   1. ✅ 需求分析 (5 分钟)");
        println!("   2. ✅ 架构设计 (10 分钟)");
        println!("   3. 🔄 代码生成 (25 分钟, 70%)");
        println!("   4. ⏳ 测试生成 (待开始)");
        println!("   5. ⏳ 文档生成 (待开始)");

        println!("\n🤖 活跃智能体:");
        println!("   - coder-1     (生成主模块)");
        println!("   - tester-1    (生成单元测试)");

        Ok(())
    }

    /// 删除会话
    pub async fn delete_session(session_id: &str, config: &Config) -> anyhow::Result<()> {
        println!("🗑️  删除会话: {}", session_id);
        println!("警告: 此操作不可恢复!");

        // 模拟确认
        println!("会话 {} 已删除", session_id);

        Ok(())
    }

    /// 导出会话
    pub async fn export_session(
        session_id: &str,
        format: Option<&str>,
        output: Option<&str>,
        config: &Config,
    ) -> anyhow::Result<()> {
        let fmt = format.unwrap_or("json");
        let out = output.unwrap_or("session_export");

        println!("📤 导出会话: {}", session_id);
        println!("格式: {}", fmt);
        println!("输出: {}", out);

        // 模拟导出
        match fmt {
            "json" => {
                let json = serde_json::json!({
                    "session_id": session_id,
                    "status": "completed",
                    "stages": []
                });
                println!("\n{}", serde_json::to_string_pretty(&json)?);
            }
            "markdown" => {
                println!("\n# 会话报告\n\n会话 ID: {}\n\n## 摘要\n\n...", session_id);
            }
            _ => {
                println!("不支持的格式: {}", fmt);
            }
        }

        Ok(())
    }

    /// 暂停会话
    pub async fn pause_session(session_id: &str, config: &Config) -> anyhow::Result<()> {
        println!("⏸️  暂停会话: {}", session_id);
        println!("会话 {} 已暂停", session_id);
        Ok(())
    }

    /// 恢复会话
    pub async fn resume_session(session_id: &str, config: &Config) -> anyhow::Result<()> {
        println!("▶️  恢复会话: {}", session_id);
        println!("会话 {} 正在恢复...", session_id);
        Ok(())
    }
}

/// /issue/* 问题追踪命令
pub mod issue_commands {
    use super::*;

    /// 问题状态
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Issue {
        pub id: String,
        pub title: String,
        pub status: IssueStatus,
        pub priority: Priority,
        pub assignee: Option<String>,
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub enum IssueStatus {
        Open,
        InProgress,
        Closed,
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub enum Priority {
        Low,
        Medium,
        High,
        Critical,
    }

    /// 列出所有问题
    pub async fn list_issues(status_filter: Option<&str>, config: &Config) -> anyhow::Result<()> {
        println!("📋 问题列表");
        println!("===========\n");

        println!("ID       标题                    状态        优先级  指派");
        println!("--------------------------------------------------------------------");

        // 模拟问题数据
        println!("ISS-001  登录页面加载慢          🔴 进行中   高      @alice");
        println!("ISS-002  搜索结果不准确          🟡 待办     中      @bob");
        println!("ISS-003  移动端样式错乱          🟡 待办     高      @charlie");
        println!("ISS-004  API 超时问题            🟢 已关闭   低      -");
        println!("ISS-005  内存泄漏                🔴 进行中   严重    @david");

        println!("\n共 5 个问题, 2 个进行中, 2 个待办, 1 个已关闭");

        Ok(())
    }

    /// 获取问题详情
    pub async fn get_issue(issue_id: &str, config: &Config) -> anyhow::Result<()> {
        println!("📋 问题详情: {}", issue_id);
        println!("==============\n");

        println!("ID: {}", issue_id);
        println!("标题: 登录页面加载慢");
        println!("状态: 🔴 进行中");
        println!("优先级: 高");
        println!("指派: @alice");
        println!("创建时间: 2026-04-01 10:00");
        println!("更新时间: 2026-04-03 14:30");

        println!("\n📝 描述:");
        println!("用户反馈登录页面加载时间过长,平均需要 5-8 秒。");
        println!("初步分析发现是资源文件过大导致。");

        println!("\n💬 评论:");
        println!("  - @bob: 我来检查图片压缩情况");
        println!("  - @alice: 已优化,准备部署测试");

        println!("\n📎 附件:");
        println!("  - performance_report.pdf");
        println!("  - screenshot.png");

        Ok(())
    }

    /// 创建问题
    pub async fn create_issue(
        title: &str,
        description: Option<&str>,
        priority: Option<&str>,
        assignee: Option<&str>,
        config: &Config,
    ) -> anyhow::Result<()> {
        let issue_id = format!("ISS-{:03}", 100);

        println!("✅ 问题已创建: {}", issue_id);
        println!("标题: {}", title);

        if let Some(desc) = description {
            println!("描述: {}", desc);
        }

        if let Some(p) = priority {
            println!("优先级: {}", p);
        }

        if let Some(a) = assignee {
            println!("指派: @{}", a);
        }

        Ok(())
    }

    /// 更新问题状态
    pub async fn update_issue_status(
        issue_id: &str,
        status: &str,
        config: &Config,
    ) -> anyhow::Result<()> {
        println!("🔄 更新问题状态: {}", issue_id);
        println!("新状态: {}", status);
        println!("问题 {} 已更新", issue_id);
        Ok(())
    }

    /// 添加评论
    pub async fn add_comment(issue_id: &str, comment: &str, config: &Config) -> anyhow::Result<()> {
        println!("💬 添加评论到 {}:", issue_id);
        println!("{}", comment);
        println!("\n评论已添加");
        Ok(())
    }

    /// 搜索问题
    pub async fn search_issues(query: &str, config: &Config) -> anyhow::Result<()> {
        println!("🔍 搜索问题: {}", query);
        println!("=============\n");

        // 模拟搜索结果
        println!("找到 2 个相关问题:\n");
        println!("ISS-001  登录页面加载慢          🔴 进行中   高      @alice");
        println!("ISS-003  移动端样式错乱          🟡 待办     高      @charlie");

        Ok(())
    }
}

/// 从 YAML 文件运行工作流
pub async fn run_workflow(
    workflow_path: &PathBuf,
    vars: Option<&str>,
    _background: bool,
    _config: &Config,
) -> anyhow::Result<()> {
    use nexus_workflow::events::InMemoryEventEmitter;
    use nexus_workflow::{WorkflowEngine, WorkflowParser};

    tracing::info!("从 {:?} 加载工作流", workflow_path);

    // 解析工作流
    let workflow = WorkflowParser::parse_file(workflow_path)?;
    tracing::info!("已解析工作流: {}", workflow.name);

    // 验证
    WorkflowParser::validate(&workflow)?;
    tracing::info!("工作流验证通过");

    // 设置事件发射器
    let event_emitter = std::sync::Arc::new(InMemoryEventEmitter::new());

    // 创建引擎（使用 Claude CLI，不依赖 AI 提供商注册表）
    let engine = WorkflowEngine::new(event_emitter);

    // 解析变量（如果提供）
    let mut workflow_vars: HashMap<String, serde_json::Value> = HashMap::new();
    if let Some(vars_json) = vars {
        if let Ok(parsed) = serde_json::from_str(vars_json) {
            workflow_vars = parsed;
        } else {
            tracing::warn!("解析变量 JSON 失败，将忽略");
        }
    }

    tracing::info!("开始执行工作流...");

    // 执行工作流
    let result = engine.execute(&workflow).await?;

    tracing::info!("工作流完成，状态: {:?}", result.status);
    tracing::info!("执行 ID: {}", result.execution_id);

    // 打印最终状态
    if !result.variables.is_empty() {
        println!("\n最终变量:");
        for (key, value) in &result.variables {
            println!("  {}: {}", key, value);
        }
    }

    // 打印阶段结果
    println!("\n阶段结果:");
    for stage_result in &result.stage_results {
        println!(
            "  - {}: {} 个输出",
            stage_result.stage_name,
            stage_result.outputs.len()
        );
    }

    Ok(())
}

/// 运行单个智能体
pub async fn run_agent(
    _role: &str,
    _model: &str,
    prompt: &str,
    system: Option<&str>,
    _config: &Config,
) -> anyhow::Result<()> {
    // Auto-yes prefix to skip confirmation prompts
    let auto_yes_prefix = "You are operating in auto-yes mode. If you ask any question requiring confirmation (yes/no, continue?, proceed?, etc.), always assume the answer is YES and proceed automatically. Never ask for confirmation.";

    // 构建 prompt（Claude CLI 格式）
    let system_prompt = system
        .map(|s| s.to_string())
        .unwrap_or_else(|| "你扮演角色并遵循指示。".to_string());
    let full_prompt = format!(
        "{}\n\n<system>\n{}\n</system>\n\n<user>\n{}\n</user>",
        auto_yes_prefix, system_prompt, prompt
    );

    tracing::info!("正在执行智能体...",);

    // 通过 Claude CLI 执行（Claude Switch 切换后自动使用新模型）
    let output = tokio::process::Command::new("claude")
        .args(["-p", &full_prompt])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Claude CLI error: {}", stderr);
    }

    let response = String::from_utf8_lossy(&output.stdout);

    println!("\n--- 响应 ---");
    println!("{}", response);
    println!("--- 响应结束 ---");

    Ok(())
}

/// 列出可用的 AI 提供商
pub async fn list_providers(detailed: bool, config: &Config) -> anyhow::Result<()> {
    println!("已配置的提供商:");
    println!("====================\n");

    for (name, provider_config) in &config.providers {
        println!("- {}:", name);
        if let Some(ref api_key) = provider_config.api_key {
            let masked = if api_key.len() > 8 {
                format!("{}...{}", &api_key[..4], &api_key[api_key.len() - 4..])
            } else {
                "***".to_string()
            };
            println!("  API 密钥: {}", masked);
        }
        if let Some(ref models) = provider_config.models {
            println!("  模型: {}", models.join(", "));
        }
        println!();
    }

    if detailed {
        println!("\n模型支持矩阵:");
        println!("=====================\n");

        // Anthropic
        if config.providers.contains_key("anthropic") {
            println!("Anthropic (Claude):");
            println!("  - claude-opus-4-5 (最强能力)");
            println!("  - claude-sonnet-4-5 (均衡)");
            println!("  - claude-haiku-4-5 (最快)");
            println!();
        }

        // OpenAI
        if config.providers.contains_key("openai") {
            println!("OpenAI (GPT):");
            println!("  - gpt-4o (最新，最强能力)");
            println!("  - gpt-4-turbo (快速，高能力)");
            println!("  - gpt-4o-mini (轻量级)");
            println!();
        }

        // Ollama
        if config.providers.contains_key("ollama") {
            println!("Ollama (本地):");
            println!("  - llama3 (通用)");
            println!("  - codellama (代码专注)");
            println!("  - mistral (均衡)");
            println!("  - qwen2 (多语言)");
            println!();
        }
    }

    Ok(())
}

/// 验证工作流文件
pub async fn validate_workflow(workflow_path: &PathBuf, show_ast: bool) -> anyhow::Result<()> {
    use nexus_workflow::WorkflowParser;

    tracing::info!("从 {:?} 加载工作流", workflow_path);

    let workflow = WorkflowParser::parse_file(workflow_path)?;

    tracing::info!("工作流: {}", workflow.name);
    tracing::info!("版本: {}", workflow.version);

    if let Some(ref desc) = workflow.description {
        tracing::info!("描述: {}", desc);
    }

    tracing::info!("\n智能体: {}", workflow.agents.len());
    for agent in &workflow.agents {
        let deps = if agent.depends_on.is_empty() {
            String::new()
        } else {
            format!(" (依赖: {})", agent.depends_on.join(", "))
        };
        println!("  - {} [{}]: {}{}", agent.id, agent.role, agent.model, deps);
    }

    tracing::info!("\n阶段: {}", workflow.stages.len());
    for (i, stage) in workflow.stages.iter().enumerate() {
        let parallel = if stage.parallel { " (并行)" } else { "" };
        println!("  {}. {}{}", i + 1, stage.name, parallel);
        for agent_id in &stage.agents {
            println!("     - {}", agent_id);
        }
    }

    // 验证
    match WorkflowParser::validate(&workflow) {
        Ok(()) => {
            println!("\n✅ 工作流验证通过!");
        }
        Err(e) => {
            println!("\n❌ 工作流验证失败: {}", e);
            std::process::exit(1);
        }
    }

    if show_ast {
        println!("\n--- AST ---");
        println!("{}", serde_yaml::to_string(&workflow)?);
    }

    Ok(())
}

/// 在沙箱中执行代码
pub async fn execute_code(
    program: &str,
    args: &[String],
    cwd: Option<&PathBuf>,
    timeout: u64,
) -> anyhow::Result<()> {
    use nexus_sandbox::{ExecuteRequest, SandboxExecutor};
    use std::collections::HashMap;

    let executor = SandboxExecutor::new();

    let mut request = ExecuteRequest {
        program: program.to_string(),
        args: args.to_vec(),
        env_vars: HashMap::new(),
        working_dir: cwd.cloned(),
        timeout_secs: timeout,
        ..Default::default()
    };

    tracing::info!("正在执行: {} {:?}", program, args);
    tracing::info!("超时: {}s", timeout);

    let response = executor.execute(request).await?;

    println!("--- 标准输出 ---");
    println!("{}", response.stdout);
    println!("--- 标准错误 ---");
    println!("{}", response.stderr);
    println!("--- 结束 ---");

    if let Some(code) = response.exit_code {
        println!("\n退出码: {}", code);
    }

    if response.timed_out {
        println!("⚠️ 执行超时");
    }

    Ok(())
}

/// 为代码建立索引以便搜索
pub async fn index_code(path: &PathBuf, stats: bool) -> anyhow::Result<()> {
    use nexus_code_intel::{CodeIndex, TreeSitterParser};

    tracing::info!("正在为 {:?} 中的代码建立索引", path);

    let index = CodeIndex::new(path.clone());
    let parser = TreeSitterParser::new();

    // 收集文件
    let files: Vec<_> = walkdir(path)?;
    tracing::info!("找到 {} 个文件需要索引", files.len());

    for file in files.iter().take(100) {
        // 目前限制数量
        if let Ok(result) = parser.parse_file(file) {
            let symbols = nexus_code_intel::SymbolExtractor::extract(&result);
            index.index_file(file.clone(), symbols);
        }
    }

    if stats {
        let s = index.stats();
        println!("索引统计:");
        println!("  文件: {}", s.file_count);
        println!("  符号: {}", s.symbol_count);
        println!("  唯一: {}", s.unique_symbols);
    } else {
        println!("已索引 {} 个文件", index.stats().file_count);
    }

    Ok(())
}

/// 搜索已索引的代码
pub async fn search_code(query: &str, limit: usize) -> anyhow::Result<()> {
    println!("正在搜索: {}", query);
    println!("(索引搜索尚未实现，请先使用 'nx index' 建立索引)");
    Ok(())
}

/// 启动 API 服务器
pub async fn start_server(port: u16, host: &str) -> anyhow::Result<()> {
    use axum::{routing::get, Json, Router};
    use std::net::SocketAddr;

    tracing::info!("在 {}:{} 启动 NexusFlow API 服务器", host, port);

    async fn health() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION"),
            "service": "nexusflow"
        }))
    }

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/workflows", get(list_workflows));

    async fn list_workflows() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "workflows": []
        }))
    }

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("服务器监听于 {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

fn walkdir(path: &PathBuf) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let extensions = ["rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "sql"];

    if path.is_file() {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if extensions.contains(&ext.to_lowercase().as_str()) {
                files.push(path.clone());
            }
        }
        return Ok(files);
    }

    fn walkdir_recursive(
        dir: &PathBuf,
        extensions: &[&str],
        results: &mut Vec<PathBuf>,
    ) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // 跳过常见目录
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !matches!(
                    dir_name,
                    "node_modules" | "target" | ".git" | "dist" | "build"
                ) {
                    walkdir_recursive(&path, extensions, results)?;
                }
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if extensions.contains(&ext.to_lowercase().as_str()) {
                    results.push(path);
                }
            }
        }
        Ok(())
    }

    walkdir_recursive(path, &extensions, &mut files)?;
    Ok(files)
}
