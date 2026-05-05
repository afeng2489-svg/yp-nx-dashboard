//! 知识库仓储层
//!
//! SQLite CRUD：knowledge_bases, kb_documents, kb_chunks

use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),
}

// ── 数据模型 ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeBase {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub embedding_provider: String,
    pub embedding_model: String,
    pub embedding_dimension: i64,
    pub chunk_size: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KbDocument {
    pub id: String,
    pub knowledge_base_id: String,
    pub filename: String,
    pub content_type: String,
    pub file_size: i64,
    pub chunk_count: i64,
    pub status: String,
    pub error: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct KbChunk {
    pub id: String,
    pub document_id: String,
    pub knowledge_base_id: String,
    pub chunk_index: usize,
    pub content: String,
    pub token_count: i64,
    pub embedding: Option<Vec<f32>>,
    pub created_at: String,
}

pub struct KnowledgeRepository {
    conn: Mutex<Connection>,
}

impl KnowledgeRepository {
    pub fn new(db_path: &Path) -> Result<Self, RepositoryError> {
        let conn = Connection::open(db_path)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

// ── KnowledgeBase CRUD ──

impl KnowledgeRepository {
    pub fn create_knowledge_base(&self, kb: &KnowledgeBase) -> Result<(), RepositoryError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO knowledge_bases (id, name, description, embedding_provider, embedding_model, embedding_dimension, chunk_size, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![kb.id, kb.name, kb.description, kb.embedding_provider, kb.embedding_model, kb.embedding_dimension, kb.chunk_size, kb.created_at, kb.updated_at],
        )?;
        Ok(())
    }

    pub fn list_knowledge_bases(&self) -> Result<Vec<KnowledgeBase>, RepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, embedding_provider, embedding_model, embedding_dimension, chunk_size, created_at, updated_at
             FROM knowledge_bases ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(KnowledgeBase {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                embedding_provider: row.get(3)?,
                embedding_model: row.get(4)?,
                embedding_dimension: row.get(5)?,
                chunk_size: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn get_knowledge_base(&self, id: &str) -> Result<Option<KnowledgeBase>, RepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, embedding_provider, embedding_model, embedding_dimension, chunk_size, created_at, updated_at
             FROM knowledge_bases WHERE id = ?1",
        )?;
        match stmt.query_row(params![id], |row| {
            Ok(KnowledgeBase {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                embedding_provider: row.get(3)?,
                embedding_model: row.get(4)?,
                embedding_dimension: row.get(5)?,
                chunk_size: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        }) {
            Ok(kb) => Ok(Some(kb)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn delete_knowledge_base(&self, id: &str) -> Result<(), RepositoryError> {
        let conn = self.conn.lock();
        // CASCADE 会自动删除 documents 和 chunks
        conn.execute("DELETE FROM knowledge_bases WHERE id = ?1", params![id])?;
        Ok(())
    }
}

// ── Document CRUD ──

impl KnowledgeRepository {
    pub fn insert_document(&self, doc: &KbDocument) -> Result<(), RepositoryError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO kb_documents (id, knowledge_base_id, filename, content_type, file_size, chunk_count, status, error, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![doc.id, doc.knowledge_base_id, doc.filename, doc.content_type, doc.file_size, doc.chunk_count, doc.status, doc.error, doc.created_at],
        )?;
        Ok(())
    }

    pub fn update_document_status(
        &self,
        id: &str,
        status: &str,
        chunk_count: i64,
        error: Option<&str>,
    ) -> Result<(), RepositoryError> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE kb_documents SET status = ?1, chunk_count = ?2, error = ?3 WHERE id = ?4",
            params![status, chunk_count, error, id],
        )?;
        Ok(())
    }

    pub fn list_documents(&self, kb_id: &str) -> Result<Vec<KbDocument>, RepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, knowledge_base_id, filename, content_type, file_size, chunk_count, status, error, created_at
             FROM kb_documents WHERE knowledge_base_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![kb_id], |row| {
            Ok(KbDocument {
                id: row.get(0)?,
                knowledge_base_id: row.get(1)?,
                filename: row.get(2)?,
                content_type: row.get(3)?,
                file_size: row.get(4)?,
                chunk_count: row.get(5)?,
                status: row.get(6)?,
                error: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn delete_document(&self, id: &str) -> Result<(), RepositoryError> {
        let conn = self.conn.lock();
        // CASCADE 会自动删除 chunks
        conn.execute("DELETE FROM kb_documents WHERE id = ?1", params![id])?;
        Ok(())
    }
}

// ── Chunk CRUD ──

impl KnowledgeRepository {
    pub fn insert_chunks_batch(&self, chunks: &[KbChunk]) -> Result<(), RepositoryError> {
        let conn = self.conn.lock();
        let tx = conn.unchecked_transaction()?;
        for chunk in chunks {
            let embedding_blob = chunk.embedding.as_ref().map(|v| encode_vector(v));
            tx.execute(
                "INSERT INTO kb_chunks (id, document_id, knowledge_base_id, chunk_index, content, token_count, embedding, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![chunk.id, chunk.document_id, chunk.knowledge_base_id, chunk.chunk_index as i64, chunk.content, chunk.token_count, embedding_blob, chunk.created_at],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// 获取指定知识库的所有 chunks（含 embedding）
    pub fn get_chunks_with_embeddings(&self, kb_id: &str) -> Result<Vec<KbChunk>, RepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, document_id, knowledge_base_id, chunk_index, content, token_count, embedding, created_at
             FROM kb_chunks WHERE knowledge_base_id = ?1 ORDER BY chunk_index ASC",
        )?;
        let rows = stmt.query_map(params![kb_id], |row| {
            let embedding_blob: Option<Vec<u8>> = row.get(6)?;
            Ok(KbChunk {
                id: row.get(0)?,
                document_id: row.get(1)?,
                knowledge_base_id: row.get(2)?,
                chunk_index: row.get::<_, i64>(3)? as usize,
                content: row.get(4)?,
                token_count: row.get(5)?,
                embedding: embedding_blob.as_ref().map(|b| decode_vector(b)),
                created_at: row.get(7)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// 更新单个 chunk 的 embedding
    pub fn update_chunk_embedding(
        &self,
        chunk_id: &str,
        embedding: &[f32],
    ) -> Result<(), RepositoryError> {
        let conn = self.conn.lock();
        let blob = encode_vector(embedding);
        conn.execute(
            "UPDATE kb_chunks SET embedding = ?1 WHERE id = ?2",
            params![blob, chunk_id],
        )?;
        Ok(())
    }

    /// 全文关键词搜索（无 embedding 时的降级方案）
    pub fn search_by_keyword(
        &self,
        kb_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<KbChunk>, RepositoryError> {
        let conn = self.conn.lock();
        let pattern = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
        let mut stmt = conn.prepare(
            "SELECT id, document_id, knowledge_base_id, chunk_index, content, token_count, embedding, created_at
             FROM kb_chunks WHERE knowledge_base_id = ?1 AND content LIKE ?2 ESCAPE '\\'
             LIMIT ?3",
        )?;
        let chunks = stmt
            .query_map(params![kb_id, pattern, top_k as i64], |row| {
                let blob: Option<Vec<u8>> = row.get(6)?;
                Ok(KbChunk {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    knowledge_base_id: row.get(2)?,
                    chunk_index: row.get::<_, i64>(3)? as usize,
                    content: row.get(4)?,
                    token_count: row.get(5)?,
                    embedding: blob.map(|b| decode_vector(&b)),
                    created_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(chunks)
    }
}

// ── 向量编解码（与 nx_memory 同模式） ──

fn encode_vector(v: &[f32]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(v.len() * 4);
    for &f in v {
        buf.extend_from_slice(&f.to_le_bytes());
    }
    buf
}

fn decode_vector(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

// ── App Settings ──

impl KnowledgeRepository {
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, RepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        Ok(rows.next()?.map(|r| r.get(0)).transpose()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), RepositoryError> {
        let conn = self.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO app_settings(key, value, updated_at) VALUES(?1,?2,?3)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
            params![key, value, now],
        )?;
        Ok(())
    }
}
