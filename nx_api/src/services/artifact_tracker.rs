//! 产物追踪器
//!
//! 用于在工作流 stage 执行前后拍 working_dir 的"快照"，
//! 然后 diff 出该 stage 新增 / 修改 / 删除的文件，
//! 写入 `artifacts` 表供前端"产物面板"展示。
//!
//! 设计原则：
//! - 忽略噪音目录（.git / node_modules / target / dist 等）
//! - 限制单个文件最大尺寸（>10MB 不算 hash，仅记 size）
//! - 限制总文件数（默认 10000，避免 monorepo 卡死）

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

/// 单个文件的元信息（足以判断是否变化）
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FileMeta {
    /// 文件大小
    pub size: u64,
    /// 内容 SHA-256（大文件为空字符串，仅靠 size 判断变化）
    pub sha256: String,
    /// 修改时间（unix epoch 秒）
    pub mtime: i64,
}

/// 工作目录快照
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct WorkdirSnapshot {
    /// 相对路径 → 元信息
    pub files: HashMap<String, FileMeta>,
}

/// 一次 stage 执行的产物 diff 结果
#[derive(Debug, Default, serde::Serialize)]
pub struct ArtifactDiff {
    pub added: Vec<ArtifactEntry>,
    pub modified: Vec<ArtifactEntry>,
    pub deleted: Vec<String>, // 只需路径
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ArtifactEntry {
    pub relative_path: String,
    pub size: u64,
    pub sha256: String,
}

/// 配置
#[derive(Debug, Clone)]
pub struct SnapshotOptions {
    /// 最大文件总数（超过后停止扫描）
    pub max_files: usize,
    /// 单文件最大字节数（超过的不计算 hash）
    pub max_hash_bytes: u64,
    /// 额外要忽略的相对路径前缀（除内置黑名单外）
    pub extra_ignored: Vec<String>,
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        Self {
            max_files: 10_000,
            max_hash_bytes: 10 * 1024 * 1024, // 10MB
            extra_ignored: Vec::new(),
        }
    }
}

/// 内置忽略目录（这些目录不被扫描）
const IGNORED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".cache",
    ".venv",
    "venv",
    "__pycache__",
    ".idea",
    ".vscode",
    ".DS_Store",
    "vendor",
    ".pytest_cache",
];

fn is_ignored_dir(name: &str, extra: &[String]) -> bool {
    if IGNORED_DIRS.contains(&name) {
        return true;
    }
    extra.iter().any(|p| p == name)
}

/// 拍快照：扫描 workdir 下所有文件，记录元信息
pub fn snapshot(workdir: &Path) -> WorkdirSnapshot {
    snapshot_with_options(workdir, &SnapshotOptions::default())
}

pub fn snapshot_with_options(workdir: &Path, opts: &SnapshotOptions) -> WorkdirSnapshot {
    let mut files = HashMap::new();
    if !workdir.exists() || !workdir.is_dir() {
        return WorkdirSnapshot { files };
    }

    let walker = WalkDir::new(workdir)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // 跳过忽略目录
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    return !is_ignored_dir(name, &opts.extra_ignored);
                }
            }
            true
        });

    for entry in walker.flatten() {
        if files.len() >= opts.max_files {
            tracing::warn!(
                "[ArtifactTracker] 文件数超过 {}，停止扫描 {:?}",
                opts.max_files,
                workdir
            );
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        if let Some(meta) = compute_meta(&entry, opts) {
            let rel = match entry.path().strip_prefix(workdir) {
                Ok(r) => r.to_string_lossy().to_string(),
                Err(_) => continue,
            };
            files.insert(rel, meta);
        }
    }

    WorkdirSnapshot { files }
}

fn compute_meta(entry: &DirEntry, opts: &SnapshotOptions) -> Option<FileMeta> {
    let metadata = entry.metadata().ok()?;
    let size = metadata.len();
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // 大文件不计算 hash（性能 + 内存保护）
    let sha256 = if size > opts.max_hash_bytes {
        String::new()
    } else {
        match compute_sha256(entry.path()) {
            Ok(h) => h,
            Err(_) => String::new(),
        }
    };

    Some(FileMeta {
        size,
        sha256,
        mtime,
    })
}

fn compute_sha256(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

/// 比较两个快照，输出 diff
///
/// 判断逻辑：
/// - 路径在 after 不在 before → added
/// - 路径在 before 不在 after → deleted
/// - 路径都在但 sha256 / size 变化 → modified
/// - 大文件（sha256 为空）只看 size + mtime
pub fn diff_snapshots(before: &WorkdirSnapshot, after: &WorkdirSnapshot) -> ArtifactDiff {
    let mut diff = ArtifactDiff::default();

    for (path, meta) in &after.files {
        match before.files.get(path) {
            None => diff.added.push(ArtifactEntry {
                relative_path: path.clone(),
                size: meta.size,
                sha256: meta.sha256.clone(),
            }),
            Some(old) => {
                let changed = if meta.sha256.is_empty() || old.sha256.is_empty() {
                    // 大文件 fallback: 看 size 或 mtime
                    old.size != meta.size || old.mtime != meta.mtime
                } else {
                    old.sha256 != meta.sha256
                };
                if changed {
                    diff.modified.push(ArtifactEntry {
                        relative_path: path.clone(),
                        size: meta.size,
                        sha256: meta.sha256.clone(),
                    });
                }
            }
        }
    }

    for path in before.files.keys() {
        if !after.files.contains_key(path) {
            diff.deleted.push(path.clone());
        }
    }

    diff
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn write_file(dir: &Path, name: &str, content: &[u8]) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(content).unwrap();
    }

    #[test]
    fn snapshot_empty_dir() {
        let dir = tmp_dir();
        let snap = snapshot(dir.path());
        assert!(snap.files.is_empty());
    }

    #[test]
    fn snapshot_picks_up_files() {
        let dir = tmp_dir();
        write_file(dir.path(), "a.txt", b"hello");
        write_file(dir.path(), "sub/b.txt", b"world");
        let snap = snapshot(dir.path());
        assert_eq!(snap.files.len(), 2);
        assert!(snap.files.contains_key("a.txt"));
        // 跨平台：sub 路径使用平台分隔符
        let sub_key = snap
            .files
            .keys()
            .find(|k| k.ends_with("b.txt"))
            .expect("b.txt should be in snapshot");
        assert!(sub_key.contains("b.txt"));
    }

    #[test]
    fn snapshot_ignores_node_modules_and_git() {
        let dir = tmp_dir();
        write_file(dir.path(), "src/main.rs", b"fn main(){}");
        write_file(dir.path(), "node_modules/foo/index.js", b"x");
        write_file(dir.path(), ".git/HEAD", b"ref:refs/heads/main");
        write_file(dir.path(), "target/debug/x", b"binary");
        let snap = snapshot(dir.path());
        // 只看到 src/main.rs
        assert_eq!(snap.files.len(), 1);
    }

    #[test]
    fn diff_added_modified_deleted() {
        let dir = tmp_dir();
        write_file(dir.path(), "keep.txt", b"a");
        write_file(dir.path(), "to_modify.txt", b"v1");
        write_file(dir.path(), "to_delete.txt", b"x");
        let before = snapshot(dir.path());

        // 模拟 stage 改文件
        write_file(dir.path(), "to_modify.txt", b"v2_changed");
        write_file(dir.path(), "new_file.txt", b"y");
        fs::remove_file(dir.path().join("to_delete.txt")).unwrap();

        let after = snapshot(dir.path());
        let diff = diff_snapshots(&before, &after);

        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].relative_path, "new_file.txt");
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.modified[0].relative_path, "to_modify.txt");
        assert_eq!(diff.deleted, vec!["to_delete.txt"]);
    }

    #[test]
    fn diff_no_changes() {
        let dir = tmp_dir();
        write_file(dir.path(), "a.txt", b"hello");
        let snap1 = snapshot(dir.path());
        let snap2 = snapshot(dir.path());
        let diff = diff_snapshots(&snap1, &snap2);
        assert!(diff.added.is_empty());
        assert!(diff.modified.is_empty());
        assert!(diff.deleted.is_empty());
    }

    #[test]
    fn snapshot_respects_max_files() {
        let dir = tmp_dir();
        for i in 0..50 {
            write_file(dir.path(), &format!("f{}.txt", i), b"x");
        }
        let opts = SnapshotOptions {
            max_files: 10,
            ..Default::default()
        };
        let snap = snapshot_with_options(dir.path(), &opts);
        // 最多 10 个（实际可能因目录扫描顺序略多，但绝不超过限制太多）
        assert!(snap.files.len() <= 11, "got {}", snap.files.len());
    }
}
