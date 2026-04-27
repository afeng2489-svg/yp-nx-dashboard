//! 技能仓储层
//!
//! 提供技能的持久化存储，支持 SQLite 数据库操作。

use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::Path;
use std::sync::Arc;

use crate::models::skill::{CreateSkillRequest, SkillParameter, SkillRecord, UpdateSkillRequest};

/// SQLite 技能仓储错误
#[derive(Debug, thiserror::Error)]
pub enum SkillRepositoryError {
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("技能不存在: {0}")]
    NotFound(String),

    #[error("技能已存在: {0}")]
    AlreadyExists(String),

    #[error("JSON 解析错误: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// 技能仓储 trait - 支持内存和持久化实现
pub trait SkillRepository: Send + Sync {
    fn create(&self, req: CreateSkillRequest) -> Result<SkillRecord, SkillRepositoryError>;
    fn update(
        &self,
        id: &str,
        req: UpdateSkillRequest,
    ) -> Result<SkillRecord, SkillRepositoryError>;
    fn delete(&self, id: &str) -> Result<(), SkillRepositoryError>;
    fn get(&self, id: &str) -> Result<Option<SkillRecord>, SkillRepositoryError>;
    fn list(&self) -> Result<Vec<SkillRecord>, SkillRepositoryError>;
    fn list_by_category(&self, category: &str) -> Result<Vec<SkillRecord>, SkillRepositoryError>;
    fn list_by_tag(&self, tag: &str) -> Result<Vec<SkillRecord>, SkillRepositoryError>;
    fn search(&self, query: &str) -> Result<Vec<SkillRecord>, SkillRepositoryError>;
    fn exists(&self, id: &str) -> Result<bool, SkillRepositoryError>;
    fn init_preset(&self, req: CreateSkillRequest) -> Result<(), SkillRepositoryError>;
}

/// SQLite 技能仓储实现
#[derive(Clone)]
pub struct SqliteSkillRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteSkillRepository {
    /// 创建新的仓储实例
    pub fn new(db_path: &str) -> Result<Self, SkillRepositoryError> {
        let conn = Connection::open(Path::new(db_path))?;
        let repo = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        repo.init_table()?;
        Ok(repo)
    }

    /// 初始化表结构
    fn init_table(&self) -> Result<(), SkillRepositoryError> {
        let conn = self.conn.lock();
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                category TEXT NOT NULL,
                version TEXT DEFAULT '1.0.0',
                author TEXT,
                tags TEXT DEFAULT '[]',
                parameters TEXT DEFAULT '[]',
                code TEXT,
                is_preset INTEGER DEFAULT 0,
                enabled INTEGER DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
            [],
        )?;

        // 创建索引
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_skills_category ON skills(category)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_skills_is_preset ON skills(is_preset)",
            [],
        )?;

        Ok(())
    }

    /// 检查技能是否存在
    pub fn exists(&self, id: &str) -> Result<bool, SkillRepositoryError> {
        let conn = self.conn.lock();
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM skills WHERE id = ?",
            params![id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// 将 CreateSkillRequest 转换为 SkillRecord
    fn req_to_record(req: CreateSkillRequest) -> SkillRecord {
        let now = Utc::now();
        SkillRecord {
            id: req.id,
            name: req.name,
            description: req.description,
            category: req.category,
            version: req.version.unwrap_or_else(|| "1.0.0".to_string()),
            author: req.author,
            tags: req.tags.unwrap_or_default(),
            parameters: req.parameters.unwrap_or_default(),
            code: req.code,
            is_preset: false,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }
}

impl SkillRepository for SqliteSkillRepository {
    fn create(&self, req: CreateSkillRequest) -> Result<SkillRecord, SkillRepositoryError> {
        // 检查是否已存在
        if self.exists(&req.id)? {
            return Err(SkillRepositoryError::AlreadyExists(req.id));
        }

        let record = Self::req_to_record(req);
        let conn = self.conn.lock();

        conn.execute(
            r#"
            INSERT INTO skills (id, name, description, category, version, author, tags, parameters, code, is_preset, enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                record.id,
                record.name,
                record.description,
                record.category,
                record.version,
                record.author,
                serde_json::to_string(&record.tags)?,
                serde_json::to_string(&record.parameters)?,
                record.code,
                record.is_preset as i32,
                record.enabled as i32,
                record.created_at.to_rfc3339(),
                record.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(record)
    }

    fn update(
        &self,
        id: &str,
        req: UpdateSkillRequest,
    ) -> Result<SkillRecord, SkillRepositoryError> {
        let conn = self.conn.lock();

        // 先获取现有记录
        let existing: SkillRecord = {
            let mut stmt = conn.prepare(
                "SELECT id, name, description, category, version, author, tags, parameters, code, is_preset, enabled, created_at, updated_at FROM skills WHERE id = ?"
            )?;
            let row = stmt
                .query_row(params![id], |row| {
                    Ok(SkillRecord::from_row(row).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Null,
                            Box::new(e),
                        )
                    })?)
                })
                .map_err(|_| SkillRepositoryError::NotFound(id.to_string()))?;
            row
        };

        // 构建更新后的记录
        let updated = SkillRecord {
            id: existing.id.clone(),
            name: req.name.unwrap_or(existing.name),
            description: req.description.unwrap_or(existing.description),
            category: req.category.unwrap_or(existing.category),
            version: req.version.unwrap_or(existing.version),
            author: req.author.or(existing.author),
            tags: req.tags.unwrap_or(existing.tags),
            parameters: req.parameters.unwrap_or(existing.parameters),
            code: req.code.or(existing.code),
            is_preset: existing.is_preset,
            enabled: req.enabled.unwrap_or(existing.enabled),
            created_at: existing.created_at,
            updated_at: Utc::now(),
        };

        conn.execute(
            r#"
            UPDATE skills SET
                name = ?,
                description = ?,
                category = ?,
                version = ?,
                author = ?,
                tags = ?,
                parameters = ?,
                code = ?,
                enabled = ?,
                updated_at = ?
            WHERE id = ?
            "#,
            params![
                updated.name,
                updated.description,
                updated.category,
                updated.version,
                updated.author,
                serde_json::to_string(&updated.tags)?,
                serde_json::to_string(&updated.parameters)?,
                updated.code,
                updated.enabled as i32,
                updated.updated_at.to_rfc3339(),
                id,
            ],
        )?;

        Ok(updated)
    }

    fn delete(&self, id: &str) -> Result<(), SkillRepositoryError> {
        let conn = self.conn.lock();
        let affected = conn.execute("DELETE FROM skills WHERE id = ?", params![id])?;
        if affected == 0 {
            return Err(SkillRepositoryError::NotFound(id.to_string()));
        }
        Ok(())
    }

    fn get(&self, id: &str) -> Result<Option<SkillRecord>, SkillRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, category, version, author, tags, parameters, code, is_preset, enabled, created_at, updated_at FROM skills WHERE id = ?"
        )?;

        let result = stmt.query_row(params![id], |row| {
            SkillRecord::from_row(row).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Null,
                    Box::new(e),
                )
            })
        });

        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(SkillRepositoryError::Database(e)),
        }
    }

    fn list(&self) -> Result<Vec<SkillRecord>, SkillRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, category, version, author, tags, parameters, code, is_preset, enabled, created_at, updated_at FROM skills ORDER BY name"
        )?;

        let records = stmt
            .query_map([], |row| {
                SkillRecord::from_row(row).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Null,
                        Box::new(e),
                    )
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(records)
    }

    fn list_by_category(&self, category: &str) -> Result<Vec<SkillRecord>, SkillRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, category, version, author, tags, parameters, code, is_preset, enabled, created_at, updated_at FROM skills WHERE category = ? ORDER BY name"
        )?;

        let records = stmt
            .query_map(params![category], |row| {
                SkillRecord::from_row(row).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Null,
                        Box::new(e),
                    )
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(records)
    }

    fn list_by_tag(&self, tag: &str) -> Result<Vec<SkillRecord>, SkillRepositoryError> {
        let conn = self.conn.lock();
        let pattern = format!("%\"{}%\"", tag);
        let mut stmt = conn.prepare(
            "SELECT id, name, description, category, version, author, tags, parameters, code, is_preset, enabled, created_at, updated_at FROM skills WHERE tags LIKE ? ORDER BY name"
        )?;

        let records = stmt
            .query_map(params![pattern], |row| {
                SkillRecord::from_row(row).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Null,
                        Box::new(e),
                    )
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(records)
    }

    fn search(&self, query: &str) -> Result<Vec<SkillRecord>, SkillRepositoryError> {
        let conn = self.conn.lock();
        let pattern = format!("%{}%", query.to_lowercase());
        let mut stmt = conn.prepare(
            r#"
            SELECT id, name, description, category, version, author, tags, parameters, code, is_preset, enabled, created_at, updated_at
            FROM skills
            WHERE LOWER(name) LIKE ? OR LOWER(description) LIKE ? OR LOWER(tags) LIKE ?
            ORDER BY name
            "#
        )?;

        let records = stmt
            .query_map(params![&pattern, &pattern, &pattern], |row| {
                SkillRecord::from_row(row).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Null,
                        Box::new(e),
                    )
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(records)
    }

    fn exists(&self, id: &str) -> Result<bool, SkillRepositoryError> {
        SqliteSkillRepository::exists(self, id)
    }

    fn init_preset(&self, req: CreateSkillRequest) -> Result<(), SkillRepositoryError> {
        // 预设技能如果已存在就跳过
        if self.exists(&req.id)? {
            return Ok(());
        }

        let mut record = Self::req_to_record(req);
        record.is_preset = true;

        let conn = self.conn.lock();
        conn.execute(
            r#"
            INSERT OR IGNORE INTO skills (id, name, description, category, version, author, tags, parameters, code, is_preset, enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                record.id,
                record.name,
                record.description,
                record.category,
                record.version,
                record.author,
                serde_json::to_string(&record.tags)?,
                serde_json::to_string(&record.parameters)?,
                record.code,
                record.is_preset as i32,
                record.enabled as i32,
                record.created_at.to_rfc3339(),
                record.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_repo() -> SqliteSkillRepository {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        SqliteSkillRepository::new(db_path.to_str().unwrap()).unwrap()
    }

    #[test]
    fn test_create_and_get() {
        let repo = create_test_repo();

        let req = CreateSkillRequest {
            id: "test-skill-1".to_string(),
            name: "测试技能".to_string(),
            description: "这是一个测试技能".to_string(),
            category: "development".to_string(),
            version: Some("1.0.0".to_string()),
            author: Some("test".to_string()),
            tags: Some(vec!["test".to_string()]),
            parameters: None,
            code: Some("echo hello".to_string()),
        };

        let created = repo.create(req).unwrap();
        assert_eq!(created.id, "test-skill-1");
        assert_eq!(created.name, "测试技能");
        assert!(!created.is_preset);

        let retrieved = repo.get("test-skill-1").unwrap().unwrap();
        assert_eq!(retrieved.id, created.id);
    }

    #[test]
    fn test_update() {
        let repo = create_test_repo();

        repo.create(CreateSkillRequest {
            id: "test-skill".to_string(),
            name: "原始名称".to_string(),
            description: "原始描述".to_string(),
            category: "development".to_string(),
            version: None,
            author: None,
            tags: None,
            parameters: None,
            code: None,
        })
        .unwrap();

        let updated = repo
            .update(
                "test-skill",
                UpdateSkillRequest {
                    name: Some("新名称".to_string()),
                    description: None,
                    category: None,
                    version: None,
                    author: None,
                    tags: None,
                    parameters: None,
                    code: None,
                    enabled: None,
                },
            )
            .unwrap();

        assert_eq!(updated.name, "新名称");
        assert_eq!(updated.description, "原始描述");
    }

    #[test]
    fn test_delete() {
        let repo = create_test_repo();

        repo.create(CreateSkillRequest {
            id: "to-delete".to_string(),
            name: "删除测试".to_string(),
            description: "".to_string(),
            category: "development".to_string(),
            version: None,
            author: None,
            tags: None,
            parameters: None,
            code: None,
        })
        .unwrap();

        repo.delete("to-delete").unwrap();
        assert!(repo.get("to-delete").unwrap().is_none());
    }

    #[test]
    fn test_duplicate_create() {
        let repo = create_test_repo();

        repo.create(CreateSkillRequest {
            id: "dup".to_string(),
            name: "dup".to_string(),
            description: "".to_string(),
            category: "development".to_string(),
            version: None,
            author: None,
            tags: None,
            parameters: None,
            code: None,
        })
        .unwrap();

        let result = repo.create(CreateSkillRequest {
            id: "dup".to_string(),
            name: "dup2".to_string(),
            description: "".to_string(),
            category: "development".to_string(),
            version: None,
            author: None,
            tags: None,
            parameters: None,
            code: None,
        });

        assert!(result.is_err());
    }
}
