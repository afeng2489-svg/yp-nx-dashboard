//! NexusFlow CLI - 主入口
//!
//! 高性能多智能体开发框架 CLI。

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;
mod config;
mod slash_commands;

use commands::{list_providers, run_agent, run_workflow};
use slash_commands::CommandDispatcher;

/// NexusFlow CLI
#[derive(Parser, Debug)]
#[command(
    name = "nx",
    about = "NexusFlow - 高性能多智能体开发框架",
    version = "0.1.0",
    author = "NexusFlow Team"
)]
struct Cli {
    /// 启用详细日志
    #[arg(short, long, global = true)]
    verbose: bool,

    /// 设置日志级别
    #[arg(short, long, value_enum, default_value_t = LogLevel::info)]
    log_level: LogLevel,

    /// 配置文件路径
    #[arg(short, long, global = true, default_value = "nexus.yaml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum LogLevel {
    trace,
    debug,
    info,
    warn,
    error,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// 从 YAML 文件运行工作流
    Run {
        /// 工作流文件路径
        #[arg(short, long)]
        workflow: PathBuf,

        /// JSON 格式的输入变量
        #[arg(short, long)]
        vars: Option<String>,

        /// 后台运行
        #[arg(short, long)]
        background: bool,
    },

    /// 运行单个智能体
    Agent {
        /// 智能体角色
        #[arg(short, long)]
        role: String,

        /// 使用的模型
        #[arg(short, long, default_value = "claude-sonnet-4-5")]
        model: String,

        /// 发送的提示词
        #[arg(short, long)]
        prompt: String,

        /// 系统提示词
        #[arg(long)]
        system: Option<String>,
    },

    /// 列出可用的 AI 提供商和模型
    Providers {
        /// 显示详细的模型信息
        #[arg(short, long)]
        detailed: bool,
    },

    /// 解析并验证工作流文件
    Validate {
        /// 工作流文件路径
        #[arg(short, long)]
        workflow: PathBuf,

        /// 显示解析后的 AST
        #[arg(short, long)]
        show_ast: bool,
    },

    /// 在沙箱中执行代码
    Exec {
        /// 要执行的程序
        #[arg(short, long)]
        program: String,

        /// 参数
        #[arg(short, long)]
        args: Vec<String>,

        /// 工作目录
        #[arg(short, long)]
        cwd: Option<PathBuf>,

        /// 超时时间（秒）
        #[arg(short, long, default_value = "30")]
        timeout: u64,
    },

    /// 为代码建立索引以便搜索
    Index {
        /// 要索引的目录
        #[arg(short, long, default_value = ".")]
        path: PathBuf,

        /// 显示索引统计信息
        #[arg(short, long)]
        stats: bool,
    },

    /// 搜索已索引的代码
    Search {
        /// 搜索查询
        #[arg(short, long)]
        query: String,

        /// 限制结果数量
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// 启动 API 服务器
    Serve {
        /// 监听端口
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// 绑定主机
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,
    },

    /// CCW - 自动工作流编排器
    Ccw {
        /// 工作流名称
        #[arg(short, long)]
        workflow: Option<String>,

        /// 自动模式
        #[arg(short, long)]
        auto: bool,

        /// 并行执行
        #[arg(short, long)]
        parallel: bool,

        /// 最大并发智能体数
        #[arg(long)]
        max_agents: Option<usize>,

        /// 子命令
        #[command(subcommand)]
        subcommand: Option<CcwSubcommand>,
    },

    /// CCW-Coordinator - 智能编排协调器
    CcwCoordinator {
        /// 项目路径
        #[arg(short, long)]
        project: Option<PathBuf>,

        /// 编排策略
        #[arg(short, long)]
        strategy: Option<String>,

        /// 子命令
        #[command(subcommand)]
        subcommand: Option<CcwCoordinatorSubcommand>,
    },

    /// 工作流会话管理
    WorkflowSession {
        /// 会话 ID
        #[arg(short, long)]
        session_id: Option<String>,

        /// 子命令
        #[command(subcommand)]
        subcommand: WorkflowSessionSubcommand,
    },

    /// 问题追踪管理
    Issue {
        /// 问题 ID
        #[arg(short, long)]
        id: Option<String>,

        /// 子命令
        #[command(subcommand)]
        subcommand: IssueSubcommand,
    },

    /// Slash 命令 (自动解析 /command:subcommand args)
    Slash {
        /// 命令字符串 (例如: "issue:new title=...")
        #[arg(short, long)]
        command: String,
    },

    /// 列出所有可用命令
    CommandsList,

    /// 获取命令帮助
    Help {
        /// 命令名称 (例如: "issue:new")
        #[arg(short, long)]
        command: String,
    },
}

/// CCW 子命令
#[derive(Debug, Subcommand)]
enum CcwSubcommand {
    /// 列出所有可用工作流
    List,
    /// 运行指定工作流
    Run {
        /// 工作流名称
        name: String,
    },
    /// 显示当前状态
    Status,
    /// 停止当前工作流
    Stop,
}

/// CCW-Coordinator 子命令
#[derive(Debug, Subcommand)]
enum CcwCoordinatorSubcommand {
    /// 分析项目
    Analyze {
        /// 项目路径
        path: PathBuf,
    },
    /// 计划执行
    Plan {
        /// 工作流名称
        workflow: String,
    },
    /// 执行编排
    Execute {
        /// 工作流名称
        workflow: String,
    },
}

/// 工作流会话子命令
#[derive(Debug, Subcommand)]
enum WorkflowSessionSubcommand {
    /// 列出所有会话
    List,
    /// 获取会话详情
    Get {
        /// 会话 ID
        session_id: String,
    },
    /// 删除会话
    Delete {
        /// 会话 ID
        session_id: String,
    },
    /// 导出会话
    Export {
        /// 会话 ID
        session_id: String,
        /// 导出格式
        #[arg(long)]
        format: Option<String>,
        /// 输出路径
        #[arg(long)]
        output: Option<String>,
    },
    /// 暂停会话
    Pause {
        /// 会话 ID
        session_id: String,
    },
    /// 恢复会话
    Resume {
        /// 会话 ID
        session_id: String,
    },
}

/// 问题追踪子命令
#[derive(Debug, Subcommand)]
enum IssueSubcommand {
    /// 列出所有问题
    List {
        /// 状态过滤器
        #[arg(short, long)]
        status: Option<String>,
    },
    /// 获取问题详情
    Get {
        /// 问题 ID
        id: String,
    },
    /// 创建问题
    Create {
        /// 问题标题
        title: String,
        /// 问题描述
        #[arg(short, long)]
        description: Option<String>,
        /// 优先级
        #[arg(short, long)]
        priority: Option<String>,
        /// 指派人员
        #[arg(short, long)]
        assignee: Option<String>,
    },
    /// 更新问题状态
    UpdateStatus {
        /// 问题 ID
        id: String,
        /// 新状态
        status: String,
    },
    /// 添加评论
    Comment {
        /// 问题 ID
        id: String,
        /// 评论内容
        comment: String,
    },
    /// 搜索问题
    Search {
        /// 搜索查询
        query: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // 初始化日志
    init_logging(cli.verbose || matches!(cli.log_level, LogLevel::debug | LogLevel::trace));

    tracing::info!("NexusFlow v{}", env!("CARGO_PKG_VERSION"));

    // 加载配置
    let config = config::load_config(&cli.config)?;
    tracing::debug!("已从 {:?} 加载配置", cli.config);

    match &cli.command {
        Commands::Run {
            workflow,
            vars,
            background,
        } => {
            run_workflow(workflow, vars.as_deref(), *background, &config).await?;
        }
        Commands::Agent {
            role,
            model,
            prompt,
            system,
        } => {
            run_agent(role, model, prompt, system.as_deref(), &config).await?;
        }
        Commands::Providers { detailed } => {
            list_providers(*detailed, &config).await?;
        }
        Commands::Validate { workflow, show_ast } => {
            commands::validate_workflow(workflow, *show_ast).await?;
        }
        Commands::Exec {
            program,
            args,
            cwd,
            timeout,
        } => {
            commands::execute_code(program, args, cwd.as_ref(), *timeout).await?;
        }
        Commands::Index { path, stats } => {
            commands::index_code(path, *stats).await?;
        }
        Commands::Search { query, limit } => {
            commands::search_code(query, *limit).await?;
        }
        Commands::Serve { port, host } => {
            commands::start_server(*port, host).await?;
        }
        Commands::Ccw {
            workflow,
            auto,
            parallel,
            max_agents,
            subcommand,
        } => match subcommand {
            Some(CcwSubcommand::List) => {
                println!("📋 可用工作流:");
                println!("  - code_review");
                println!("  - test_generation");
                println!("  - documentation");
                println!("  - refactoring");
                println!("  - bug_fixing");
            }
            Some(CcwSubcommand::Run { name }) => {
                commands::run_ccw(Some(name.clone()), *auto, *parallel, *max_agents, &config)
                    .await?;
            }
            Some(CcwSubcommand::Status) => {
                println!("🔄 CCW 状态: 空闲");
            }
            Some(CcwSubcommand::Stop) => {
                println!("🛑 停止 CCW");
            }
            None => {
                commands::run_ccw(workflow.clone(), *auto, *parallel, *max_agents, &config).await?;
            }
        },
        Commands::CcwCoordinator {
            project,
            strategy,
            subcommand,
        } => match subcommand {
            Some(CcwCoordinatorSubcommand::Analyze { path }) => {
                println!("🔍 分析项目: {:?}", path);
            }
            Some(CcwCoordinatorSubcommand::Plan { workflow }) => {
                println!("📋 规划工作流: {}", workflow);
            }
            Some(CcwCoordinatorSubcommand::Execute { workflow }) => {
                println!("▶️  执行工作流: {}", workflow);
            }
            None => {
                commands::run_ccw_coordinator(project.clone(), strategy.clone(), &config).await?;
            }
        },
        Commands::WorkflowSession {
            session_id,
            subcommand,
        } => match subcommand {
            WorkflowSessionSubcommand::List => {
                commands::session_commands::list_sessions(&config).await?;
            }
            WorkflowSessionSubcommand::Get { session_id } => {
                commands::session_commands::get_session(session_id, &config).await?;
            }
            WorkflowSessionSubcommand::Delete { session_id } => {
                commands::session_commands::delete_session(session_id, &config).await?;
            }
            WorkflowSessionSubcommand::Export {
                session_id,
                format,
                output,
            } => {
                commands::session_commands::export_session(
                    session_id,
                    format.as_deref(),
                    output.as_deref(),
                    &config,
                )
                .await?;
            }
            WorkflowSessionSubcommand::Pause { session_id } => {
                commands::session_commands::pause_session(session_id, &config).await?;
            }
            WorkflowSessionSubcommand::Resume { session_id } => {
                commands::session_commands::resume_session(session_id, &config).await?;
            }
        },
        Commands::Issue { id, subcommand } => match subcommand {
            IssueSubcommand::List { status } => {
                commands::issue_commands::list_issues(status.as_deref(), &config).await?;
            }
            IssueSubcommand::Get { id } => {
                commands::issue_commands::get_issue(id, &config).await?;
            }
            IssueSubcommand::Create {
                title,
                description,
                priority,
                assignee,
            } => {
                commands::issue_commands::create_issue(
                    title,
                    description.as_deref(),
                    priority.as_deref(),
                    assignee.as_deref(),
                    &config,
                )
                .await?;
            }
            IssueSubcommand::UpdateStatus { id, status } => {
                commands::issue_commands::update_issue_status(id, status, &config).await?;
            }
            IssueSubcommand::Comment { id, comment } => {
                commands::issue_commands::add_comment(id, comment, &config).await?;
            }
            IssueSubcommand::Search { query } => {
                commands::issue_commands::search_issues(query, &config).await?;
            }
        },
        Commands::Slash { command } => {
            let dispatcher = CommandDispatcher::new();
            match dispatcher.dispatch_raw(command).await {
                Ok(result) => {
                    println!("{}", result.output.message);
                    if let Some(data) = result.output.data {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&data).unwrap_or_default()
                        );
                    }
                    tracing::info!("Command executed in {}ms", result.execution_time_ms);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::CommandsList => {
            let dispatcher = CommandDispatcher::new();
            println!("Available Slash Commands:");
            println!("========================\n");
            for cmd in dispatcher.list_all_commands() {
                println!("  /{}  - {}", cmd.command, cmd.description);
            }
            println!("\nUse /help --command=<name> for detailed help.");
        }
        Commands::Help { command } => {
            let dispatcher = CommandDispatcher::new();
            match dispatcher.get_help(&command) {
                Some(help) => {
                    println!("{}", help);
                }
                None => {
                    eprintln!("Unknown command: /{}", command);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

fn init_logging(verbose: bool) {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        if verbose {
            tracing_subscriber::EnvFilter::new("debug")
        } else {
            tracing_subscriber::EnvFilter::new("info")
        }
    });

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();
}
