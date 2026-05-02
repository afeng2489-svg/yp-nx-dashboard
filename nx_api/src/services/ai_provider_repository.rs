//! AI Provider Repository
//!
//! SQLite implementation for storing AI provider configurations.

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Repository errors
#[derive(Error, Debug)]
pub enum ProviderRepositoryError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Provider not found: {0}")]
    NotFound(String),

    #[error("Encryption/decryption error: {0}")]
    Crypto(String),

    #[error("Invalid config: {0}")]
    InvalidConfig(String),
}

/// API Format enum
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum APIFormat {
    OpenAI,
    Anthropic,
    Custom(String),
}

impl Default for APIFormat {
    fn default() -> Self {
        APIFormat::OpenAI
    }
}

impl std::fmt::Display for APIFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            APIFormat::OpenAI => write!(f, "openai"),
            APIFormat::Anthropic => write!(f, "anthropic"),
            APIFormat::Custom(s) => write!(f, "custom:{}", s),
        }
    }
}

impl From<&str> for APIFormat {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "openai" => APIFormat::OpenAI,
            "anthropic" => APIFormat::Anthropic,
            other => APIFormat::Custom(other.to_string()),
        }
    }
}

/// Mapping type enum
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MappingType {
    Main,
    Thinking,
    Haiku,
    Sonnet,
    Opus,
}

impl std::fmt::Display for MappingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MappingType::Main => write!(f, "main"),
            MappingType::Thinking => write!(f, "thinking"),
            MappingType::Haiku => write!(f, "haiku"),
            MappingType::Sonnet => write!(f, "sonnet"),
            MappingType::Opus => write!(f, "opus"),
        }
    }
}

impl From<&str> for MappingType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "main" => MappingType::Main,
            "thinking" => MappingType::Thinking,
            "haiku" => MappingType::Haiku,
            "sonnet" => MappingType::Sonnet,
            "opus" => MappingType::Opus,
            other => MappingType::Main,
        }
    }
}

/// AI Provider model
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AIProvider {
    pub id: String,
    pub provider_key: String,
    pub name: String,
    pub description: Option<String>,
    pub website: Option<String>,
    #[serde(skip_serializing)]
    pub api_key: Option<String>,
    pub base_url: String,
    pub api_format: APIFormat,
    pub auth_field: String,
    pub enabled: bool,
    pub config_json: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Model Mapping model
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelMapping {
    pub id: String,
    pub provider_id: String,
    pub mapping_type: MappingType,
    pub model_id: String,
    pub display_name: Option<String>,
    pub config_json: Option<String>,
}

/// Preset provider definition
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderPreset {
    pub key: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub website: &'static str,
    pub base_url: &'static str,
    pub api_format: APIFormat,
    pub default_auth_field: &'static str,
}

/// All preset providers
pub const PRESET_PROVIDERS: &[ProviderPreset] = &[
    ProviderPreset {
        key: "deepseek",
        name: "DeepSeek",
        description: "深度求索AI",
        website: "https://platform.deepseek.com",
        base_url: "https://api.deepseek.com/chat/completions",
        api_format: APIFormat::OpenAI,
        default_auth_field: "Authorization",
    },
    ProviderPreset {
        key: "zhipu_glm",
        name: "Zhipu GLM",
        description: "智谱AI",
        website: "https://open.bigmodel.cn",
        base_url: "https://open.bigmodel.cn/api/paas/v4/chat/completions",
        api_format: APIFormat::OpenAI,
        default_auth_field: "Authorization",
    },
    ProviderPreset {
        key: "zhipu_glm_en",
        name: "Zhipu GLM en",
        description: "智谱AI国际版",
        website: "https://open.bigmodel.cn",
        base_url: "https://open.bigmodel.cn/api/paas/v4/chat/completions",
        api_format: APIFormat::OpenAI,
        default_auth_field: "Authorization",
    },
    ProviderPreset {
        key: "qwen_coder",
        name: "Qwen Coder",
        description: "阿里通义千问代码模型",
        website: "https://qwen.cloud.alibaba.com",
        base_url: "https://qwen/cloud.alibaba.com/api/paas/v4/chat/completions",
        api_format: APIFormat::OpenAI,
        default_auth_field: "Authorization",
    },
    ProviderPreset {
        key: "kimi",
        name: "Kimi",
        description: "月之暗面 Moonshot AI",
        website: "https://platform.moonshot.cn",
        base_url: "https://api.moonshot.cn/v1/chat/completions",
        api_format: APIFormat::OpenAI,
        default_auth_field: "Authorization",
    },
    ProviderPreset {
        key: "kimi_k25",
        name: "Kimi k2.5",
        description: "Kimi 2.5",
        website: "https://platform.moonshot.cn",
        base_url: "https://api.moonshot.cn/v1/chat/completions",
        api_format: APIFormat::OpenAI,
        default_auth_field: "Authorization",
    },
    ProviderPreset {
        key: "kimi_for_coding",
        name: "Kimi For Coding",
        description: "Kimi 代码专用模型",
        website: "https://platform.moonshot.cn",
        base_url: "https://api.moonshot.cn/v1/chat/completions",
        api_format: APIFormat::OpenAI,
        default_auth_field: "Authorization",
    },
    ProviderPreset {
        key: "stepfun",
        name: "StepFun",
        description: "阶跃星辰",
        website: "https://www.stepfun.com",
        base_url: "https://api.stepfun.com/v1/chat/completions",
        api_format: APIFormat::OpenAI,
        default_auth_field: "Authorization",
    },
    ProviderPreset {
        key: "minimax",
        name: "MiniMax",
        description: "MiniMax 海螺AI",
        website: "https://platform.minimaxi.com",
        base_url: "https://api.minimaxi.com/anthropic",
        api_format: APIFormat::Anthropic,
        default_auth_field: "Authorization",
    },
    ProviderPreset {
        key: "minimax_en",
        name: "MiniMax en",
        description: "MiniMax 国际版",
        website: "https://www.minimax.chat",
        base_url: "https://api.minimax.chat/v1/chat/completions",
        api_format: APIFormat::OpenAI,
        default_auth_field: "Authorization",
    },
    ProviderPreset {
        key: "openrouter",
        name: "OpenRouter",
        description: "OpenRouter 聚合平台",
        website: "https://openrouter.ai",
        base_url: "https://openrouter.ai/api/v1/chat/completions",
        api_format: APIFormat::OpenAI,
        default_auth_field: "Authorization",
    },
    ProviderPreset {
        key: "modelscope",
        name: "ModelScope",
        description: "魔搭社区",
        website: "https://modelscope.cn",
        base_url: "https://api.modelscope.cn/v1/chat/completions",
        api_format: APIFormat::OpenAI,
        default_auth_field: "Authorization",
    },
    ProviderPreset {
        key: "siliconflow",
        name: "SiliconFlow",
        description: "SiliconFlow",
        website: "https://siliconflow.cn",
        base_url: "https://api.siliconflow.cn/v1/chat/completions",
        api_format: APIFormat::OpenAI,
        default_auth_field: "Authorization",
    },
    ProviderPreset {
        key: "novita",
        name: "Novita AI",
        description: "Novita AI",
        website: "https://novita.ai",
        base_url: "https://api.novita.ai/v1/chat/completions",
        api_format: APIFormat::OpenAI,
        default_auth_field: "Authorization",
    },
    ProviderPreset {
        key: "nvidia",
        name: "NVIDIA",
        description: "NVIDIA NIM",
        website: "https://ai.nvidia.com",
        base_url: "https://api.nvidia.com/executing-container/chat/completions",
        api_format: APIFormat::OpenAI,
        default_auth_field: "nv-api-key",
    },
    ProviderPreset {
        key: "aws_bedrock",
        name: "AWS Bedrock",
        description: "AWS Bedrock",
        website: "https://aws.amazon.com/bedrock",
        base_url: "",
        api_format: APIFormat::Anthropic,
        default_auth_field: "x-amz-security-token",
    },
];

/// Provider repository trait
pub trait ProviderRepository: Send + Sync {
    // Provider CRUD
    fn create_provider(&self, provider: &AIProvider) -> Result<(), ProviderRepositoryError>;
    fn get_provider(&self, id: &str) -> Result<Option<AIProvider>, ProviderRepositoryError>;
    fn get_provider_by_key(
        &self,
        provider_key: &str,
    ) -> Result<Option<AIProvider>, ProviderRepositoryError>;
    fn list_providers(&self) -> Result<Vec<AIProvider>, ProviderRepositoryError>;
    fn update_provider(&self, provider: &AIProvider) -> Result<(), ProviderRepositoryError>;
    fn delete_provider(&self, id: &str) -> Result<bool, ProviderRepositoryError>;

    // Model Mappings
    fn create_model_mapping(&self, mapping: &ModelMapping) -> Result<(), ProviderRepositoryError>;
    fn get_model_mappings(
        &self,
        provider_id: &str,
    ) -> Result<Vec<ModelMapping>, ProviderRepositoryError>;
    fn get_model_mapping_by_type(
        &self,
        provider_id: &str,
        mapping_type: &MappingType,
    ) -> Result<Option<ModelMapping>, ProviderRepositoryError>;
    fn update_model_mapping(&self, mapping: &ModelMapping) -> Result<(), ProviderRepositoryError>;
    fn delete_model_mapping(&self, id: &str) -> Result<bool, ProviderRepositoryError>;
    fn delete_model_mappings_by_provider(
        &self,
        provider_id: &str,
    ) -> Result<(), ProviderRepositoryError>;

    // Encryption helpers
    fn save_api_key(&self, provider_id: &str, api_key: &str)
        -> Result<(), ProviderRepositoryError>;
    fn get_api_key(&self, provider_id: &str) -> Result<Option<String>, ProviderRepositoryError>;
    fn delete_api_key(&self, provider_id: &str) -> Result<bool, ProviderRepositoryError>;
}

/// Simple XOR encryption
fn simple_encrypt(key: &str, plaintext: &str) -> Result<String, ProviderRepositoryError> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let seed = hasher.finish();

    let encrypted: Vec<u8> = plaintext
        .bytes()
        .enumerate()
        .map(|(i, b)| {
            let key_byte = ((seed >> ((i % 8) * 8)) & 0xFF) as u8;
            b ^ key_byte
        })
        .collect();

    Ok(base64_encode_bytes(&encrypted))
}

fn simple_decrypt(key: &str, ciphertext: &str) -> Result<String, ProviderRepositoryError> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let encrypted = base64_decode(ciphertext)
        .map_err(|e| ProviderRepositoryError::Crypto(format!("Base64 decode error: {}", e)))?;

    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let seed = hasher.finish();

    let decrypted: Vec<u8> = encrypted
        .iter()
        .enumerate()
        .map(|(i, &b)| {
            let key_byte = ((seed >> ((i % 8) * 8)) & 0xFF) as u8;
            b ^ key_byte
        })
        .collect();

    String::from_utf8(decrypted)
        .map_err(|e| ProviderRepositoryError::Crypto(format!("UTF-8 decode error: {}", e)))
}

fn base64_encode_bytes(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3F] as char);
        } else {
            result.push('=');
        }
    }

    result
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim_end_matches('=');
    let mut result = Vec::new();

    let table: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    for chunk in input.as_bytes().chunks(4) {
        let mut buf = [0u8; 4];
        for (i, &byte) in chunk.iter().enumerate() {
            buf[i] = match byte {
                b'A'..=b'Z' => byte - b'A',
                b'a'..=b'z' => byte - b'a' + 26,
                b'0'..=b'9' => byte - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                _ => return Err(format!("Invalid base64 character: {}", byte as char)),
            };
        }

        result.push((buf[0] << 2) | (buf[1] >> 4));
        if chunk.len() > 2 {
            result.push((buf[1] << 4) | (buf[2] >> 2));
        }
        if chunk.len() > 3 {
            result.push((buf[2] << 6) | buf[3]);
        }
    }

    Ok(result)
}

/// SQLite AI Provider Repository
#[derive(Debug, Clone)]
pub struct SqliteProviderRepository {
    conn: Arc<Mutex<Connection>>,
    encryption_key: String,
}

impl SqliteProviderRepository {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, ProviderRepositoryError> {
        let conn = Connection::open(db_path)?;

        // WAL mode for concurrent access, busy_timeout to wait for locks
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;",
        )?;

        let encryption_key = std::env::var("NEXUS_ENCRYPTION_KEY")
            .unwrap_or_else(|_| "default-encryption-key-change-in-production".to_string());

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            encryption_key,
        })
    }

    fn row_to_provider(row: &rusqlite::Row) -> Result<AIProvider, rusqlite::Error> {
        let api_format_str: String = row.get(7)?;
        let created_at_str: String = row.get(11)?;
        let updated_at_str: String = row.get(12)?;

        Ok(AIProvider {
            id: row.get(0)?,
            provider_key: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            website: row.get(4)?,
            api_key: None, // Never expose encrypted key directly
            base_url: row.get(6)?,
            api_format: APIFormat::from(api_format_str.as_str()),
            auth_field: row.get(8)?,
            enabled: row.get::<_, i32>(9)? == 1,
            config_json: row.get(10)?,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }

    fn row_to_mapping(row: &rusqlite::Row) -> Result<ModelMapping, rusqlite::Error> {
        let mapping_type_str: String = row.get(2)?;

        Ok(ModelMapping {
            id: row.get(0)?,
            provider_id: row.get(1)?,
            mapping_type: MappingType::from(mapping_type_str.as_str()),
            model_id: row.get(3)?,
            display_name: row.get(4)?,
            config_json: row.get(5)?,
        })
    }
}

impl ProviderRepository for SqliteProviderRepository {
    fn create_provider(&self, provider: &AIProvider) -> Result<(), ProviderRepositoryError> {
        let now = Utc::now().to_rfc3339();
        let api_format_str = provider.api_format.to_string();

        // Encrypt API key if provided
        let encrypted_key = if let Some(ref key) = provider.api_key {
            Some(simple_encrypt(&self.encryption_key, key)?)
        } else {
            None
        };

        self.conn.lock().execute(
            "INSERT INTO ai_providers (id, provider_key, name, description, website, encrypted_api_key, base_url, api_format, auth_field, enabled, config_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                provider.id,
                provider.provider_key,
                provider.name,
                provider.description,
                provider.website,
                encrypted_key,
                provider.base_url,
                api_format_str,
                provider.auth_field,
                if provider.enabled { 1 } else { 0 },
                provider.config_json,
                now,
                now,
            ],
        )?;

        Ok(())
    }

    fn get_provider(&self, id: &str) -> Result<Option<AIProvider>, ProviderRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, provider_key, name, description, website, encrypted_api_key, base_url, api_format, auth_field, enabled, config_json, created_at, updated_at
             FROM ai_providers WHERE id = ?1",
        )?;

        let result = stmt
            .query_row(params![id], Self::row_to_provider)
            .optional()?;

        Ok(result)
    }

    fn get_provider_by_key(
        &self,
        provider_key: &str,
    ) -> Result<Option<AIProvider>, ProviderRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, provider_key, name, description, website, encrypted_api_key, base_url, api_format, auth_field, enabled, config_json, created_at, updated_at
             FROM ai_providers WHERE provider_key = ?1",
        )?;

        let result = stmt
            .query_row(params![provider_key], Self::row_to_provider)
            .optional()?;

        Ok(result)
    }

    fn list_providers(&self) -> Result<Vec<AIProvider>, ProviderRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, provider_key, name, description, website, encrypted_api_key, base_url, api_format, auth_field, enabled, config_json, created_at, updated_at
             FROM ai_providers ORDER BY name",
        )?;

        let providers = stmt
            .query_map([], Self::row_to_provider)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(providers)
    }

    fn update_provider(&self, provider: &AIProvider) -> Result<(), ProviderRepositoryError> {
        let now = Utc::now().to_rfc3339();
        let api_format_str = provider.api_format.to_string();

        // Only update API key if a new one is provided; otherwise preserve the existing value
        let encrypted_key = if let Some(ref key) = provider.api_key {
            Some(simple_encrypt(&self.encryption_key, key)?)
        } else {
            // Keep existing API key - fetch it from DB
            let existing_key: Option<String> = self
                .conn
                .lock()
                .query_row(
                    "SELECT encrypted_api_key FROM ai_providers WHERE id = ?1",
                    params![provider.id],
                    |row| row.get(0),
                )
                .ok()
                .flatten();
            existing_key
        };

        let affected = self.conn.lock().execute(
            "UPDATE ai_providers SET
             name = ?1, description = ?2, website = ?3, encrypted_api_key = ?4,
             base_url = ?5, api_format = ?6, auth_field = ?7, enabled = ?8,
             config_json = ?9, updated_at = ?10
             WHERE id = ?11",
            params![
                provider.name,
                provider.description,
                provider.website,
                encrypted_key,
                provider.base_url,
                api_format_str,
                provider.auth_field,
                if provider.enabled { 1 } else { 0 },
                provider.config_json,
                now,
                provider.id,
            ],
        )?;

        if affected == 0 {
            return Err(ProviderRepositoryError::NotFound(provider.id.clone()));
        }

        Ok(())
    }

    fn delete_provider(&self, id: &str) -> Result<bool, ProviderRepositoryError> {
        // Delete model mappings first (foreign key)
        self.delete_model_mappings_by_provider(id)?;

        let affected = self
            .conn
            .lock()
            .execute("DELETE FROM ai_providers WHERE id = ?1", params![id])?;

        Ok(affected > 0)
    }

    fn create_model_mapping(&self, mapping: &ModelMapping) -> Result<(), ProviderRepositoryError> {
        let mapping_type_str = mapping.mapping_type.to_string();

        self.conn.lock().execute(
            "INSERT INTO ai_model_mappings (id, provider_id, mapping_type, model_id, display_name, config_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                mapping.id,
                mapping.provider_id,
                mapping_type_str,
                mapping.model_id,
                mapping.display_name,
                mapping.config_json,
            ],
        )?;

        Ok(())
    }

    fn get_model_mappings(
        &self,
        provider_id: &str,
    ) -> Result<Vec<ModelMapping>, ProviderRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, provider_id, mapping_type, model_id, display_name, config_json
             FROM ai_model_mappings WHERE provider_id = ?1",
        )?;

        let mappings = stmt
            .query_map(params![provider_id], Self::row_to_mapping)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(mappings)
    }

    fn get_model_mapping_by_type(
        &self,
        provider_id: &str,
        mapping_type: &MappingType,
    ) -> Result<Option<ModelMapping>, ProviderRepositoryError> {
        let conn = self.conn.lock();
        let mapping_type_str = mapping_type.to_string();

        let mut stmt = conn.prepare(
            "SELECT id, provider_id, mapping_type, model_id, display_name, config_json
             FROM ai_model_mappings WHERE provider_id = ?1 AND mapping_type = ?2",
        )?;

        let result = stmt
            .query_row(params![provider_id, mapping_type_str], Self::row_to_mapping)
            .optional()?;

        Ok(result)
    }

    fn update_model_mapping(&self, mapping: &ModelMapping) -> Result<(), ProviderRepositoryError> {
        let mapping_type_str = mapping.mapping_type.to_string();

        let affected = self.conn.lock().execute(
            "UPDATE ai_model_mappings SET
             mapping_type = ?1, model_id = ?2, display_name = ?3, config_json = ?4
             WHERE id = ?5",
            params![
                mapping_type_str,
                mapping.model_id,
                mapping.display_name,
                mapping.config_json,
                mapping.id,
            ],
        )?;

        if affected == 0 {
            return Err(ProviderRepositoryError::NotFound(mapping.id.clone()));
        }

        Ok(())
    }

    fn delete_model_mapping(&self, id: &str) -> Result<bool, ProviderRepositoryError> {
        let affected = self
            .conn
            .lock()
            .execute("DELETE FROM ai_model_mappings WHERE id = ?1", params![id])?;

        Ok(affected > 0)
    }

    fn delete_model_mappings_by_provider(
        &self,
        provider_id: &str,
    ) -> Result<(), ProviderRepositoryError> {
        self.conn.lock().execute(
            "DELETE FROM ai_model_mappings WHERE provider_id = ?1",
            params![provider_id],
        )?;

        Ok(())
    }

    fn save_api_key(
        &self,
        provider_id: &str,
        api_key: &str,
    ) -> Result<(), ProviderRepositoryError> {
        tracing::info!(
            "[save_api_key] provider_id={}, api_key_len={}",
            provider_id,
            api_key.len()
        );

        // First check if provider exists
        let exists = self.get_provider(provider_id)?.is_some();
        if !exists {
            return Err(ProviderRepositoryError::NotFound(format!(
                "Provider {} not found",
                provider_id
            )));
        }

        let encrypted = simple_encrypt(&self.encryption_key, api_key)?;
        let now = Utc::now().to_rfc3339();

        tracing::info!(
            "[save_api_key] encrypted len={}, encrypted value={}",
            encrypted.len(),
            encrypted
        );

        // Use parameterized query to avoid SQL injection and quoting issues
        let mut conn = self.conn.lock();
        let rows_updated = conn.execute(
            "UPDATE ai_providers SET encrypted_api_key = ?1, updated_at = ?2 WHERE id = ?3",
            params![encrypted, now, provider_id],
        )?;
        tracing::info!("[save_api_key] rows_updated={}", rows_updated);

        if rows_updated == 0 {
            tracing::warn!(
                "[save_api_key] No rows updated for provider_id={}",
                provider_id
            );
        }

        Ok(())
    }

    fn get_api_key(&self, provider_id: &str) -> Result<Option<String>, ProviderRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT encrypted_api_key FROM ai_providers WHERE id = ?1")?;

        let result = stmt
            .query_row(params![provider_id], |row| row.get::<_, Option<String>>(0))
            .optional()?;

        tracing::info!(
            "[get_api_key] provider_id={}, result={:?}",
            provider_id,
            result
        );

        match result {
            Some(Some(encrypted)) => {
                if encrypted.is_empty() {
                    Ok(None)
                } else {
                    let decrypted = simple_decrypt(&self.encryption_key, &encrypted)?;
                    Ok(Some(decrypted))
                }
            }
            Some(None) | None => Ok(None),
        }
    }

    fn delete_api_key(&self, provider_id: &str) -> Result<bool, ProviderRepositoryError> {
        let now = Utc::now().to_rfc3339();

        let affected = self.conn.lock().execute(
            "UPDATE ai_providers SET encrypted_api_key = NULL, updated_at = ?1 WHERE id = ?2",
            params![now, provider_id],
        )?;

        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_providers() {
        assert!(!PRESET_PROVIDERS.is_empty());
        for preset in PRESET_PROVIDERS {
            assert!(!preset.key.is_empty());
            assert!(!preset.name.is_empty());
            // Some presets (e.g. aws_bedrock) use credential-based auth without a base_url
        }
    }

    #[test]
    fn test_api_format() {
        assert_eq!(APIFormat::from("openai"), APIFormat::OpenAI);
        assert_eq!(APIFormat::from("anthropic"), APIFormat::Anthropic);
        assert_eq!(
            APIFormat::from("custom:test"),
            APIFormat::Custom("custom:test".to_string())
        );
    }

    #[test]
    fn test_mapping_type() {
        assert_eq!(MappingType::from("main"), MappingType::Main);
        assert_eq!(MappingType::from("thinking"), MappingType::Thinking);
        assert_eq!(MappingType::from("haiku"), MappingType::Haiku);
        assert_eq!(MappingType::from("sonnet"), MappingType::Sonnet);
        assert_eq!(MappingType::from("opus"), MappingType::Opus);
    }
}
