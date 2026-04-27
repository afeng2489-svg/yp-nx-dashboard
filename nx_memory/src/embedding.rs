//! Embedding 提供者
//!
//! 支持多种嵌入模型：
//! - Claude Embeddings ( Anthropic API )
//! - OpenAI Embeddings
//! - Ollama (本地模型)

// pub mod provider_adapter;

use std::future::Future;
use std::pin::Pin;

/// Embedding 结果
#[derive(Debug, Clone)]
pub struct EmbeddingResult {
    /// 向量
    pub vector: Vec<f32>,
    /// 模型名称
    pub model: String,
    /// Token 数
    pub token_count: usize,
}

/// Embedding 提供者特征
pub trait EmbeddingProvider: Send + Sync {
    /// 提供者名称
    fn name(&self) -> &str;

    /// 生成单个文本的 embedding
    fn embed(
        &self,
        text: &str,
    ) -> Pin<Box<dyn Future<Output = Result<EmbeddingResult, EmbedError>> + Send + '_>>;

    /// 批量生成 embedding
    fn embed_batch(
        &self,
        texts: &[String],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<EmbeddingResult>, EmbedError>> + Send + '_>>;

    /// 获取向量维度
    fn dimension(&self) -> usize;
}

/// Embedding 错误
#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("API 错误: {0}")]
    Api(String),

    #[error("网络错误: {0}")]
    Network(String),

    #[error("认证错误: {0}")]
    Auth(String),

    #[error("解析错误: {0}")]
    Parse(String),

    #[error("不支持的操作: {0}")]
    Unsupported(String),
}

// ─────────────────────────────────────────────────────────────────────────────
// Claude Embedding Provider
// ─────────────────────────────────────────────────────────────────────────────

/// Claude Embedding Provider
///
/// 使用 Anthropic 的 API 生成嵌入向量
/// 注意：Anthropic 已停止提供嵌入 API，这里使用 OpenAI 兼容端点
pub struct ClaudeEmbeddingProvider {
    #[allow(dead_code)]
    client: reqwest::Client,
    api_key: String,
    model: String,
    dimension: usize,
    base_url: String,
}

impl ClaudeEmbeddingProvider {
    /// 创建新的 Claude Embedding Provider
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            model: "claude-embedding-3".to_string(),
            dimension: 1024,
            base_url: "https://api.anthropic.com/v1".to_string(),
        }
    }

    /// 创建带自定义配置的 Provider
    pub fn with_config(api_key: impl Into<String>, model: &str, dimension: usize) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            model: model.to_string(),
            dimension,
            base_url: "https://api.anthropic.com/v1".to_string(),
        }
    }
}

impl EmbeddingProvider for ClaudeEmbeddingProvider {
    fn name(&self) -> &str {
        "claude-embedding"
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed(
        &self,
        text: &str,
    ) -> Pin<Box<dyn Future<Output = Result<EmbeddingResult, EmbedError>> + Send + '_>> {
        let text = text.to_string();
        let api_key = self.api_key.clone();
        let model = self.model.clone();
        let base_url = self.base_url.clone();

        Box::pin(async move {
            let response = reqwest::Client::new()
                .post(format!("{}/embeddings", base_url))
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .header("anthropic-version", "2023-06-01")
                .json(&serde_json::json!({
                    "model": model,
                    "input": text,
                }))
                .send()
                .await
                .map_err(|e| EmbedError::Network(e.to_string()))?;

            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(match status.as_u16() {
                    401 | 403 => EmbedError::Auth(format!("认证失败: {}", error_text)),
                    _ => EmbedError::Api(format!("API 错误 {}: {}", status, error_text)),
                });
            }

            #[derive(serde::Deserialize)]
            struct Response {
                data: Vec<DataItem>,
                usage: Usage,
            }

            #[derive(serde::Deserialize)]
            struct DataItem {
                embedding: Vec<f32>,
            }

            #[derive(serde::Deserialize)]
            struct Usage {
                input_tokens: usize,
            }

            let resp: Response = response
                .json()
                .await
                .map_err(|e| EmbedError::Parse(e.to_string()))?;

            let embedding = resp
                .data
                .into_iter()
                .next()
                .ok_or_else(|| EmbedError::Parse("响应中缺少 embedding 数据".to_string()))?;

            let vector = embedding.embedding;

            Ok(EmbeddingResult {
                vector,
                model,
                token_count: resp.usage.input_tokens,
            })
        })
    }

    fn embed_batch(
        &self,
        texts: &[String],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<EmbeddingResult>, EmbedError>> + Send + '_>> {
        let texts = texts.to_vec();
        let api_key = self.api_key.clone();
        let model = self.model.clone();
        let base_url = self.base_url.clone();

        Box::pin(async move {
            let response = reqwest::Client::new()
                .post(format!("{}/embeddings", base_url))
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .header("anthropic-version", "2023-06-01")
                .json(&serde_json::json!({
                    "model": model,
                    "input": texts,
                }))
                .send()
                .await
                .map_err(|e| EmbedError::Network(e.to_string()))?;

            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(match status.as_u16() {
                    401 | 403 => EmbedError::Auth(format!("认证失败: {}", error_text)),
                    _ => EmbedError::Api(format!("API 错误 {}: {}", status, error_text)),
                });
            }

            #[derive(serde::Deserialize)]
            struct Response {
                data: Vec<DataItem>,
                usage: Usage,
            }

            #[derive(serde::Deserialize)]
            struct DataItem {
                embedding: Vec<f32>,
            }

            #[derive(serde::Deserialize)]
            struct Usage {
                input_tokens: usize,
            }

            let resp: Response = response
                .json()
                .await
                .map_err(|e| EmbedError::Parse(e.to_string()))?;

            let total_tokens = resp.usage.input_tokens;
            let per_text_tokens = if texts.is_empty() {
                0
            } else {
                total_tokens / texts.len()
            };

            let results = resp
                .data
                .into_iter()
                .map(|item| EmbeddingResult {
                    vector: item.embedding,
                    model: model.clone(),
                    token_count: per_text_tokens,
                })
                .collect();

            Ok(results)
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// OpenAI Embedding Provider
// ─────────────────────────────────────────────────────────────────────────────

/// OpenAI Embedding Provider
pub struct OpenAIEmbeddingProvider {
    #[allow(dead_code)]
    client: reqwest::Client,
    api_key: String,
    model: String,
    dimension: usize,
}

impl OpenAIEmbeddingProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model: "text-embedding-3-small".to_string(),
            dimension: 1536,
        }
    }

    pub fn with_config(api_key: impl Into<String>, model: &str, dimension: usize) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model: model.to_string(),
            dimension,
        }
    }
}

impl EmbeddingProvider for OpenAIEmbeddingProvider {
    fn name(&self) -> &str {
        "openai-embedding"
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed(
        &self,
        text: &str,
    ) -> Pin<Box<dyn Future<Output = Result<EmbeddingResult, EmbedError>> + Send + '_>> {
        let text = text.to_string();
        let api_key = self.api_key.clone();
        let model = self.model.clone();

        Box::pin(async move {
            #[derive(serde::Serialize)]
            struct Request {
                model: String,
                input: String,
            }

            #[derive(serde::Deserialize)]
            struct Response {
                data: Vec<DataItem>,
                usage: Usage,
            }

            #[derive(serde::Deserialize)]
            struct DataItem {
                embedding: Vec<f32>,
            }

            #[derive(serde::Deserialize)]
            struct Usage {
                total_tokens: usize,
            }

            let response = reqwest::Client::new()
                .post("https://api.openai.com/v1/embeddings")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&Request {
                    model: model.clone(),
                    input: text,
                })
                .send()
                .await
                .map_err(|e| EmbedError::Network(e.to_string()))?;

            let resp: Response = response
                .json()
                .await
                .map_err(|e| EmbedError::Parse(e.to_string()))?;

            let embedding = resp
                .data
                .into_iter()
                .next()
                .ok_or_else(|| EmbedError::Parse("响应中缺少 embedding 数据".to_string()))?;

            Ok(EmbeddingResult {
                vector: embedding.embedding,
                model,
                token_count: resp.usage.total_tokens,
            })
        })
    }

    fn embed_batch(
        &self,
        texts: &[String],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<EmbeddingResult>, EmbedError>> + Send + '_>> {
        let texts = texts.to_vec();
        let api_key = self.api_key.clone();
        let model = self.model.clone();

        Box::pin(async move {
            #[derive(serde::Serialize)]
            struct Request {
                model: String,
                input: Vec<String>,
            }

            #[derive(serde::Deserialize)]
            struct Response {
                data: Vec<DataItem>,
                usage: Usage,
            }

            #[derive(serde::Deserialize)]
            struct DataItem {
                embedding: Vec<f32>,
            }

            #[derive(serde::Deserialize)]
            struct Usage {
                total_tokens: usize,
            }

            let response = reqwest::Client::new()
                .post("https://api.openai.com/v1/embeddings")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&Request {
                    model: model.clone(),
                    input: texts.clone(),
                })
                .send()
                .await
                .map_err(|e| EmbedError::Network(e.to_string()))?;

            let resp: Response = response
                .json()
                .await
                .map_err(|e| EmbedError::Parse(e.to_string()))?;

            let per_text_tokens = if texts.is_empty() {
                0
            } else {
                resp.usage.total_tokens / texts.len()
            };

            let results = resp
                .data
                .into_iter()
                .map(|item| EmbeddingResult {
                    vector: item.embedding,
                    model: model.clone(),
                    token_count: per_text_tokens,
                })
                .collect();

            Ok(results)
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Ollama Embedding Provider
// ─────────────────────────────────────────────────────────────────────────────

/// Ollama 本地 Embedding Provider
pub struct OllamaEmbeddingProvider {
    #[allow(dead_code)]
    client: reqwest::Client,
    base_url: String,
    model: String,
    dimension: usize,
}

impl OllamaEmbeddingProvider {
    pub fn new(base_url: impl Into<String>, model: &str, dimension: usize) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            model: model.to_string(),
            dimension,
        }
    }

    pub fn default_local() -> Self {
        Self::new("http://localhost:11434", "nomic-embed-text", 768)
    }
}

impl EmbeddingProvider for OllamaEmbeddingProvider {
    fn name(&self) -> &str {
        "ollama-embedding"
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed(
        &self,
        text: &str,
    ) -> Pin<Box<dyn Future<Output = Result<EmbeddingResult, EmbedError>> + Send + '_>> {
        let text = text.to_string();
        let base_url = self.base_url.clone();
        let model = self.model.clone();

        Box::pin(async move {
            #[derive(serde::Serialize)]
            struct Request {
                model: String,
                input: String,
            }

            #[derive(serde::Deserialize)]
            struct Response {
                embeddings: Vec<Vec<f32>>,
            }

            let response = reqwest::Client::new()
                .post(format!("{}/api/embed", base_url))
                .json(&Request {
                    model: model.clone(),
                    input: text.clone(),
                })
                .send()
                .await
                .map_err(|e| EmbedError::Network(e.to_string()))?;

            let resp: Response = response
                .json()
                .await
                .map_err(|e| EmbedError::Parse(e.to_string()))?;

            let vector = resp
                .embeddings
                .into_iter()
                .next()
                .ok_or_else(|| EmbedError::Parse("响应中缺少 embedding 数据".to_string()))?;

            Ok(EmbeddingResult {
                vector,
                model,
                token_count: 0, // Ollama 不返回 token 数
            })
        })
    }

    fn embed_batch(
        &self,
        texts: &[String],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<EmbeddingResult>, EmbedError>> + Send + '_>> {
        let texts = texts.to_vec();
        let base_url = self.base_url.clone();
        let model = self.model.clone();

        Box::pin(async move {
            #[derive(serde::Serialize)]
            struct Request {
                model: String,
                input: Vec<String>,
            }

            #[derive(serde::Deserialize)]
            struct Response {
                embeddings: Vec<Vec<f32>>,
            }

            let response = reqwest::Client::new()
                .post(format!("{}/api/embed", base_url))
                .json(&Request {
                    model: model.clone(),
                    input: texts.clone(),
                })
                .send()
                .await
                .map_err(|e| EmbedError::Network(e.to_string()))?;

            let resp: Response = response
                .json()
                .await
                .map_err(|e| EmbedError::Parse(e.to_string()))?;

            let results = resp
                .embeddings
                .into_iter()
                .map(|vector| EmbeddingResult {
                    vector,
                    model: model.clone(),
                    token_count: 0,
                })
                .collect();

            Ok(results)
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 辅助函数
// ─────────────────────────────────────────────────────────────────────────────

use reqwest::Client;

/// 从环境变量或配置创建 Provider
pub fn create_provider_from_config(
    provider_type: &str,
    api_key: Option<&str>,
    model: Option<&str>,
    dimension: Option<usize>,
) -> Result<Box<dyn EmbeddingProvider>, EmbedError> {
    match provider_type.to_lowercase().as_str() {
        "claude" | "anthropic" => {
            let key = api_key.ok_or_else(|| EmbedError::Auth("缺少 API Key".to_string()))?;
            Ok(Box::new(ClaudeEmbeddingProvider::with_config(
                key,
                model.unwrap_or("claude-embedding-3"),
                dimension.unwrap_or(1024),
            )))
        }
        "openai" => {
            let key = api_key.ok_or_else(|| EmbedError::Auth("缺少 API Key".to_string()))?;
            Ok(Box::new(OpenAIEmbeddingProvider::with_config(
                key,
                model.unwrap_or("text-embedding-3-small"),
                dimension.unwrap_or(1536),
            )))
        }
        "ollama" | "local" => Ok(Box::new(OllamaEmbeddingProvider::new(
            "http://localhost:11434",
            model.unwrap_or("nomic-embed-text"),
            dimension.unwrap_or(768),
        ))),
        _ => Err(EmbedError::Unsupported(format!(
            "不支持的 Provider: {}",
            provider_type
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_provider() {
        let result =
            create_provider_from_config("ollama", None, Some("nomic-embed-text"), Some(768));
        assert!(result.is_ok());
    }
}
