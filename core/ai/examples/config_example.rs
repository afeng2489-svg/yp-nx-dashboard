//! AI Manager 使用示例
//!
//! 展示如何配置和使用 AI 模型管理器

use nexus_ai::{
    AIModelManager, AIManagerConfig, AIRequestParams, APIConfig,
    ModelConfig, ProviderType, AIResponse,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ========== 1. 基础配置 ==========
    println!("=== 1. 基础配置示例 ===");

    // 配置 API 密钥
    let mut api_config = std::collections::HashMap::new();
    api_config.insert(ProviderType::Anthropic, APIConfig {
        api_key: "sk-ant-xxxxx".to_string(),
        base_url: "".to_string(),
        organization_id: "".to_string(),
        timeout_secs: 120,
    });

    // 创建默认模型配置
    let default_model = ModelConfig {
        model_id: "claude-sonnet-4-5".to_string(),
        provider: ProviderType::Anthropic,
        max_tokens: 4096,
        temperature: 0.7,
        stop_sequences: vec![],
        extra_params: std::collections::HashMap::new(),
    };

    // 创建管理器配置
    let manager_config = AIManagerConfig {
        default_model,
        api_config,
        enabled_providers: vec![ProviderType::Anthropic],
    };

    // 创建管理器
    let manager = AIModelManager::from_config(manager_config);

    // ========== 2. 配置多个模型 ==========
    println!("\n=== 2. 配置多个模型 ===");

    // 注册额外的模型配置
    let gpt4_config = ModelConfig {
        model_id: "gpt-4-turbo".to_string(),
        provider: ProviderType::OpenAI,
        max_tokens: 4096,
        temperature: 0.5,
        stop_sequences: vec![],
        extra_params: std::collections::HashMap::new(),
    };
    manager.register_model(gpt4_config);

    let gemini_config = ModelConfig {
        model_id: "gemini-pro".to_string(),
        provider: ProviderType::Google,
        max_tokens: 8192,
        temperature: 0.9,
        stop_sequences: vec![],
        extra_params: std::collections::HashMap::new(),
    };
    manager.register_model(gemini_config);

    // 列出所有可用模型
    println!("可用的模型:");
    for model in manager.list_models() {
        println!("  - {} ({})", model.model_id, model.provider);
    }

    // ========== 3. 执行 AI 调用 ==========
    println!("\n=== 3. AI 调用示例 ===");

    // 简单补全调用
    let response = manager.call(
        "claude-sonnet-4-5",
        "解释什么是机器学习".to_string()
    ).await;

    match response {
        Ok(result) => {
            println!("模型: {}", result.model);
            println!("提供商: {}", result.provider);
            println!("输入 Tokens: {}", result.input_tokens);
            println!("输出 Tokens: {}", result.output_tokens);
            println!("响应:\n{}", result.text);
        }
        Err(e) => {
            eprintln!("调用失败: {}", e);
        }
    }

    // ========== 4. 聊天请求 ==========
    println!("\n=== 4. 聊天请求示例 ===");

    let chat_response = manager.chat(
        AIRequestParams::chat(
            manager.get_model_config("claude-sonnet-4-5").unwrap(),
            "给我写一个 Rust 语言的 hello world 程序".to_string()
        )
        .with_system_prompt("你是一个专业的编程助手".to_string())
    ).await;

    match chat_response {
        Ok(result) => {
            println!("聊天响应:\n{}", result.text);
        }
        Err(e) => {
            eprintln!("聊天失败: {}", e);
        }
    }

    // ========== 5. JSON 配置导出 ==========
    println!("\n=== 5. 配置序列化 ===");

    let current_config = manager.get_config();
    let json = serde_json::to_string_pretty(&current_config)?;
    println!("当前配置:\n{}", json);

    // ========== 6. 从 JSON 加载配置 ==========
    println!("\n=== 6. 从 JSON 加载配置 ===");

    let json_config = r#"{
        "default_model": {
            "model_id": "claude-opus-4-5",
            "provider": "anthropic",
            "max_tokens": 8192,
            "temperature": 0.8
        },
        "api_config": {
            "anthropic": {
                "api_key": "sk-ant-xxxxx",
                "timeout_secs": 120
            }
        },
        "enabled_providers": ["anthropic"]
    }"#;

    let loaded_config: AIManagerConfig = serde_json::from_str(json_config)?;
    println!("成功从 JSON 加载配置");
    println!("默认模型: {} ({})",
        loaded_config.default_model.model_id,
        loaded_config.default_model.provider
    );

    Ok(())
}
