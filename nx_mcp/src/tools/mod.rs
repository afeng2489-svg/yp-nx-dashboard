//! MCP 工具定义
//!
//! 提供 40+ 工具支持工作流操作。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 输入模式 JSON Schema
    pub input_schema: ToolInputSchema,
    /// 工具类别
    pub category: ToolCategory,
}

/// 工具类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    /// 工作流操作
    Workflow,
    /// 文件操作
    FileSystem,
    /// 代码搜索
    Search,
    /// 智能体操作
    Agent,
    /// 会话管理
    Session,
    /// 通知
    Notification,
    /// 进度跟踪
    Progress,
    /// 系统信息
    System,
    /// 网络操作
    Network,
    /// 数据处理
    Data,
}

/// 工具输入模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInputSchema {
    /// 类型
    #[serde(rename = "type")]
    pub schema_type: String,
    /// 属性
    pub properties: HashMap<String, PropertySchema>,
    /// 必需的属性
    pub required: Vec<String>,
}

/// 属性模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySchema {
    /// 类型
    #[serde(rename = "type")]
    pub property_type: String,
    /// 描述
    pub description: Option<String>,
    /// 默认值
    pub default: Option<serde_json::Value>,
    /// 枚举值
    pub enum_values: Option<Vec<serde_json::Value>>,
}

/// 工具输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    /// 工具名称
    pub name: String,
    /// 参数
    pub arguments: serde_json::Value,
}

/// 工具输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// 内容
    pub content: Vec<ToolContent>,
    /// 是否有错误
    pub is_error: bool,
}

/// 工具内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolContent {
    /// 文本
    Text(String),
    /// 图片
    Image { data: String, mime_type: String },
}

/// 工具处理器特征
pub trait ToolHandler: Send + Sync {
    /// 处理工具调用
    fn handle(&self, input: serde_json::Value) -> Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, crate::server::McpError>> + Send + '_>>;
}

/// 简单函数工具处理器
pub struct FunctionToolHandler<F, Fut>
where
    F: Fn(serde_json::Value) -> Fut,
    Fut: std::future::Future<Output = Result<serde_json::Value, crate::server::McpError>> + Send,
{
    func: F,
}

impl<F, Fut> FunctionToolHandler<F, Fut>
where
    F: Fn(serde_json::Value) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<serde_json::Value, crate::server::McpError>> + Send,
{
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

impl<F, Fut> ToolHandler for FunctionToolHandler<F, Fut>
where
    F: Fn(serde_json::Value) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<serde_json::Value, crate::server::McpError>> + Send,
{
    fn handle(&self, input: serde_json::Value) -> Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, crate::server::McpError>> + Send + '_>> {
        Box::pin(async move { (self.func)(input).await })
    }
}

/// 内置工具注册表
pub struct ToolRegistry {
    tools: HashMap<String, Tool>,
}

impl ToolRegistry {
    /// 创建新的工具注册表
    pub fn new() -> Self {
        let mut registry = Self { tools: HashMap::new() };
        registry.register_all();
        registry
    }

    /// 注册所有内置工具
    fn register_all(&mut self) {
        // ========== 工作流工具 ==========
        self.register(workflow_create_tool());
        self.register(workflow_execute_tool());
        self.register(workflow_validate_tool());
        self.register(workflow_list_tool());
        self.register(workflow_get_tool());
        self.register(workflow_stop_tool());
        self.register(workflow_pause_tool());
        self.register(workflow_resume_tool());
        self.register(workflow_status_tool());

        // ========== 文件系统工具 ==========
        self.register(fs_read_tool());
        self.register(fs_write_tool());
        self.register(fs_copy_tool());
        self.register(fs_move_tool());
        self.register(fs_delete_tool());
        self.register(fs_exists_tool());
        self.register(fs_mkdir_tool());
        self.register(fs_list_tool());
        self.register(fs_search_tool());
        self.register(fs_stat_tool());

        // ========== 代码搜索工具 ==========
        self.register(search_semantic_tool());
        self.register(search_keyword_tool());
        self.register(search_hybrid_tool());
        self.register(search_symbol_tool());
        self.register(search_imports_tool());
        self.register(search_references_tool());
        self.register(search_documentation_tool());

        // ========== 智能体工具 ==========
        self.register(agent_create_tool());
        self.register(agent_execute_tool());
        self.register(agent_list_tool());
        self.register(agent_stop_tool());
        self.register(agent_state_tool());

        // ========== 会话工具 ==========
        self.register(session_create_tool());
        self.register(session_get_tool());
        self.register(session_list_tool());
        self.register(session_delete_tool());
        self.register(session_export_tool());

        // ========== 通知工具 ==========
        self.register(notify_send_tool());
        self.register(notify_list_tool());
        self.register(notify_acknowledge_tool());
        self.register(notify_clear_tool());

        // ========== 进度工具 ==========
        self.register(progress_start_tool());
        self.register(progress_update_tool());
        self.register(progress_complete_tool());
        self.register(progress_fail_tool());
        self.register(progress_list_tool());

        // ========== 系统工具 ==========
        self.register(system_info_tool());
        self.register(system_health_tool());
        self.register(system_metrics_tool());
        self.register(system_config_tool());

        // ========== 网络工具 ==========
        self.register(net_fetch_tool());
        self.register(net_webhook_tool());
        self.register(net_status_tool());

        // ========== 数据处理工具 ==========
        self.register(data_parse_tool());
        self.register(data_transform_tool());
        self.register(data_validate_tool());
        self.register(data_convert_tool());
    }

    /// 注册工具
    fn register(&mut self, tool: Tool) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// 获取工具
    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }

    /// 获取所有工具
    pub fn get_all(&self) -> Vec<&Tool> {
        self.tools.values().collect()
    }

    /// 按类别获取工具
    pub fn get_by_category(&self, category: ToolCategory) -> Vec<&Tool> {
        self.tools.values()
            .filter(|t| t.category == category)
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ========== 工作流工具 ==========

fn workflow_create_tool() -> Tool {
    Tool {
        name: "workflow_create".to_string(),
        description: "创建新的工作流".to_string(),
        category: ToolCategory::Workflow,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("name", PropertySchema { property_type: "string".to_string(), description: Some("工作流名称".to_string()), default: None, enum_values: None }),
                ("definition", PropertySchema { property_type: "object".to_string(), description: Some("工作流定义".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["name".to_string(), "definition".to_string()],
        },
    }
}

fn workflow_execute_tool() -> Tool {
    Tool {
        name: "workflow_execute".to_string(),
        description: "执行工作流".to_string(),
        category: ToolCategory::Workflow,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("workflow_id", PropertySchema { property_type: "string".to_string(), description: Some("工作流 ID".to_string()), default: None, enum_values: None }),
                ("input", PropertySchema { property_type: "object".to_string(), description: Some("输入变量".to_string()), default: None, enum_values: None }),
                ("background", PropertySchema { property_type: "boolean".to_string(), description: Some("是否后台执行".to_string()), default: Some(serde_json::Value::Bool(false)), enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["workflow_id".to_string()],
        },
    }
}

fn workflow_validate_tool() -> Tool {
    Tool {
        name: "workflow_validate".to_string(),
        description: "验证工作流定义".to_string(),
        category: ToolCategory::Workflow,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("workflow_id", PropertySchema { property_type: "string".to_string(), description: Some("工作流 ID".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["workflow_id".to_string()],
        },
    }
}

fn workflow_list_tool() -> Tool {
    Tool {
        name: "workflow_list".to_string(),
        description: "列出所有工作流".to_string(),
        category: ToolCategory::Workflow,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("status", PropertySchema { property_type: "string".to_string(), description: Some("按状态过滤".to_string()), default: None, enum_values: Some(vec![serde_json::Value::String("running".to_string()), serde_json::Value::String("completed".to_string()), serde_json::Value::String("failed".to_string())]) }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec![],
        },
    }
}

fn workflow_get_tool() -> Tool {
    Tool {
        name: "workflow_get".to_string(),
        description: "获取工作流详情".to_string(),
        category: ToolCategory::Workflow,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("workflow_id", PropertySchema { property_type: "string".to_string(), description: Some("工作流 ID".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["workflow_id".to_string()],
        },
    }
}

fn workflow_stop_tool() -> Tool {
    Tool {
        name: "workflow_stop".to_string(),
        description: "停止运行中的工作流".to_string(),
        category: ToolCategory::Workflow,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("workflow_id", PropertySchema { property_type: "string".to_string(), description: Some("工作流 ID".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["workflow_id".to_string()],
        },
    }
}

fn workflow_pause_tool() -> Tool {
    Tool {
        name: "workflow_pause".to_string(),
        description: "暂停工作流".to_string(),
        category: ToolCategory::Workflow,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("workflow_id", PropertySchema { property_type: "string".to_string(), description: Some("工作流 ID".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["workflow_id".to_string()],
        },
    }
}

fn workflow_resume_tool() -> Tool {
    Tool {
        name: "workflow_resume".to_string(),
        description: "恢复暂停的工作流".to_string(),
        category: ToolCategory::Workflow,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("workflow_id", PropertySchema { property_type: "string".to_string(), description: Some("工作流 ID".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["workflow_id".to_string()],
        },
    }
}

fn workflow_status_tool() -> Tool {
    Tool {
        name: "workflow_status".to_string(),
        description: "获取工作流执行状态".to_string(),
        category: ToolCategory::Workflow,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("execution_id", PropertySchema { property_type: "string".to_string(), description: Some("执行 ID".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["execution_id".to_string()],
        },
    }
}

// ========== 文件系统工具 ==========

fn fs_read_tool() -> Tool {
    Tool {
        name: "fs_read".to_string(),
        description: "读取文件内容".to_string(),
        category: ToolCategory::FileSystem,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("path", PropertySchema { property_type: "string".to_string(), description: Some("文件路径".to_string()), default: None, enum_values: None }),
                ("encoding", PropertySchema { property_type: "string".to_string(), description: Some("文件编码".to_string()), default: Some(serde_json::Value::String("utf-8".to_string())), enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["path".to_string()],
        },
    }
}

fn fs_write_tool() -> Tool {
    Tool {
        name: "fs_write".to_string(),
        description: "写入文件内容".to_string(),
        category: ToolCategory::FileSystem,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("path", PropertySchema { property_type: "string".to_string(), description: Some("文件路径".to_string()), default: None, enum_values: None }),
                ("content", PropertySchema { property_type: "string".to_string(), description: Some("文件内容".to_string()), default: None, enum_values: None }),
                ("create_dirs", PropertySchema { property_type: "boolean".to_string(), description: Some("是否创建目录".to_string()), default: Some(serde_json::Value::Bool(true)), enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["path".to_string(), "content".to_string()],
        },
    }
}

fn fs_copy_tool() -> Tool {
    Tool {
        name: "fs_copy".to_string(),
        description: "复制文件或目录".to_string(),
        category: ToolCategory::FileSystem,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("source", PropertySchema { property_type: "string".to_string(), description: Some("源路径".to_string()), default: None, enum_values: None }),
                ("destination", PropertySchema { property_type: "string".to_string(), description: Some("目标路径".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["source".to_string(), "destination".to_string()],
        },
    }
}

fn fs_move_tool() -> Tool {
    Tool {
        name: "fs_move".to_string(),
        description: "移动文件或目录".to_string(),
        category: ToolCategory::FileSystem,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("source", PropertySchema { property_type: "string".to_string(), description: Some("源路径".to_string()), default: None, enum_values: None }),
                ("destination", PropertySchema { property_type: "string".to_string(), description: Some("目标路径".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["source".to_string(), "destination".to_string()],
        },
    }
}

fn fs_delete_tool() -> Tool {
    Tool {
        name: "fs_delete".to_string(),
        description: "删除文件或目录".to_string(),
        category: ToolCategory::FileSystem,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("path", PropertySchema { property_type: "string".to_string(), description: Some("路径".to_string()), default: None, enum_values: None }),
                ("recursive", PropertySchema { property_type: "boolean".to_string(), description: Some("是否递归删除".to_string()), default: Some(serde_json::Value::Bool(false)), enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["path".to_string()],
        },
    }
}

fn fs_exists_tool() -> Tool {
    Tool {
        name: "fs_exists".to_string(),
        description: "检查文件或目录是否存在".to_string(),
        category: ToolCategory::FileSystem,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("path", PropertySchema { property_type: "string".to_string(), description: Some("路径".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["path".to_string()],
        },
    }
}

fn fs_mkdir_tool() -> Tool {
    Tool {
        name: "fs_mkdir".to_string(),
        description: "创建目录".to_string(),
        category: ToolCategory::FileSystem,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("path", PropertySchema { property_type: "string".to_string(), description: Some("目录路径".to_string()), default: None, enum_values: None }),
                ("parents", PropertySchema { property_type: "boolean".to_string(), description: Some("是否创建父目录".to_string()), default: Some(serde_json::Value::Bool(true)), enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["path".to_string()],
        },
    }
}

fn fs_list_tool() -> Tool {
    Tool {
        name: "fs_list".to_string(),
        description: "列出目录内容".to_string(),
        category: ToolCategory::FileSystem,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("path", PropertySchema { property_type: "string".to_string(), description: Some("目录路径".to_string()), default: None, enum_values: None }),
                ("recursive", PropertySchema { property_type: "boolean".to_string(), description: Some("是否递归列出".to_string()), default: Some(serde_json::Value::Bool(false)), enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["path".to_string()],
        },
    }
}

fn fs_search_tool() -> Tool {
    Tool {
        name: "fs_search".to_string(),
        description: "搜索文件".to_string(),
        category: ToolCategory::FileSystem,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("path", PropertySchema { property_type: "string".to_string(), description: Some("搜索路径".to_string()), default: None, enum_values: None }),
                ("pattern", PropertySchema { property_type: "string".to_string(), description: Some("glob 模式".to_string()), default: None, enum_values: None }),
                ("max_depth", PropertySchema { property_type: "integer".to_string(), description: Some("最大搜索深度".to_string()), default: Some(serde_json::Value::Number(10.into())), enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["path".to_string(), "pattern".to_string()],
        },
    }
}

fn fs_stat_tool() -> Tool {
    Tool {
        name: "fs_stat".to_string(),
        description: "获取文件或目录信息".to_string(),
        category: ToolCategory::FileSystem,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("path", PropertySchema { property_type: "string".to_string(), description: Some("路径".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["path".to_string()],
        },
    }
}

// ========== 代码搜索工具 ==========

fn search_semantic_tool() -> Tool {
    Tool {
        name: "search_semantic".to_string(),
        description: "语义搜索代码".to_string(),
        category: ToolCategory::Search,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("query", PropertySchema { property_type: "string".to_string(), description: Some("搜索查询".to_string()), default: None, enum_values: None }),
                ("max_results", PropertySchema { property_type: "integer".to_string(), description: Some("最大结果数".to_string()), default: Some(serde_json::Value::Number(10.into())), enum_values: None }),
                ("language", PropertySchema { property_type: "string".to_string(), description: Some("语言过滤".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["query".to_string()],
        },
    }
}

fn search_keyword_tool() -> Tool {
    Tool {
        name: "search_keyword".to_string(),
        description: "关键词搜索代码".to_string(),
        category: ToolCategory::Search,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("query", PropertySchema { property_type: "string".to_string(), description: Some("搜索查询".to_string()), default: None, enum_values: None }),
                ("case_sensitive", PropertySchema { property_type: "boolean".to_string(), description: Some("是否区分大小写".to_string()), default: Some(serde_json::Value::Bool(false)), enum_values: None }),
                ("whole_word", PropertySchema { property_type: "boolean".to_string(), description: Some("是否全词匹配".to_string()), default: Some(serde_json::Value::Bool(false)), enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["query".to_string()],
        },
    }
}

fn search_hybrid_tool() -> Tool {
    Tool {
        name: "search_hybrid".to_string(),
        description: "混合搜索（语义+关键词）".to_string(),
        category: ToolCategory::Search,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("query", PropertySchema { property_type: "string".to_string(), description: Some("搜索查询".to_string()), default: None, enum_values: None }),
                ("semantic_weight", PropertySchema { property_type: "number".to_string(), description: Some("语义权重".to_string()), default: Some(serde_json::Value::Number(serde_json::Number::from_f64(0.5).unwrap())), enum_values: None }),
                ("max_results", PropertySchema { property_type: "integer".to_string(), description: Some("最大结果数".to_string()), default: Some(serde_json::Value::Number(10.into())), enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["query".to_string()],
        },
    }
}

fn search_symbol_tool() -> Tool {
    Tool {
        name: "search_symbol".to_string(),
        description: "搜索代码符号（函数、类等）".to_string(),
        category: ToolCategory::Search,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("name", PropertySchema { property_type: "string".to_string(), description: Some("符号名称".to_string()), default: None, enum_values: None }),
                ("kind", PropertySchema { property_type: "string".to_string(), description: Some("符号类型".to_string()), default: None, enum_values: Some(vec![serde_json::Value::String("function".to_string()), serde_json::Value::String("class".to_string()), serde_json::Value::String("struct".to_string()), serde_json::Value::String("enum".to_string())]) }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["name".to_string()],
        },
    }
}

fn search_imports_tool() -> Tool {
    Tool {
        name: "search_imports".to_string(),
        description: "搜索导入/引用关系".to_string(),
        category: ToolCategory::Search,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("module", PropertySchema { property_type: "string".to_string(), description: Some("模块名".to_string()), default: None, enum_values: None }),
                ("direction", PropertySchema { property_type: "string".to_string(), description: Some("搜索方向: imports 或 imported_by".to_string()), default: Some(serde_json::Value::String("imports".to_string())), enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["module".to_string()],
        },
    }
}

fn search_references_tool() -> Tool {
    Tool {
        name: "search_references".to_string(),
        description: "查找符号的所有引用".to_string(),
        category: ToolCategory::Search,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("symbol_id", PropertySchema { property_type: "string".to_string(), description: Some("符号 ID".to_string()), default: None, enum_values: None }),
                ("include_definition", PropertySchema { property_type: "boolean".to_string(), description: Some("是否包含定义".to_string()), default: Some(serde_json::Value::Bool(true)), enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["symbol_id".to_string()],
        },
    }
}

fn search_documentation_tool() -> Tool {
    Tool {
        name: "search_documentation".to_string(),
        description: "搜索文档".to_string(),
        category: ToolCategory::Search,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("query", PropertySchema { property_type: "string".to_string(), description: Some("搜索查询".to_string()), default: None, enum_values: None }),
                ("language", PropertySchema { property_type: "string".to_string(), description: Some("语言".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["query".to_string()],
        },
    }
}

// ========== 智能体工具 ==========

fn agent_create_tool() -> Tool {
    Tool {
        name: "agent_create".to_string(),
        description: "创建智能体实例".to_string(),
        category: ToolCategory::Agent,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("role", PropertySchema { property_type: "string".to_string(), description: Some("智能体角色".to_string()), default: None, enum_values: None }),
                ("model", PropertySchema { property_type: "string".to_string(), description: Some("模型名称".to_string()), default: None, enum_values: None }),
                ("config", PropertySchema { property_type: "object".to_string(), description: Some("智能体配置".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["role".to_string()],
        },
    }
}

fn agent_execute_tool() -> Tool {
    Tool {
        name: "agent_execute".to_string(),
        description: "执行智能体任务".to_string(),
        category: ToolCategory::Agent,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("agent_id", PropertySchema { property_type: "string".to_string(), description: Some("智能体 ID".to_string()), default: None, enum_values: None }),
                ("prompt", PropertySchema { property_type: "string".to_string(), description: Some("执行提示".to_string()), default: None, enum_values: None }),
                ("system", PropertySchema { property_type: "string".to_string(), description: Some("系统提示".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["agent_id".to_string(), "prompt".to_string()],
        },
    }
}

fn agent_list_tool() -> Tool {
    Tool {
        name: "agent_list".to_string(),
        description: "列出所有智能体".to_string(),
        category: ToolCategory::Agent,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("status", PropertySchema { property_type: "string".to_string(), description: Some("按状态过滤".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec![],
        },
    }
}

fn agent_stop_tool() -> Tool {
    Tool {
        name: "agent_stop".to_string(),
        description: "停止智能体".to_string(),
        category: ToolCategory::Agent,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("agent_id", PropertySchema { property_type: "string".to_string(), description: Some("智能体 ID".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["agent_id".to_string()],
        },
    }
}

fn agent_state_tool() -> Tool {
    Tool {
        name: "agent_state".to_string(),
        description: "获取智能体状态".to_string(),
        category: ToolCategory::Agent,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("agent_id", PropertySchema { property_type: "string".to_string(), description: Some("智能体 ID".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["agent_id".to_string()],
        },
    }
}

// ========== 会话工具 ==========

fn session_create_tool() -> Tool {
    Tool {
        name: "session_create".to_string(),
        description: "创建新会话".to_string(),
        category: ToolCategory::Session,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("user_id", PropertySchema { property_type: "string".to_string(), description: Some("用户 ID".to_string()), default: None, enum_values: None }),
                ("metadata", PropertySchema { property_type: "object".to_string(), description: Some("会话元数据".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec![],
        },
    }
}

fn session_get_tool() -> Tool {
    Tool {
        name: "session_get".to_string(),
        description: "获取会话信息".to_string(),
        category: ToolCategory::Session,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("session_id", PropertySchema { property_type: "string".to_string(), description: Some("会话 ID".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["session_id".to_string()],
        },
    }
}

fn session_list_tool() -> Tool {
    Tool {
        name: "session_list".to_string(),
        description: "列出所有会话".to_string(),
        category: ToolCategory::Session,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("user_id", PropertySchema { property_type: "string".to_string(), description: Some("用户 ID".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec![],
        },
    }
}

fn session_delete_tool() -> Tool {
    Tool {
        name: "session_delete".to_string(),
        description: "删除会话".to_string(),
        category: ToolCategory::Session,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("session_id", PropertySchema { property_type: "string".to_string(), description: Some("会话 ID".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["session_id".to_string()],
        },
    }
}

fn session_export_tool() -> Tool {
    Tool {
        name: "session_export".to_string(),
        description: "导出会话数据".to_string(),
        category: ToolCategory::Session,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("session_id", PropertySchema { property_type: "string".to_string(), description: Some("会话 ID".to_string()), default: None, enum_values: None }),
                ("format", PropertySchema { property_type: "string".to_string(), description: Some("导出格式: json 或 markdown".to_string()), default: Some(serde_json::Value::String("json".to_string())), enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["session_id".to_string()],
        },
    }
}

// ========== 通知工具 ==========

fn notify_send_tool() -> Tool {
    Tool {
        name: "notify_send".to_string(),
        description: "发送通知".to_string(),
        category: ToolCategory::Notification,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("title", PropertySchema { property_type: "string".to_string(), description: Some("通知标题".to_string()), default: None, enum_values: None }),
                ("body", PropertySchema { property_type: "string".to_string(), description: Some("通知内容".to_string()), default: None, enum_values: None }),
                ("level", PropertySchema { property_type: "string".to_string(), description: Some("通知级别".to_string()), default: Some(serde_json::Value::String("info".to_string())), enum_values: Some(vec![serde_json::Value::String("info".to_string()), serde_json::Value::String("warning".to_string()), serde_json::Value::String("error".to_string()), serde_json::Value::String("success".to_string())]) }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["title".to_string(), "body".to_string()],
        },
    }
}

fn notify_list_tool() -> Tool {
    Tool {
        name: "notify_list".to_string(),
        description: "列出通知".to_string(),
        category: ToolCategory::Notification,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("unread_only", PropertySchema { property_type: "boolean".to_string(), description: Some("仅未读".to_string()), default: Some(serde_json::Value::Bool(false)), enum_values: None }),
                ("limit", PropertySchema { property_type: "integer".to_string(), description: Some("最大数量".to_string()), default: Some(serde_json::Value::Number(20.into())), enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec![],
        },
    }
}

fn notify_acknowledge_tool() -> Tool {
    Tool {
        name: "notify_acknowledge".to_string(),
        description: "确认通知".to_string(),
        category: ToolCategory::Notification,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("notification_id", PropertySchema { property_type: "string".to_string(), description: Some("通知 ID".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["notification_id".to_string()],
        },
    }
}

fn notify_clear_tool() -> Tool {
    Tool {
        name: "notify_clear".to_string(),
        description: "清除通知".to_string(),
        category: ToolCategory::Notification,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("notification_id", PropertySchema { property_type: "string".to_string(), description: Some("通知 ID".to_string()), default: None, enum_values: None }),
                ("all", PropertySchema { property_type: "boolean".to_string(), description: Some("清除所有".to_string()), default: Some(serde_json::Value::Bool(false)), enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec![],
        },
    }
}

// ========== 进度工具 ==========

fn progress_start_tool() -> Tool {
    Tool {
        name: "progress_start".to_string(),
        description: "开始进度跟踪".to_string(),
        category: ToolCategory::Progress,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("task_name", PropertySchema { property_type: "string".to_string(), description: Some("任务名称".to_string()), default: None, enum_values: None }),
                ("total_steps", PropertySchema { property_type: "integer".to_string(), description: Some("总步骤数".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["task_name".to_string(), "total_steps".to_string()],
        },
    }
}

fn progress_update_tool() -> Tool {
    Tool {
        name: "progress_update".to_string(),
        description: "更新进度".to_string(),
        category: ToolCategory::Progress,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("progress_id", PropertySchema { property_type: "string".to_string(), description: Some("进度 ID".to_string()), default: None, enum_values: None }),
                ("current_step", PropertySchema { property_type: "integer".to_string(), description: Some("当前步骤".to_string()), default: None, enum_values: None }),
                ("message", PropertySchema { property_type: "string".to_string(), description: Some("进度消息".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["progress_id".to_string(), "current_step".to_string()],
        },
    }
}

fn progress_complete_tool() -> Tool {
    Tool {
        name: "progress_complete".to_string(),
        description: "完成进度".to_string(),
        category: ToolCategory::Progress,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("progress_id", PropertySchema { property_type: "string".to_string(), description: Some("进度 ID".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["progress_id".to_string()],
        },
    }
}

fn progress_fail_tool() -> Tool {
    Tool {
        name: "progress_fail".to_string(),
        description: "标记进度失败".to_string(),
        category: ToolCategory::Progress,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("progress_id", PropertySchema { property_type: "string".to_string(), description: Some("进度 ID".to_string()), default: None, enum_values: None }),
                ("error", PropertySchema { property_type: "string".to_string(), description: Some("错误信息".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["progress_id".to_string(), "error".to_string()],
        },
    }
}

fn progress_list_tool() -> Tool {
    Tool {
        name: "progress_list".to_string(),
        description: "列出活动进度".to_string(),
        category: ToolCategory::Progress,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: HashMap::new(),
            required: vec![],
        },
    }
}

// ========== 系统工具 ==========

fn system_info_tool() -> Tool {
    Tool {
        name: "system_info".to_string(),
        description: "获取系统信息".to_string(),
        category: ToolCategory::System,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: HashMap::new(),
            required: vec![],
        },
    }
}

fn system_health_tool() -> Tool {
    Tool {
        name: "system_health".to_string(),
        description: "检查系统健康状态".to_string(),
        category: ToolCategory::System,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: HashMap::new(),
            required: vec![],
        },
    }
}

fn system_metrics_tool() -> Tool {
    Tool {
        name: "system_metrics".to_string(),
        description: "获取系统指标".to_string(),
        category: ToolCategory::System,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("metric_type", PropertySchema { property_type: "string".to_string(), description: Some("指标类型".to_string()), default: None, enum_values: Some(vec![serde_json::Value::String("cpu".to_string()), serde_json::Value::String("memory".to_string()), serde_json::Value::String("disk".to_string()), serde_json::Value::String("network".to_string())]) }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec![],
        },
    }
}

fn system_config_tool() -> Tool {
    Tool {
        name: "system_config".to_string(),
        description: "获取或设置系统配置".to_string(),
        category: ToolCategory::System,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("key", PropertySchema { property_type: "string".to_string(), description: Some("配置键".to_string()), default: None, enum_values: None }),
                ("value", PropertySchema { property_type: "string".to_string(), description: Some("配置值".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["key".to_string()],
        },
    }
}

// ========== 网络工具 ==========

fn net_fetch_tool() -> Tool {
    Tool {
        name: "net_fetch".to_string(),
        description: "获取 HTTP 资源".to_string(),
        category: ToolCategory::Network,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("url", PropertySchema { property_type: "string".to_string(), description: Some("URL".to_string()), default: None, enum_values: None }),
                ("method", PropertySchema { property_type: "string".to_string(), description: Some("HTTP 方法".to_string()), default: Some(serde_json::Value::String("GET".to_string())), enum_values: None }),
                ("headers", PropertySchema { property_type: "object".to_string(), description: Some("请求头".to_string()), default: None, enum_values: None }),
                ("body", PropertySchema { property_type: "string".to_string(), description: Some("请求体".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["url".to_string()],
        },
    }
}

fn net_webhook_tool() -> Tool {
    Tool {
        name: "net_webhook".to_string(),
        description: "注册或触发 Webhook".to_string(),
        category: ToolCategory::Network,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("action", PropertySchema { property_type: "string".to_string(), description: Some("动作: register 或 trigger".to_string()), default: None, enum_values: Some(vec![serde_json::Value::String("register".to_string()), serde_json::Value::String("trigger".to_string())]) }),
                ("url", PropertySchema { property_type: "string".to_string(), description: Some("Webhook URL".to_string()), default: None, enum_values: None }),
                ("event", PropertySchema { property_type: "string".to_string(), description: Some("事件类型".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["action".to_string()],
        },
    }
}

fn net_status_tool() -> Tool {
    Tool {
        name: "net_status".to_string(),
        description: "检查网络端点状态".to_string(),
        category: ToolCategory::Network,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("url", PropertySchema { property_type: "string".to_string(), description: Some("URL".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["url".to_string()],
        },
    }
}

// ========== 数据处理工具 ==========

fn data_parse_tool() -> Tool {
    Tool {
        name: "data_parse".to_string(),
        description: "解析数据格式".to_string(),
        category: ToolCategory::Data,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("data", PropertySchema { property_type: "string".to_string(), description: Some("数据字符串".to_string()), default: None, enum_values: None }),
                ("format", PropertySchema { property_type: "string".to_string(), description: Some("格式: json, yaml, csv, xml".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["data".to_string(), "format".to_string()],
        },
    }
}

fn data_transform_tool() -> Tool {
    Tool {
        name: "data_transform".to_string(),
        description: "转换数据格式".to_string(),
        category: ToolCategory::Data,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("data", PropertySchema { property_type: "string".to_string(), description: Some("数据".to_string()), default: None, enum_values: None }),
                ("from", PropertySchema { property_type: "string".to_string(), description: Some("源格式".to_string()), default: None, enum_values: None }),
                ("to", PropertySchema { property_type: "string".to_string(), description: Some("目标格式".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["data".to_string(), "from".to_string(), "to".to_string()],
        },
    }
}

fn data_validate_tool() -> Tool {
    Tool {
        name: "data_validate".to_string(),
        description: "验证数据".to_string(),
        category: ToolCategory::Data,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("data", PropertySchema { property_type: "string".to_string(), description: Some("数据".to_string()), default: None, enum_values: None }),
                ("schema", PropertySchema { property_type: "string".to_string(), description: Some("JSON Schema".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["data".to_string(), "schema".to_string()],
        },
    }
}

fn data_convert_tool() -> Tool {
    Tool {
        name: "data_convert".to_string(),
        description: "数据转换操作".to_string(),
        category: ToolCategory::Data,
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: [
                ("operation", PropertySchema { property_type: "string".to_string(), description: Some("操作类型".to_string()), default: None, enum_values: Some(vec![serde_json::Value::String("uppercase".to_string()), serde_json::Value::String("lowercase".to_string()), serde_json::Value::String("base64_encode".to_string()), serde_json::Value::String("base64_decode".to_string()), serde_json::Value::String("hash".to_string())]) }),
                ("data", PropertySchema { property_type: "string".to_string(), description: Some("数据".to_string()), default: None, enum_values: None }),
            ].into_iter().map(|(k, v): (_, _)| (k.to_string(), v)).collect(),
            required: vec!["operation".to_string(), "data".to_string()],
        },
    }
}
