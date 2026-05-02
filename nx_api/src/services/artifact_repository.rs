//! 产物仓库
//!
//! 持久化每个 stage 执行后的文件 diff 结果（添加/修改/删除）。
//! 前端"产物面板"读这个表展示。

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

use crate::services::artifact_tracker::ArtifactDiff;

#[derive(Error, Debug)]
pub enum ArtifactRepositoryError {
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),
}

/// 产物记录（持久化形式）
#[derive(Debug, Clone, serde::Serialize)]
pub struct ArtifactRecord {
    pub id: String,
    pub execution_id: String,
    pub stage_name: Option<String>,
    pub relative_path: String,
    pub change_type: String, // added / modified / deleted
    pub size_bytes: i64,
    pub sha256: Option<String>,
    pub mime_type: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub struct SqliteArtifactRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteArtifactRepository {
    pub fn new(db_path: &Path) -> Result<Self, ArtifactRepositoryError> {
        let conn = Connection::open(db_path)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 批量记录一次 stage 的 diff 结果
    pub fn record_diff(
        &self,
        execution_id: &str,
        stage_name: Option<&str>,
        diff: &ArtifactDiff,
    ) -> Result<usize, ArtifactRepositoryError> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let now = Utc::now().to_rfc3339();
        let mut count = 0;

        // 内联函数：插入一条记录
        let insert = |tx: &rusqlite::Transaction,
                      path: &str,
                      change_type: &str,
                      size: u64,
                      sha: Option<&str>|
         -> Result<(), rusqlite::Error> {
            let mime = guess_mime_type(path);
            tx.execute(
                "INSERT INTO artifacts
                 (id, execution_id, stage_name, relative_path, change_type, size_bytes, sha256, mime_type, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    Uuid::new_v4().to_string(),
                    execution_id,
                    stage_name,
                    path,
                    change_type,
                    size as i64,
                    sha,
                    mime,
                    now
                ],
            )?;
            Ok(())
        };

        for entry in &diff.added {
            insert(
                &tx,
                &entry.relative_path,
                "added",
                entry.size,
                if entry.sha256.is_empty() {
                    None
                } else {
                    Some(&entry.sha256)
                },
            )?;
            count += 1;
        }
        for entry in &diff.modified {
            insert(
                &tx,
                &entry.relative_path,
                "modified",
                entry.size,
                if entry.sha256.is_empty() {
                    None
                } else {
                    Some(&entry.sha256)
                },
            )?;
            count += 1;
        }
        for path in &diff.deleted {
            insert(&tx, path, "deleted", 0, None)?;
            count += 1;
        }

        tx.commit()?;
        Ok(count)
    }

    /// 列出某次 execution 的全部产物（按 stage 顺序，最新在前）
    pub fn list_by_execution(
        &self,
        execution_id: &str,
    ) -> Result<Vec<ArtifactRecord>, ArtifactRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, execution_id, stage_name, relative_path, change_type, size_bytes,
                    sha256, mime_type, created_at
             FROM artifacts
             WHERE execution_id = ?1
             ORDER BY created_at ASC, relative_path ASC",
        )?;

        let rows = stmt.query_map(params![execution_id], Self::row_to_record)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 找出某次 execution 中指定路径的最新一条记录
    pub fn find_by_path(
        &self,
        execution_id: &str,
        relative_path: &str,
    ) -> Result<Option<ArtifactRecord>, ArtifactRepositoryError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, execution_id, stage_name, relative_path, change_type, size_bytes,
                    sha256, mime_type, created_at
             FROM artifacts
             WHERE execution_id = ?1 AND relative_path = ?2
             ORDER BY created_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![execution_id, relative_path], Self::row_to_record)?;
        match rows.next() {
            Some(Ok(r)) => Ok(Some(r)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    fn row_to_record(row: &rusqlite::Row) -> rusqlite::Result<ArtifactRecord> {
        let created_at: String = row.get(8)?;
        Ok(ArtifactRecord {
            id: row.get(0)?,
            execution_id: row.get(1)?,
            stage_name: row.get(2)?,
            relative_path: row.get(3)?,
            change_type: row.get(4)?,
            size_bytes: row.get(5)?,
            sha256: row.get(6)?,
            mime_type: row.get(7)?,
            created_at: DateTime::parse_from_rfc3339(&created_at)
                .map(|t| t.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }
}

/// 简单的 mime 推断（覆盖最常见的几类，未知返回 None）
fn guess_mime_type(path: &str) -> Option<String> {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    let mime = match ext.as_str() {
        "md" | "markdown" => "text/markdown",
        "txt" => "text/plain",
        "json" => "application/json",
        "yaml" | "yml" => "application/yaml",
        "toml" => "application/toml",
        "rs" => "text/x-rust",
        "ts" | "tsx" => "text/typescript",
        "js" | "jsx" | "mjs" => "text/javascript",
        "py" => "text/x-python",
        "go" => "text/x-go",
        "java" => "text/x-java",
        "c" | "h" => "text/x-c",
        "cpp" | "hpp" | "cc" => "text/x-c++",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "scss" | "sass" => "text/x-scss",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "sql" => "application/sql",
        "sh" | "bash" => "text/x-shellscript",
        _ => return None,
    };
    Some(mime.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::artifact_tracker::{ArtifactDiff, ArtifactEntry};

    fn tmp_db() -> SqliteArtifactRepository {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        // 让 dir 活到测试结束（leak 是测试场景下可接受的代价）
        Box::leak(Box::new(dir));
        crate::migrations::run_all(path.to_str().unwrap()).unwrap();
        SqliteArtifactRepository::new(&path).unwrap()
    }

    #[test]
    fn record_and_list() {
        let repo = tmp_db();
        let diff = ArtifactDiff {
            added: vec![ArtifactEntry {
                relative_path: "src/main.rs".to_string(),
                size: 100,
                sha256: "abc".to_string(),
            }],
            modified: vec![],
            deleted: vec!["old.txt".to_string()],
        };
        let n = repo.record_diff("exec-1", Some("backend"), &diff).unwrap();
        assert_eq!(n, 2);

        let list = repo.list_by_execution("exec-1").unwrap();
        assert_eq!(list.len(), 2);
        let added = list.iter().find(|r| r.change_type == "added").unwrap();
        assert_eq!(added.relative_path, "src/main.rs");
        assert_eq!(added.mime_type.as_deref(), Some("text/x-rust"));
    }

    #[test]
    fn isolation_per_execution() {
        let repo = tmp_db();
        let diff_a = ArtifactDiff {
            added: vec![ArtifactEntry {
                relative_path: "a.txt".into(),
                size: 1,
                sha256: "x".into(),
            }],
            ..Default::default()
        };
        let diff_b = ArtifactDiff {
            added: vec![ArtifactEntry {
                relative_path: "b.txt".into(),
                size: 1,
                sha256: "y".into(),
            }],
            ..Default::default()
        };
        repo.record_diff("exec-A", Some("s1"), &diff_a).unwrap();
        repo.record_diff("exec-B", Some("s1"), &diff_b).unwrap();
        assert_eq!(repo.list_by_execution("exec-A").unwrap().len(), 1);
        assert_eq!(repo.list_by_execution("exec-B").unwrap().len(), 1);
    }

    #[test]
    fn find_by_path_returns_latest() {
        let repo = tmp_db();
        let diff1 = ArtifactDiff {
            added: vec![ArtifactEntry {
                relative_path: "f.txt".into(),
                size: 1,
                sha256: "v1".into(),
            }],
            ..Default::default()
        };
        repo.record_diff("e1", Some("s1"), &diff1).unwrap();

        // 第二次：modified
        std::thread::sleep(std::time::Duration::from_millis(20));
        let diff2 = ArtifactDiff {
            modified: vec![ArtifactEntry {
                relative_path: "f.txt".into(),
                size: 2,
                sha256: "v2".into(),
            }],
            ..Default::default()
        };
        repo.record_diff("e1", Some("s2"), &diff2).unwrap();

        let found = repo.find_by_path("e1", "f.txt").unwrap().unwrap();
        assert_eq!(found.change_type, "modified");
        assert_eq!(found.sha256.as_deref(), Some("v2"));
    }
}
