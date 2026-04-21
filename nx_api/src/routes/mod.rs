//! API 路由

use axum::{
    extract::{Path, State},
    routing::{get, post, put, delete},
    Router,
};
use tower_http::cors::{CorsLayer, Any};
use std::sync::Arc;
use std::collections::HashMap;
use std::path::PathBuf;
use parking_lot::RwLock;

use crate::config::ApiConfig;
use crate::services::{WorkflowService, ExecutionService, SessionService, SqliteSessionRepository, SqliteWorkflowRepository, SqliteWorkspaceRepository, WorkspaceService, TestGenerator, PluginService, WisdomService, SharedWisdomService, SkillService, TelegramService, SqliteTeamRepository, SqliteApiKeyRepository, ProjectService, SqliteProjectRepository, AgentTeamService, SqliteProviderRepository, ProviderService, GroupChatService, SqliteGroupChatRepository, SqliteIssueRepository};
use crate::middleware::auth::ApiKeyAuth;
use crate::ws::TerminalWsHandler;
use crate::ws::ClaudeStreamWsHandler;
use crate::ws::RunCommandWsHandler;
use crate::ws::AgentExecutionManager;
use crate::routes::teams_state::TeamsAppState;
use nexus_ai::{AIModelManager, AIManagerConfig, APIConfig, ProviderType, ModelConfig};

pub mod health;
pub mod workflows;
pub mod executions;
pub mod sessions;
pub mod workspaces;
pub mod test_gen;
pub mod plugins;
pub mod templates;
pub mod scheduler;
pub mod search;
pub mod wisdom;
pub mod ai_config;
pub mod skills;
pub mod teams;
pub mod teams_state;
pub mod projects;
pub mod group_chat;
pub mod memory;
pub mod processes;
pub mod issues;

/// 查找 config/workflows 目录（YAML 种子文件）
fn resolve_workflows_dir() -> Option<PathBuf> {
    let subpath = std::path::Path::new("config").join("workflows");

    let is_workspace_root = |dir: &std::path::Path| -> bool {
        dir.join("Cargo.toml").exists() && dir.join("nx_dashboard").is_dir()
    };

    // 策略1: WORKFLOWS_DIR 环境变量
    if let Ok(dir) = std::env::var("WORKFLOWS_DIR") {
        let p = PathBuf::from(dir);
        if p.is_dir() {
            return Some(p);
        }
    }

    // 策略2: exe 祖先
    if let Ok(exe) = std::env::current_exe() {
        let exe = exe.canonicalize().unwrap_or(exe);
        for ancestor in exe.ancestors().skip(1) {
            if is_workspace_root(ancestor) {
                let p = ancestor.join(&subpath);
                if p.is_dir() {
                    return Some(p);
                }
            }
        }
    }

    // 策略3: CWD 祖先
    if let Ok(cwd) = std::env::current_dir() {
        for ancestor in cwd.ancestors() {
            if is_workspace_root(ancestor) {
                let p = ancestor.join(&subpath);
                if p.is_dir() {
                    return Some(p);
                }
            }
        }
    }

    // 策略4: 编译期路径
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    if let Some(parent) = manifest_dir.parent() {
        if is_workspace_root(parent) {
            let p = parent.join(&subpath);
            if p.is_dir() {
                return Some(p);
            }
        }
    }

    None
}

/// 将 config/workflows/**/*.yaml 种子文件 upsert 到数据库（递归扫描子目录）
/// 规则：按 name 匹配；不存在则创建，已存在则跳过（避免覆盖用户修改）
fn seed_workflows_from_yaml(workflow_service: &WorkflowService) {
    let Some(dir) = resolve_workflows_dir() else {
        tracing::debug!("[WorkflowSeeder] config/workflows 目录未找到，跳过");
        return;
    };

    tracing::info!("[WorkflowSeeder] 扫描目录: {:?}", dir);

    // 获取已有工作流名称集合
    let existing_names: std::collections::HashSet<String> = match workflow_service.list_workflows() {
        Ok(list) => list.into_iter().map(|w| w.name).collect(),
        Err(e) => {
            tracing::warn!("[WorkflowSeeder] 无法读取已有工作流: {}", e);
            return;
        }
    };

    seed_dir_recursive(workflow_service, &dir, &existing_names);
}

/// 递归扫描目录，导入所有 .yaml 工作流文件
fn seed_dir_recursive(
    workflow_service: &WorkflowService,
    dir: &std::path::Path,
    existing_names: &std::collections::HashSet<String>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        tracing::warn!("[WorkflowSeeder] 无法读取目录: {:?}", dir);
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // 递归进入子目录
        if path.is_dir() {
            seed_dir_recursive(workflow_service, &path, existing_names);
            continue;
        }

        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }

        let Ok(yaml) = std::fs::read_to_string(&path) else {
            tracing::warn!("[WorkflowSeeder] 无法读取文件: {:?}", path);
            continue;
        };

        // 解析 YAML 为 JSON
        let definition: serde_json::Value = match serde_yaml::from_str(&yaml) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("[WorkflowSeeder] YAML 解析失败 {:?}: {}", path, e);
                continue;
            }
        };

        let name = definition.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        if name.is_empty() {
            tracing::warn!("[WorkflowSeeder] 跳过无名称文件: {:?}", path);
            continue;
        }

        if existing_names.contains(&name) {
            tracing::debug!("[WorkflowSeeder] 已存在，跳过: {}", name);
            continue;
        }

        let version = definition.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0")
            .to_string();

        let description = definition.get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        match workflow_service.create_workflow(name.clone(), Some(version), description, definition) {
            Ok(w) => tracing::info!("[WorkflowSeeder] 已导入: {} (id={})", name, w.id),
            Err(e) => tracing::warn!("[WorkflowSeeder] 导入失败 {}: {}", name, e),
        }
    }
}

/// 查找 .claude/agents 目录
/// 与 DB 路径解析同策略：exe 祖先 → CWD 祖先 → 编译期路径
fn resolve_agents_dir() -> PathBuf {
    let subpath = std::path::Path::new(".claude").join("agents");

    let is_workspace_root = |dir: &std::path::Path| -> bool {
        dir.join("Cargo.toml").exists() && dir.join("nx_dashboard").is_dir()
    };

    // 策略1: exe 祖先
    if let Ok(exe) = std::env::current_exe() {
        let exe = exe.canonicalize().unwrap_or(exe);
        for ancestor in exe.ancestors().skip(1) {
            if is_workspace_root(ancestor) {
                return ancestor.join(&subpath);
            }
        }
    }

    // 策略2: CWD 祖先
    if let Ok(cwd) = std::env::current_dir() {
        for ancestor in cwd.ancestors() {
            if is_workspace_root(ancestor) {
                return ancestor.join(&subpath);
            }
        }
    }

    // 策略3: 编译期路径
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    if let Some(parent) = manifest_dir.parent() {
        if is_workspace_root(parent) {
            return parent.join(&subpath);
        }
    }

    // fallback
    std::env::current_dir()
        .unwrap_or_default()
        .join(&subpath)
}

/// 从环境变量加载 AI 配置
fn load_ai_config_from_env() -> AIManagerConfig {
    let mut api_config = HashMap::new();

    // 加载 Anthropic API 配置
    if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
        if !api_key.is_empty() {
            api_config.insert(ProviderType::Anthropic, APIConfig {
                api_key,
                base_url: String::new(),
                organization_id: String::new(),
                timeout_secs: 120,
            });
        }
    }

    // 加载 OpenAI API 配置
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        if !api_key.is_empty() {
            api_config.insert(ProviderType::OpenAI, APIConfig {
                api_key,
                base_url: String::new(),
                organization_id: String::new(),
                timeout_secs: 120,
            });
        }
    }

    // 加载 Google API 配置
    if let Ok(api_key) = std::env::var("GOOGLE_API_KEY") {
        if !api_key.is_empty() {
            api_config.insert(ProviderType::Google, APIConfig {
                api_key,
                base_url: String::new(),
                organization_id: String::new(),
                timeout_secs: 120,
            });
        }
    }

    // 加载 MiniMax API 配置
    if let Ok(api_key) = std::env::var("MINIMAX_API_KEY") {
        if !api_key.is_empty() {
            api_config.insert(ProviderType::MiniMax, APIConfig {
                api_key,
                base_url: String::new(),
                organization_id: String::new(),
                timeout_secs: 120,
            });
        }
    }

    // 加载默认模型
    let default_model = if let Ok(model_id) = std::env::var("NEXUS_DEFAULT_MODEL") {
        ModelConfig {
            model_id,
            provider: ProviderType::Anthropic,
            max_tokens: 4096,
            temperature: 0.7,
            stop_sequences: vec![],
            extra_params: HashMap::new(),
        }
    } else {
        ModelConfig::default()
    };

    AIManagerConfig {
        default_model,
        api_config,
        enabled_providers: vec![
            ProviderType::Anthropic,
            ProviderType::OpenAI,
            ProviderType::Google,
            ProviderType::Ollama,
            ProviderType::Codex,
            ProviderType::Qwen,
            ProviderType::OpenCode,
            ProviderType::MiniMax,
        ],
    }
}

/// Application state for search
use crate::routes::search::SearchState;
use crate::routes::scheduler::SchedulerState;
use crate::routes::memory::MemoryState;

/// 应用状态
pub struct AppState {
    pub workflow_service: WorkflowService,
    pub execution_service: ExecutionService,
    pub session_service: SessionService,
    pub workspace_service: WorkspaceService,
    pub test_generator: TestGenerator,
    pub plugin_service: PluginService,
    pub skill_service: SkillService,
    pub search_state: Arc<SearchState>,
    pub scheduler_state: Arc<SchedulerState>,
    pub wisdom_service: SharedWisdomService,
    pub ai_model_manager: Arc<nexus_ai::AIModelManager>,
    pub teams_state: TeamsAppState,
    pub api_key_repository: Arc<SqliteApiKeyRepository>,
    pub project_service: Arc<ProjectService>,
    pub provider_service: Arc<ProviderService>,
    pub group_chat_service: Arc<GroupChatService>,
    pub memory_state: Arc<MemoryState>,
    /// Agent 执行管理器（WebSocket 事件推送 + 取消支持）
    pub agent_execution_manager: AgentExecutionManager,
    /// 当前工作区路径，用于 Claude CLI --project 参数
    pub current_workspace_path: Arc<RwLock<Option<String>>>,
    /// API 密钥（用于认证中间件）
    pub api_key_config: Option<String>,
    /// Issue 仓储
    pub issue_repository: Arc<SqliteIssueRepository>,
}

impl AppState {
    pub fn new(config: &ApiConfig) -> Self {
        // 保存 API 密钥配置用于认证中间件
        let api_key_config = config.api_key.clone();

        // 创建当前工作区路径（用于 Claude CLI --project 参数）
        let current_workspace_path = Arc::new(RwLock::new(None));

        tracing::info!("[DB] Using database path: {}", config.db_path);

        // 创建会话仓库和服务
        let session_repo = Arc::new(
            SqliteSessionRepository::new(&config.db_path)
                .expect("Failed to create session repository")
        );
        let session_service = SessionService::new(session_repo);

        // 创建工作区仓库和服务
        let workspace_repo = Arc::new(
            SqliteWorkspaceRepository::new(&config.db_path)
                .expect("Failed to create workspace repository")
        );
        let workspace_service = WorkspaceService::new(workspace_repo);

        // 创建工作流仓库和服务
        let workflow_repo = Arc::new(
            SqliteWorkflowRepository::new(config.db_path.as_ref())
                .expect("Failed to create workflow repository")
        );
        let workflow_service = WorkflowService::with_repository(workflow_repo);

        // 启动时将 config/workflows/*.yaml 种子文件导入数据库
        seed_workflows_from_yaml(&workflow_service);

        // 创建执行服务
        let execution_service = ExecutionService::new();

        // 创建测试生成器
        let ai_registry = Arc::new(nexus_ai::AIProviderRegistry::new());
        let test_generator = TestGenerator::new(ai_registry);

        // 创建插件服务
        let plugin_service = PluginService::new();

        // 创建搜索状态
        let search_state = Arc::new(SearchState::new(current_workspace_path.clone()));

        // 创建调度器状态
        let scheduler_state = Arc::new(SchedulerState::new());
        scheduler_state.init(Some(execution_service.clone())); // Initialize scheduler with execution service

        // 创建 Wisdom 服务
        let wisdom_service = Arc::new(
            WisdomService::new(&config.db_path).expect("Failed to create wisdom service")
        );

        // 创建技能服务（文件型，直接读写 .claude/agents/*.md）
        // 优先使用 AGENTS_DIR 环境变量（通过 Tauri app 传递），否则自动查找 workspace root
        let agents_dir = std::env::var("AGENTS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| resolve_agents_dir());
        tracing::info!("[Skills] Using agents directory: {:?}", agents_dir);
        let skill_service = crate::services::SkillService::with_agents_dir(agents_dir.clone())
            .expect(&format!("Failed to create skill service with agents_dir: {:?}", agents_dir));
        let skill_service_for_agent = skill_service.clone();

        // 创建团队仓库和服务
        let team_repo = Arc::new(
            SqliteTeamRepository::new(&config.db_path)
                .expect("Failed to create team repository")
        );
        let team_service = crate::services::TeamService::new(team_repo);

        // 创建 AI 模型管理器（从环境变量加载 API 密钥）
        let ai_model_manager = Arc::new(AIModelManager::from_config(load_ai_config_from_env()));

        // 创建 AI Provider 仓库和服务
        let provider_repo = Arc::new(
            SqliteProviderRepository::new(&config.db_path)
                .expect("Failed to create provider repository")
        );
        let provider_service = Arc::new(ProviderService::new(provider_repo));

        // 创建 AgentTeamService（带有 provider_service 以便从数据库获取 API keys）
        let agent_team_service = Arc::new(
            AgentTeamService::with_provider_service(
                team_service.clone(),
                skill_service_for_agent,
                TelegramService::new(),
                ai_model_manager.clone(),
                provider_service.clone(),
                current_workspace_path.clone(),
            )
        );

        // 创建 Memory 状态（使用单独的数据库文件）- 需要在 teams_state 之前创建
        let memory_db_path = if config.db_path.contains('/') {
            // 如果是绝对路径或包含目录
            format!("{}_memory.db", config.db_path.replace(".db", ""))
        } else {
            // 相对路径：加上当前工作目录
            let cwd = std::env::current_dir().unwrap_or_default();
            format!("{}/{}_memory.db", cwd.display(), config.db_path.replace(".db", ""))
        };
        tracing::info!("[Memory] Using database path: {}", memory_db_path);
        let memory_state = Arc::new(
            crate::routes::memory::create_memory_state(&memory_db_path, None)
        );

        // 创建团队服务状态（将 memory_state 注入到 agent_team_service）
        let teams_state = TeamsAppState::new_with_agent_and_memory(
            team_service.clone(),
            TelegramService::new(),
            ai_model_manager.clone(),
            agent_team_service.clone(),
            memory_state.clone(),
        );

        // 创建项目仓库和服务
        let project_repo = Arc::new(
            SqliteProjectRepository::new(&config.db_path)
                .expect("Failed to create project repository")
        );
        let project_service = Arc::new(ProjectService::new(project_repo, Arc::new(team_service.clone()), agent_team_service.clone(), Arc::new(workspace_service.clone())));

        // 创建 API 密钥仓库
        let api_key_repo = Arc::new(
            SqliteApiKeyRepository::new(&config.db_path)
                .expect("Failed to create API key repository")
        );

        // 创建群组讨论服务
        let group_chat_repo = Arc::new(
            SqliteGroupChatRepository::new(&config.db_path)
                .expect("Failed to create group chat repository")
        );
        group_chat_repo.init_tables().expect("Failed to init group chat tables");
        let group_chat_service = Arc::new(
            GroupChatService::new(
                group_chat_repo,
                team_service.clone(),
                ai_model_manager.clone(),
                current_workspace_path.clone(),
            )
        );

        // 创建 Agent 执行管理器
        let agent_execution_manager = AgentExecutionManager::new();

        // 创建 Issue 仓储
        let issue_repository = Arc::new(
            SqliteIssueRepository::new(config.db_path.as_ref())
                .expect("Failed to create issue repository")
        );

        Self {
            workflow_service,
            execution_service,
            session_service,
            workspace_service,
            test_generator,
            plugin_service,
            skill_service,
            search_state,
            scheduler_state,
            wisdom_service,
            ai_model_manager,
            teams_state,
            api_key_repository: api_key_repo,
            project_service,
            provider_service,
            group_chat_service,
            memory_state,
            agent_execution_manager,
            current_workspace_path,
            api_key_config,
            issue_repository,
        }
    }
}

/// 创建 API 路由器
pub fn create_router(config: ApiConfig) -> (Router, Arc<AppState>) {
    let app_state = Arc::new(AppState::new(&config));
    let app_state_for_router = Arc::clone(&app_state);

    // 需要认证的 API 路由
    let api_routes = Router::new()
        // 工作流路由
        .route("/api/v1/workflows", get(workflows::list_workflows))
        .route("/api/v1/workflows", post(workflows::create_workflow))
        .route("/api/v1/workflows/:id", get(workflows::get_workflow))
        .route("/api/v1/workflows/:id", put(workflows::update_workflow))
        .route("/api/v1/workflows/:id", delete(workflows::delete_workflow))
        .route("/api/v1/workflows/:id/execute", post(workflows::execute_workflow))
        // 执行路由
        .route("/api/v1/executions", get(executions::list_executions))
        .route("/api/v1/executions/:id", get(executions::get_execution))
        .route("/api/v1/executions/:id/cancel", post(executions::cancel_execution))
        .route("/api/v1/executions/start", post(executions::start_execution))
        // 会话路由
        .route("/api/v1/sessions", get(sessions::list_sessions))
        .route("/api/v1/sessions", post(sessions::create_session))
        .route("/api/v1/sessions/:id", get(sessions::get_session))
        .route("/api/v1/sessions/:id", delete(sessions::delete_session))
        .route("/api/v1/sessions/:id/pause", post(sessions::pause_session))
        .route("/api/v1/sessions/:id/activate", post(sessions::activate_session))
        .route("/api/v1/sessions/:id/sync", post(sessions::sync_session))
        .route("/api/v1/sessions/resume/:resume_key", post(sessions::resume_session))
        // 工作区路由
        .route("/api/v1/workspaces", get(workspaces::list_workspaces))
        .route("/api/v1/workspaces", post(workspaces::create_workspace))
        .route("/api/v1/workspaces/:id", get(workspaces::get_workspace))
        .route("/api/v1/workspaces/:id", put(workspaces::update_workspace))
        .route("/api/v1/workspaces/:id", delete(workspaces::delete_workspace))
        .route("/api/v1/workspaces/:id/browse", get(workspaces::browse_workspace))
        .route("/api/v1/workspaces/:id/diffs", get(workspaces::get_git_diffs))
        .route("/api/v1/workspaces/:id/diff/*file_path", get(workspaces::get_file_diff))
        .route("/api/v1/workspaces/:id/git/status", get(workspaces::get_git_status))
        .route("/api/v1/workspaces/:id/scripts", get(workspaces::detect_scripts))
        .route("/api/v1/workspaces/:id/file", get(workspaces::read_file).put(workspaces::write_file).delete(workspaces::delete_file))
        // 测试生成路由
        .route("/api/v1/test-gen", post(test_gen::generate_tests))
        .route("/api/v1/test-gen/unit", post(test_gen::generate_unit_tests))
        .route("/api/v1/test-gen/integration", post(test_gen::generate_integration_tests))
        // 插件路由
        .route("/api/v1/plugins/registry", get(plugins::get_plugin_registry_status))
        .route("/api/v1/plugins/:id", get(plugins::get_plugin))
        .route("/api/v1/plugins", get(plugins::list_plugins))
        // 模板路由
        .route("/api/v1/templates", get(templates::list_templates))
        .route("/api/v1/templates", post(templates::create_template))
        .route("/api/v1/templates/:id", get(templates::get_template))
        .route("/api/v1/templates/:id/instantiate", post(templates::instantiate_template))
        .route("/api/v1/templates/category/:category", get(templates::list_templates_by_category))
        // 搜索路由
        .route("/api/v1/search", get(search::search).post(search::reindex))
        .route("/api/v1/search/index", post(search::reindex))
        .route("/api/v1/search/modes", get(search::get_search_modes))
        // Wisdom 路由
        .route("/api/v1/wisdom", get(wisdom::list_wisdom))
        .route("/api/v1/wisdom", post(wisdom::create_wisdom))
        .route("/api/v1/wisdom/:id", get(wisdom::get_wisdom))
        .route("/api/v1/wisdom/:id", delete(wisdom::delete_wisdom))
        .route("/api/v1/wisdom/categories", get(wisdom::list_categories))
        .route("/api/v1/wisdom/categories/:category", get(wisdom::get_by_category))
        .route("/api/v1/wisdom/search", get(wisdom::search_wisdom))
        // 任务调度路由
        .route("/api/v1/tasks", get(scheduler::list_tasks))
        .route("/api/v1/tasks", post(scheduler::create_task))
        .route("/api/v1/tasks/stats", get(scheduler::get_stats))
        .route("/api/v1/tasks/:id", get(scheduler::get_task))
        .route("/api/v1/tasks/:id", delete(scheduler::cancel_task))
        // AI 配置路由
        .route("/api/v1/ai/providers", get(ai_config::list_providers))
        .route("/api/v1/ai/clis", get(ai_config::list_clis))
        .route("/api/v1/ai/execute", post(ai_config::execute_cli))
        .route("/api/v1/ai/clis/config", put(ai_config::update_cli_config))
        .route("/api/v1/ai/strategy", put(ai_config::update_selection_strategy))
        .route("/api/v1/ai/suggestion", post(ai_config::get_selection_suggestion))
        // 模型选择路由
        .route("/api/v1/ai/models", get(ai_config::list_models))
        .route("/api/v1/ai/selected", get(ai_config::get_selected_model))
        .route("/api/v1/ai/selected", put(ai_config::set_selected_model))
        .route("/api/v1/ai/default", put(ai_config::set_default_model))
        .route("/api/v1/ai/chat", post(ai_config::chat_with_selected))
        .route("/api/v1/ai/providers/:provider/models", get(ai_config::get_provider_models))
        .route("/api/v1/ai/models/config", put(ai_config::update_model_config))
        .route("/api/v1/ai/models/refresh-status", get(ai_config::get_refresh_status))
        .route("/api/v1/ai/models/refresh", post(ai_config::refresh_models))
        // API 密钥管理
        .route("/api/v1/ai/api-keys", get(ai_config::list_api_keys))
        .route("/api/v1/ai/api-keys", post(ai_config::save_api_key))
        .route("/api/v1/ai/api-keys/:provider", delete(ai_config::delete_api_key))
        // AI Provider 管理路由
        .route("/api/v1/ai/v2/providers", get(ai_config::list_providers_v2))
        .route("/api/v1/ai/v2/providers", post(ai_config::create_provider))
        .route("/api/v1/ai/v2/providers/:id", get(ai_config::get_provider))
        .route("/api/v1/ai/v2/providers/:id", put(ai_config::update_provider))
        .route("/api/v1/ai/v2/providers/:id", delete(ai_config::delete_provider))
        .route("/api/v1/ai/v2/providers/:id/test-connection", post(ai_config::test_provider_connection))
        .route("/api/v1/ai/v2/providers/:id/enable", post(ai_config::enable_provider))
        .route("/api/v1/ai/v2/providers/:id/disable", post(ai_config::disable_provider))
        .route("/api/v1/ai/v2/providers/:id/models", get(ai_config::get_provider_mappings))
        .route("/api/v1/ai/v2/providers/:id/models", post(ai_config::add_model_mapping))
        .route("/api/v1/ai/v2/providers/:id/models/:model_id/:mapping_type", delete(ai_config::remove_model_mapping))
        .route("/api/v1/ai/v2/presets", get(ai_config::get_provider_presets))
        .route("/api/v1/ai/v2/providers/from-preset", post(ai_config::create_from_preset))
        // Claude Switch 路由
        .route("/api/v1/ai/claude-switch/configure", post(ai_config::configure_claude_switch))
        .route("/api/v1/ai/claude-switch/backends", get(ai_config::list_claude_switch_backends))
        .route("/api/v1/ai/claude-switch/backends", post(ai_config::add_claude_switch_backend))
        .route("/api/v1/ai/claude-switch/backends/switch", post(ai_config::switch_claude_switch_backend))
        .route("/api/v1/ai/claude-switch/active", get(ai_config::get_active_claude_switch_backend))
        .route("/api/v1/ai/claude-switch/backends/test", post(ai_config::test_claude_switch_backend))
        // 当前工作区路由（用于 Claude CLI --project 参数）
        .route("/api/v1/ai/current-workspace", get(ai_config::get_current_workspace))
        .route("/api/v1/ai/current-workspace", put(ai_config::set_current_workspace))
        // 技能路由
        .route("/api/v1/skills", get(skills::list_skills))
        .route("/api/v1/skills", post(skills::create_skill))
        .route("/api/v1/skills/search", get(skills::search_skills))
        .route("/api/v1/skills/categories", get(skills::list_categories))
        .route("/api/v1/skills/tags", get(skills::list_tags))
        .route("/api/v1/skills/stats", get(skills::get_stats))
        .route("/api/v1/skills/:id", get(skills::get_skill))
        .route("/api/v1/skills/:id", put(skills::update_skill))
        .route("/api/v1/skills/:id", delete(skills::delete_skill))
        .route("/api/v1/skills/category/:category", get(skills::list_by_category))
        .route("/api/v1/skills/tag/:tag", get(skills::list_by_tag))
        .route("/api/v1/skills/:id/execute", post(skills::execute_skill))
        .route("/api/v1/skills/:id/generate-workflow", post(skills::generate_workflow_from_skill))
        .route("/api/v1/skills/import-from-agents", post(skills::import_from_agents))
        .route("/api/v1/skills/import", post(skills::import_skill))
        // 团队路由
        .route("/api/v1/teams", get(teams::list_teams))
        .route("/api/v1/teams", post(teams::create_team))
        .route("/api/v1/teams/:team_id", get(teams::get_team))
        .route("/api/v1/teams/:team_id", put(teams::update_team))
        .route("/api/v1/teams/:team_id", delete(teams::delete_team))
        .route("/api/v1/teams/:team_id/roles", get(teams::list_roles))
        .route("/api/v1/teams/:team_id/roles", post(teams::create_role))
        .route("/api/v1/teams/:team_id/roles/:role_id", delete(teams::remove_role_from_team))
        .route("/api/v1/teams/:team_id/messages", get(teams::get_team_messages))
        .route("/api/v1/teams/:team_id/execute", post(teams::execute_team_task))
        .route("/api/v1/teams/:team_id/telegram", get(teams::get_team_telegram_config))
        .route("/api/v1/teams/:team_id/telegram", put(teams::configure_team_telegram))
        .route("/api/v1/teams/:team_id/telegram/:enabled", post(teams::enable_team_telegram))
        // Per-member bot management
        .route("/api/v1/teams/:team_id/members/bots", get(teams::get_team_member_bots))
        .route("/api/v1/teams/:team_id/members/:role_id/bot", put(teams::configure_member_bot))
        .route("/api/v1/teams/:team_id/members/bots/:enabled", post(teams::toggle_all_member_bots))
        .route("/api/v1/roles/:id", get(teams::get_role))
        .route("/api/v1/roles/:id", put(teams::update_role))
        .route("/api/v1/roles/:id", delete(teams::delete_role))
        .route("/api/v1/roles/:id/team", put(teams::assign_role_to_team))
        .route("/api/v1/roles", get(teams::list_all_roles))
        .route("/api/v1/roles/:id/skills/:skill_id", post(teams::assign_skill))
        .route("/api/v1/roles/:id/skills/:skill_id", delete(teams::remove_skill))
        .route("/api/v1/roles/:id/skills", get(teams::get_role_skills))
        .route("/api/v1/roles/:id/telegram", post(teams::configure_telegram))
        .route("/api/v1/roles/:id/telegram", get(teams::get_telegram_config))
        .route("/api/v1/roles/:id/telegram/:enabled", post(teams::enable_telegram))
        .route("/api/v1/roles/:id/telegram", delete(teams::delete_telegram_config))
        .route("/api/v1/roles/:id/telegram/send", post(teams::send_telegram_message))
        .route("/api/v1/roles/:id/execute", post(teams::execute_role_task))
        // 项目路由
        .route("/api/v1/projects", get(projects::list_projects))
        .route("/api/v1/projects", post(projects::create_project))
        .route("/api/v1/projects/:id", get(projects::get_project))
        .route("/api/v1/projects/:id", put(projects::update_project))
        .route("/api/v1/projects/:id", delete(projects::delete_project))
        .route("/api/v1/projects/team/:team_id", get(projects::list_projects_by_team))
        .route("/api/v1/projects/:id/execute", post(projects::execute_project))
        // 群组讨论路由
        .route("/api/v1/group-sessions", post(group_chat::create_session))
        .route("/api/v1/group-sessions", get(group_chat::list_sessions))
        .route("/api/v1/group-sessions/:id", get(group_chat::get_session))
        .route("/api/v1/group-sessions/:id", put(group_chat::update_session))
        .route("/api/v1/group-sessions/:id", delete(group_chat::delete_session))
        .route("/api/v1/group-sessions/:id/start", post(group_chat::start_discussion))
        .route("/api/v1/group-sessions/:id/messages", get(group_chat::get_messages))
        .route("/api/v1/group-sessions/:id/messages", post(group_chat::send_message))
        .route("/api/v1/group-sessions/:id/next-speaker", get(group_chat::get_next_speaker))
        .route("/api/v1/group-sessions/:id/advance", post(group_chat::advance_speaker))
        .route("/api/v1/group-sessions/:id/conclude", post(group_chat::conclude_discussion))
        .route("/api/v1/group-sessions/:id/execute-turn/:role_id", post(group_chat::execute_role_turn))
        .route("/api/v1/group-sessions/:id/execute-round", post(group_chat::execute_round))
        // 团队记忆路由
        .route("/api/v1/teams/:team_id/memories", post(memory::store_memory))
        .route("/api/v1/teams/:team_id/memories/search", post(memory::search_memory))
        .route("/api/v1/teams/:team_id/memories/stats", get(memory::get_stats))
        .route("/api/v1/teams/:team_id/memories", delete(memory::clear_memory))

        // Process monitoring
        .route("/api/v1/processes", get(processes::list_processes))
        .route("/api/v1/processes/:execution_id/kill", post(processes::kill_process))
        // Issue 路由
        .route("/api/v1/issues", get(issues::list_issues))
        .route("/api/v1/issues", post(issues::create_issue))
        .route("/api/v1/issues/:id", get(issues::get_issue))
        .route("/api/v1/issues/:id", put(issues::update_issue))
        .route("/api/v1/issues/:id", delete(issues::delete_issue))
        // WebSocket 路由
        .route("/ws/executions/:id", get(executions::execution_ws))
        .route("/ws/sessions/:id", get(sessions::session_ws))
        .route("/ws/terminal", get(terminal_ws_handler))
        .route("/ws/claude-stream", get(claude_stream_ws_handler))
        .route("/ws/run-command", get(run_command_ws_handler))
        .route("/ws/agent-executions/:execution_id", get(agent_execution_ws_handler))
        // 应用认证中间件到所有 API 和 WebSocket 路由
        .route_layer(axum::middleware::from_fn_with_state(
            app_state_for_router.clone(),
            ApiKeyAuth::middleware,
        ))
        .with_state(app_state_for_router);

    // 合并公共路由（健康检查无需认证）
    let app = Router::new()
        .route("/health", get(health::health_check))
        .merge(api_routes);

    // 添加 CORS 中间件
    let app = if config.cors_enabled {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
        app.layer(cors)
    } else {
        app
    };

    (app, app_state)
}

/// 终端 WebSocket 处理函数
async fn terminal_ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    let handler = TerminalWsHandler::new();

    ws.on_upgrade(async move |socket| {
        handler.handle(socket).await;
    })
}

/// Claude CLI 流式 WebSocket 处理函数
async fn claude_stream_ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    let handler = ClaudeStreamWsHandler::new();

    ws.on_upgrade(async move |socket| {
        handler.handle(socket).await;
    })
}

/// 通用命令执行 WebSocket 处理函数
async fn run_command_ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    let handler = RunCommandWsHandler::new();

    ws.on_upgrade(async move |socket| {
        handler.handle(socket).await;
    })
}

/// Agent 执行 WebSocket 处理函数
async fn agent_execution_ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(execution_id): Path<String>,
) -> impl axum::response::IntoResponse {
    let manager = state.agent_execution_manager.clone();

    ws.on_upgrade(async move |socket| {
        crate::ws::agent_execution::handle_agent_execution_ws(socket, execution_id, manager).await;
    })
}
