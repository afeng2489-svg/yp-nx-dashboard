//! API 密钥仓库
//!
//! SQLite 实现的安全存储层，用于存储加密的 API 密钥。

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// 仓库错误
#[derive(Error, Debug)]
pub enum ApiKeyRepositoryError {
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("密钥不存在: {0}")]
    NotFound(String),

    #[error("加密/解密错误: {0}")]
    Crypto(String),
}

/// API 密钥模型
#[derive(Debug, Clone)]
pub struct ApiKey {
    /// 密钥 ID (provider name)
    pub id: String,
    /// 提供商类型
    pub provider: String,
    /// 加密后的密钥
    pub encrypted_key: String,
    /// 密钥最后更新時間
    pub updated_at: DateTime<Utc>,
}

impl ApiKey {
    /// 创建新的 API 密钥
    pub fn new(provider: String, encrypted_key: String) -> Self {
        Self {
            id: provider.clone(),
            provider,
            encrypted_key,
            updated_at: Utc::now(),
        }
    }
}

/// API 密钥仓库 trait
pub trait ApiKeyRepository: Send + Sync {
    /// 保存或更新 API 密钥（密钥会加密存储）
    fn save(&self, provider: &str, api_key: &str) -> Result<(), ApiKeyRepositoryError>;

    /// 获取 API 密钥（返回解密后的内容）
    fn get(&self, provider: &str) -> Result<Option<String>, ApiKeyRepositoryError>;

    /// 检查密钥是否存在
    fn exists(&self, provider: &str) -> Result<bool, ApiKeyRepositoryError>;

    /// 删除 API 密钥
    fn delete(&self, provider: &str) -> Result<bool, ApiKeyRepositoryError>;

    /// 列出所有已配置的提供商
    fn list_providers(&self) -> Result<Vec<String>, ApiKeyRepositoryError>;
}

/// 简单的 XOR 加密（仅用于演示，生产环境应使用 AES-256-GCM 或类似加密）
/// 注意：这是基础混淆，真实环境应使用更安全的加密方案
fn simple_encrypt(key: &str, plaintext: &str) -> Result<String, ApiKeyRepositoryError> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // 使用密钥的哈希作为 XOR 种子
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

    // Base64 编码
    Ok(base64_encode_bytes(&encrypted))
}

fn simple_decrypt(key: &str, ciphertext: &str) -> Result<String, ApiKeyRepositoryError> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let encrypted = base64_decode(ciphertext)
        .map_err(|e| ApiKeyRepositoryError::Crypto(format!("Base64 decode error: {}", e)))?;

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
        .map_err(|e| ApiKeyRepositoryError::Crypto(format!("UTF-8 decode error: {}", e)))
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

/// SQLite API 密钥仓库
#[derive(Debug, Clone)]
pub struct SqliteApiKeyRepository {
    conn: Arc<Mutex<Connection>>,
    /// 加密密钥（用于加密存储的 API 密钥）
    encryption_key: String,
}

impl SqliteApiKeyRepository {
    /// 创建新的 SQLite 仓库
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, ApiKeyRepositoryError> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS api_keys (
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL UNIQUE,
                encrypted_key TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_api_keys_provider ON api_keys(provider);",
        )?;

        // 使用机器特定的密钥（实际应用应该从安全存储获取）
        let encryption_key = std::env::var("NEXUS_ENCRYPTION_KEY")
            .unwrap_or_else(|_| "default-encryption-key-change-in-production".to_string());

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            encryption_key,
        })
    }

    /// 创建内存仓库（用于测试）
    #[allow(dead_code)]
    pub fn in_memory() -> Result<Self, ApiKeyRepositoryError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS api_keys (
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL UNIQUE,
                encrypted_key TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )?;
        let encryption_key = "test-encryption-key".to_string();
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            encryption_key,
        })
    }
}

impl ApiKeyRepository for SqliteApiKeyRepository {
    fn save(&self, provider: &str, api_key: &str) -> Result<(), ApiKeyRepositoryError> {
        let encrypted = simple_encrypt(&self.encryption_key, api_key)?;
        let now = Utc::now().to_rfc3339();

        self.conn.lock().execute(
            "INSERT OR REPLACE INTO api_keys (id, provider, encrypted_key, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![provider, provider, encrypted, now],
        )?;

        Ok(())
    }

    fn get(&self, provider: &str) -> Result<Option<String>, ApiKeyRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT encrypted_key FROM api_keys WHERE provider = ?1")?;

        let result = stmt.query_row(params![provider], |row| Ok(row.get::<_, String>(0)?));

        match result {
            Ok(encrypted) => {
                let decrypted = simple_decrypt(&self.encryption_key, &encrypted)?;
                Ok(Some(decrypted))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(ApiKeyRepositoryError::Database(e)),
        }
    }

    fn exists(&self, provider: &str) -> Result<bool, ApiKeyRepositoryError> {
        let conn = self.conn.lock();
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM api_keys WHERE provider = ?1",
            params![provider],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    fn delete(&self, provider: &str) -> Result<bool, ApiKeyRepositoryError> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "DELETE FROM api_keys WHERE provider = ?1",
            params![provider],
        )?;
        Ok(affected > 0)
    }

    fn list_providers(&self) -> Result<Vec<String>, ApiKeyRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT provider FROM api_keys")?;
        let providers = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(providers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = "test-key";
        let plaintext = "sk-ant-xxx123";

        let encrypted = simple_encrypt(key, plaintext).unwrap();
        let decrypted = simple_decrypt(key, &encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_base64() {
        let input = "Hello, World!";
        let encoded = base64_encode_bytes(input.as_bytes());
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(input.as_bytes(), decoded.as_slice());
    }
}
