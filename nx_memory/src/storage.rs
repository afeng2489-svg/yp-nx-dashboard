//! Memory 存储层
//!
//! 基于 SQLite 实现，支持：
//! - 转录文本存储
//! - 向量存储（序列化到 Blob）
//! - 团队隔离
//! - 全文索引支持

use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::Path;
use std::sync::Mutex;

use crate::types::{MemoryChunk, MemoryStats, MessageRole, Transcript, TranscriptMetadata};

/// Memory 存储
pub struct MemoryStore {
    conn: Mutex<Connection>,
}

impl MemoryStore {
    /// 创建新的存储实例
    pub fn new(db_path: impl AsRef<Path>) -> SqliteResult<Self> {
        let conn = Connection::open(db_path)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// 创建带连接的存储
    pub fn with_connection(conn: Connection) -> SqliteResult<Self> {
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// 初始化数据库 schema
    fn init_schema(&self) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            r#"
            -- 转录表
            CREATE TABLE IF NOT EXISTS transcripts (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL,
                session_id TEXT,
                user_id TEXT NOT NULL,
                role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                metadata TEXT DEFAULT '{}'
            );

            -- 转录表索引
            CREATE INDEX IF NOT EXISTS idx_transcripts_team_id ON transcripts(team_id);
            CREATE INDEX IF NOT EXISTS idx_transcripts_session_id ON transcripts(session_id);
            CREATE INDEX IF NOT EXISTS idx_transcripts_created_at ON transcripts(created_at);
            CREATE INDEX IF NOT EXISTS idx_transcripts_team_created ON transcripts(team_id, created_at);

            -- 记忆块表
            CREATE TABLE IF NOT EXISTS memory_chunks (
                id TEXT PRIMARY KEY,
                transcript_id TEXT NOT NULL REFERENCES transcripts(id) ON DELETE CASCADE,
                content TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                token_count INTEGER NOT NULL,
                embedding BLOB,
                created_at INTEGER NOT NULL
            );

            -- 记忆块表索引
            CREATE INDEX IF NOT EXISTS idx_chunks_transcript_id ON memory_chunks(transcript_id);

            -- 向量索引表（用于向量搜索）
            CREATE TABLE IF NOT EXISTS memory_vectors (
                chunk_id TEXT PRIMARY KEY REFERENCES memory_chunks(id) ON DELETE CASCADE,
                embedding BLOB NOT NULL
            );

            -- 向量表索引
            CREATE INDEX IF NOT EXISTS idx_vectors_chunk_id ON memory_vectors(chunk_id);

            -- BM25 元数据表（用于重建 BM25 索引）
            CREATE TABLE IF NOT EXISTS bm25_metadata (
                chunk_id TEXT PRIMARY KEY REFERENCES memory_chunks(id) ON DELETE CASCADE,
                content TEXT NOT NULL,
                metadata TEXT DEFAULT '{}'
            );

            -- BM25 表索引
            CREATE INDEX IF NOT EXISTS idx_bm25_chunk_id ON bm25_metadata(chunk_id);
            "#,
        )?;

        Ok(())
    }

    /// 存储转录条目
    pub fn store_transcript(&self, transcript: &Transcript) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            r#"
            INSERT OR REPLACE INTO transcripts
            (id, team_id, session_id, user_id, role, content, created_at, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                transcript.id,
                transcript.team_id,
                transcript.session_id,
                transcript.user_id,
                transcript.role.as_str(),
                transcript.content,
                transcript.created_at.timestamp(),
                serde_json::to_string(&transcript.metadata).unwrap_or_default(),
            ],
        )?;

        Ok(())
    }

    /// 存储记忆块
    pub fn store_chunk(&self, chunk: &MemoryChunk) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            r#"
            INSERT OR REPLACE INTO memory_chunks
            (id, transcript_id, content, chunk_index, token_count, embedding, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                chunk.id,
                chunk.transcript_id,
                chunk.content,
                chunk.chunk_index,
                chunk.token_count,
                chunk.embedding.as_ref().map(|v| encode_vector(v)),
                chunk.created_at.timestamp(),
            ],
        )?;

        Ok(())
    }

    /// 存储向量
    pub fn store_vector(&self, chunk_id: &str, embedding: &[f32]) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            r#"
            INSERT OR REPLACE INTO memory_vectors (chunk_id, embedding)
            VALUES (?1, ?2)
            "#,
            params![chunk_id, encode_vector(embedding)],
        )?;

        Ok(())
    }

    /// 存储 BM25 元数据
    pub fn store_bm25_metadata(
        &self,
        chunk_id: &str,
        content: &str,
        metadata: &serde_json::Value,
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            r#"
            INSERT OR REPLACE INTO bm25_metadata (chunk_id, content, metadata)
            VALUES (?1, ?2, ?3)
            "#,
            params![
                chunk_id,
                content,
                serde_json::to_string(metadata).unwrap_or_default()
            ],
        )?;

        Ok(())
    }

    /// 获取团队的所有记忆块（用于重建索引）
    pub fn get_team_chunks(&self, team_id: &str) -> SqliteResult<Vec<(String, String, Vec<u8>)>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            r#"
            SELECT mc.id, mc.content, mv.embedding
            FROM memory_chunks mc
            JOIN transcripts t ON mc.transcript_id = t.id
            LEFT JOIN memory_vectors mv ON mc.id = mv.chunk_id
            WHERE t.team_id = ?1
            ORDER BY mc.created_at DESC
            "#,
        )?;

        let chunks = stmt
            .query_map([team_id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get::<_, Option<Vec<u8>>>(2)?.unwrap_or_default(),
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(chunks)
    }

    /// 获取指定团队的记忆块（用于 BM25 重建）
    pub fn get_team_bm25_data(
        &self,
        team_id: &str,
    ) -> SqliteResult<Vec<(String, String, serde_json::Value)>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            r#"
            SELECT bm.chunk_id, bm.content, bm.metadata
            FROM bm25_metadata bm
            JOIN memory_chunks mc ON bm.chunk_id = mc.id
            JOIN transcripts t ON mc.transcript_id = t.id
            WHERE t.team_id = ?1
            ORDER BY mc.created_at DESC
            "#,
        )?;

        let data = stmt
            .query_map([team_id], |row| {
                let chunk_id: String = row.get(0)?;
                let content: String = row.get(1)?;
                let metadata_str: String = row.get(2)?;
                let metadata: serde_json::Value =
                    serde_json::from_str(&metadata_str).unwrap_or_default();
                Ok((chunk_id, content, metadata))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(data)
    }

    /// 获取指定团队的记忆块和向量（用于向量搜索）
    pub fn get_team_vectors(&self, team_id: &str) -> SqliteResult<Vec<(String, String, Vec<f32>)>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            r#"
            SELECT mc.id, mc.content, mv.embedding
            FROM memory_chunks mc
            JOIN transcripts t ON mc.transcript_id = t.id
            JOIN memory_vectors mv ON mc.id = mv.chunk_id
            WHERE t.team_id = ?1
            ORDER BY mc.created_at DESC
            "#,
        )?;

        let data = stmt
            .query_map([team_id], |row| {
                let chunk_id: String = row.get(0)?;
                let content: String = row.get(1)?;
                let embedding_blob: Vec<u8> = row.get(2)?;
                let embedding = decode_vector(&embedding_blob);
                Ok((chunk_id, content, embedding))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(data)
    }

    /// 获取单个记忆块
    pub fn get_chunk(&self, chunk_id: &str) -> SqliteResult<Option<MemoryChunk>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            r#"
            SELECT id, transcript_id, content, chunk_index, token_count, embedding, created_at
            FROM memory_chunks WHERE id = ?1
            "#,
        )?;

        let chunk = stmt
            .query_row([chunk_id], |row| {
                let embedding_blob: Option<Vec<u8>> = row.get(5)?;
                Ok(MemoryChunk {
                    id: row.get(0)?,
                    transcript_id: row.get(1)?,
                    content: row.get(2)?,
                    chunk_index: row.get(3)?,
                    token_count: row.get(4)?,
                    embedding: embedding_blob.as_deref().map(decode_vector),
                    created_at: chrono::DateTime::from_timestamp(row.get(6)?, 0)
                        .unwrap_or_else(chrono::Utc::now),
                })
            })
            .ok();

        Ok(chunk)
    }

    /// 获取转录条目
    pub fn get_transcript(&self, transcript_id: &str) -> SqliteResult<Option<Transcript>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            r#"
            SELECT id, team_id, session_id, user_id, role, content, created_at, metadata
            FROM transcripts WHERE id = ?1
            "#,
        )?;

        let transcript = stmt
            .query_row([transcript_id], |row| {
                let metadata_str: String = row.get(7)?;
                let metadata: TranscriptMetadata =
                    serde_json::from_str(&metadata_str).unwrap_or_default();
                let role_str: String = row.get(4)?;
                Ok(Transcript {
                    id: row.get(0)?,
                    team_id: row.get(1)?,
                    session_id: row.get(2)?,
                    user_id: row.get(3)?,
                    role: MessageRole::from_str(&role_str).unwrap_or(MessageRole::User),
                    content: row.get(5)?,
                    created_at: chrono::DateTime::from_timestamp(row.get(6)?, 0)
                        .unwrap_or_else(chrono::Utc::now),
                    metadata,
                })
            })
            .ok();

        Ok(transcript)
    }

    /// 删除记忆块
    pub fn delete_chunk(&self, chunk_id: &str) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM memory_chunks WHERE id = ?1", [chunk_id])?;
        Ok(affected > 0)
    }

    /// 删除转录条目及其关联的记忆块
    pub fn delete_transcript(&self, transcript_id: &str) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();
        // 记忆块会通过 ON DELETE CASCADE 自动删除
        let affected = conn.execute("DELETE FROM transcripts WHERE id = ?1", [transcript_id])?;
        Ok(affected > 0)
    }

    /// 获取团队统计信息
    pub fn get_team_stats(&self, team_id: &str) -> SqliteResult<MemoryStats> {
        let conn = self.conn.lock().unwrap();

        let transcript_count: usize = conn.query_row(
            "SELECT COUNT(*) FROM transcripts WHERE team_id = ?1",
            [team_id],
            |row| row.get(0),
        )?;

        let chunk_count: usize = conn.query_row(
            r#"
            SELECT COUNT(*) FROM memory_chunks mc
            JOIN transcripts t ON mc.transcript_id = t.id
            WHERE t.team_id = ?1
            "#,
            [team_id],
            |row| row.get(0),
        )?;

        let embedded_count: usize = conn.query_row(
            r#"
            SELECT COUNT(*) FROM memory_vectors mv
            JOIN memory_chunks mc ON mv.chunk_id = mc.id
            JOIN transcripts t ON mc.transcript_id = t.id
            WHERE t.team_id = ?1
            "#,
            [team_id],
            |row| row.get(0),
        )?;

        let total_tokens: usize = conn.query_row(
            r#"
            SELECT COALESCE(SUM(token_count), 0) FROM memory_chunks mc
            JOIN transcripts t ON mc.transcript_id = t.id
            WHERE t.team_id = ?1
            "#,
            [team_id],
            |row| row.get(0),
        )?;

        Ok(MemoryStats {
            team_id: team_id.to_string(),
            transcript_count,
            chunk_count,
            embedded_count,
            total_tokens,
        })
    }

    /// 检查转录是否存在
    pub fn transcript_exists(&self, transcript_id: &str) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();
        let count: usize = conn.query_row(
            "SELECT COUNT(*) FROM transcripts WHERE id = ?1",
            [transcript_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// 清空团队记忆
    pub fn clear_team(&self, team_id: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM transcripts WHERE team_id = ?1", [team_id])?;
        Ok(())
    }
}

// 向量编码/解码辅助函数
fn encode_vector(vector: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vector.len() * 4);
    for &val in vector {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}

fn decode_vector(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_store_and_retrieve() {
        let dir = tempdir().unwrap();
        let store = MemoryStore::new(dir.path().join("test.db")).unwrap();

        let transcript = Transcript::new("team1", "user1", MessageRole::User, "Hello world");
        store.store_transcript(&transcript).unwrap();

        let chunk = MemoryChunk {
            id: uuid::Uuid::new_v4().to_string(),
            transcript_id: transcript.id.clone(),
            content: "Hello world".to_string(),
            chunk_index: 0,
            token_count: 2,
            embedding: None,
            created_at: transcript.created_at,
        };
        store.store_chunk(&chunk).unwrap();

        let retrieved = store.get_transcript(&transcript.id).unwrap().unwrap();
        assert_eq!(retrieved.content, "Hello world");

        let stats = store.get_team_stats("team1").unwrap();
        assert_eq!(stats.transcript_count, 1);
        assert_eq!(stats.chunk_count, 1);
    }
}
