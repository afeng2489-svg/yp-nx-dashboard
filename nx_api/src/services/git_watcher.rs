//! Git 集成的 StageWatcher 实现
//!
//! 每个 stage 完成后自动 git add + commit，
//! workflow 开始时创建 ai-exec-{execution_id} 分支。

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::{Mutex, RwLock};

/// Git 集成 watcher
pub struct GitStageWatcher {
    /// 当前 workspace path（动态共享）
    workspace_path: Arc<RwLock<Option<String>>>,
    /// workflow 开始前的分支名
    initial_branch: Mutex<Option<String>>,
    /// 执行分支名 ai-exec-{execution_id}
    branch_name: Mutex<Option<String>>,
}

impl GitStageWatcher {
    pub fn new(workspace_path: Arc<RwLock<Option<String>>>) -> Self {
        Self {
            workspace_path,
            initial_branch: Mutex::new(None),
            branch_name: Mutex::new(None),
        }
    }

    fn current_workdir(&self) -> Option<PathBuf> {
        self.workspace_path.read().clone().map(PathBuf::from)
    }

    /// 执行 git 命令，返回 stdout
    fn run_git(workdir: &PathBuf, args: &[&str]) -> Result<String, String> {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(workdir)
            .output()
            .map_err(|e| format!("git 命令执行失败: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git {} 失败: {}", args.join(" "), stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// 检查目录是否是 git 仓库
    fn is_git_repo(workdir: &std::path::Path) -> bool {
        workdir.join(".git").exists()
    }

    /// 获取当前分支名
    fn get_current_branch(workdir: &PathBuf) -> Option<String> {
        Self::run_git(workdir, &["rev-parse", "--abbrev-ref", "HEAD"]).ok()
    }

    /// 创建并切换到执行分支
    fn create_exec_branch(workdir: &PathBuf, branch_name: &str) -> Result<(), String> {
        // 先检查分支是否已存在
        let exists = Self::run_git(workdir, &["rev-parse", "--verify", branch_name]).is_ok();
        if exists {
            Self::run_git(workdir, &["checkout", branch_name])?;
        } else {
            Self::run_git(workdir, &["checkout", "-b", branch_name])?;
        }
        Ok(())
    }

    /// 确保目录是 git 仓库；若不是，则自动 `git init` 并把现有文件提交为「执行前基线」，
    /// 这样后续才有可回滚的起点。返回 true 表示仓库可用。
    fn ensure_git_repo(workdir: &PathBuf) -> bool {
        if Self::is_git_repo(workdir) {
            return true;
        }
        if let Err(e) = Self::run_git(workdir, &["init"]) {
            tracing::warn!("[GitWatcher] 自动 git init 失败: {}", e);
            return false;
        }
        tracing::info!("[GitWatcher] 已自动初始化 git 仓库（非 git 工作区）: {:?}", workdir);
        // 暂存现有文件并提交执行前基线
        if let Err(e) = Self::run_git(workdir, &["add", "-A"]) {
            tracing::warn!("[GitWatcher] 基线 git add 失败: {}", e);
            return true; // 仓库已建好，后续仍可继续
        }
        let has_changes = Self::run_git(workdir, &["diff", "--cached", "--quiet"]).is_err();
        if has_changes {
            if let Err(e) = Self::git_commit(workdir, "factory: 初始化仓库（执行前基线）") {
                tracing::warn!("[GitWatcher] 基线 commit 失败: {}", e);
            }
        }
        true
    }

    /// 当仓库未配置 user.name / user.email 时，回退一个工厂身份；
    /// 已配置则返回空，避免覆盖用户自己的身份。
    fn identity_args(workdir: &PathBuf) -> Vec<String> {
        let has_name = Self::run_git(workdir, &["config", "user.name"])
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        let has_email = Self::run_git(workdir, &["config", "user.email"])
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        if has_name && has_email {
            Vec::new()
        } else {
            vec![
                "-c".into(),
                "user.name=NexusFlow Factory".into(),
                "-c".into(),
                "user.email=factory@nexusflow.local".into(),
            ]
        }
    }

    /// 提交（缺身份时自动回退工厂身份，保证自动初始化的仓库也能 commit）
    fn git_commit(workdir: &PathBuf, message: &str) -> Result<String, String> {
        let mut args = Self::identity_args(workdir);
        args.push("commit".into());
        args.push("-m".into());
        args.push(message.into());
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        Self::run_git(workdir, &arg_refs)
    }
}

impl nexus_workflow::watcher::StageWatcher for GitStageWatcher {
    fn before_stage(&self, execution_id: &str, _stage_name: &str) {
        let Some(workdir) = self.current_workdir() else {
            return;
        };

        // 非 git 工作区自动初始化并落基线，保证后续可回滚/审批
        if !Self::ensure_git_repo(&workdir) {
            return;
        }

        let mut initial_branch = self.initial_branch.lock();
        let mut branch_name = self.branch_name.lock();

        // 只在第一次调用时创建分支
        if initial_branch.is_none() {
            let current = Self::get_current_branch(&workdir);
            *initial_branch = current;
            let branch = format!("ai-exec-{}", execution_id);
            match Self::create_exec_branch(&workdir, &branch) {
                Ok(()) => {
                    tracing::info!("[GitWatcher] 创建执行分支: {}", branch);
                    *branch_name = Some(branch);
                }
                Err(e) => {
                    tracing::warn!("[GitWatcher] 创建分支失败: {}", e);
                }
            }
        }
    }

    fn after_stage(&self, _execution_id: &str, stage_name: &str) {
        let Some(workdir) = self.current_workdir() else {
            return;
        };

        if !Self::is_git_repo(&workdir) {
            return;
        }

        let branch = self.branch_name.lock();
        if branch.is_none() {
            return;
        }
        drop(branch);

        // git add -A
        if let Err(e) = Self::run_git(&workdir, &["add", "-A"]) {
            tracing::warn!("[GitWatcher] git add 失败: {}", e);
            return;
        }

        // 检查是否有暂存的变更
        let status = Self::run_git(&workdir, &["diff", "--cached", "--quiet"]);
        if status.is_ok() {
            // --quiet 成功 = 没有暂存的变更，跳过 commit
            tracing::debug!("[GitWatcher] {} 无变更，跳过 commit", stage_name);
            return;
        }

        // git commit
        let message = format!("stage: {}", stage_name);
        match Self::git_commit(&workdir, &message) {
            Ok(output) => {
                tracing::info!("[GitWatcher] commit: {}", message);
                if !output.is_empty() {
                    tracing::debug!("[GitWatcher] {}", output);
                }
            }
            Err(e) => {
                tracing::warn!("[GitWatcher] commit 失败: {}", e);
            }
        }
    }
}

/// Git 回滚和 PR 描述服务
pub struct GitService {
    workspace_path: Arc<RwLock<Option<String>>>,
}

impl GitService {
    pub fn new(workspace_path: Arc<RwLock<Option<String>>>) -> Self {
        Self { workspace_path }
    }

    fn current_workdir(&self) -> Option<PathBuf> {
        self.workspace_path.read().clone().map(PathBuf::from)
    }

    fn run_git(workdir: &PathBuf, args: &[&str]) -> Result<String, String> {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(workdir)
            .output()
            .map_err(|e| format!("git 命令执行失败: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git {} 失败: {}", args.join(" "), stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// 回滚执行：切回原分支，删除执行分支
    pub fn rollback_revert(&self, initial_branch: &str, exec_branch: &str) -> Result<(), String> {
        let workdir = self
            .current_workdir()
            .ok_or_else(|| "没有设置工作区路径".to_string())?;

        Self::run_git(&workdir, &["checkout", initial_branch])?;
        Self::run_git(&workdir, &["branch", "-D", exec_branch])?;
        tracing::info!(
            "[GitService] 回滚完成: 切回 {} 并删除 {}",
            initial_branch,
            exec_branch
        );
        Ok(())
    }

    /// 保留当前分支，不做操作
    pub fn rollback_keep(&self, _initial_branch: &str, _exec_branch: &str) -> Result<(), String> {
        tracing::info!("[GitService] 保留当前分支");
        Ok(())
    }

    /// 创建 fix 分支，切回原分支
    pub fn rollback_branch(
        &self,
        execution_id: &str,
        initial_branch: &str,
        exec_branch: &str,
    ) -> Result<(), String> {
        let workdir = self
            .current_workdir()
            .ok_or_else(|| "没有设置工作区路径".to_string())?;

        let fix_branch = format!("fix/{}", execution_id);
        Self::run_git(&workdir, &["branch", "-m", exec_branch, &fix_branch])?;
        Self::run_git(&workdir, &["checkout", initial_branch])?;
        tracing::info!(
            "[GitService] 创建 fix 分支: {} 并切回 {}",
            fix_branch,
            initial_branch
        );
        Ok(())
    }

    /// 采纳改动：切回原分支，把执行分支合并进来，并删除执行分支
    pub fn rollback_merge(&self, initial_branch: &str, exec_branch: &str) -> Result<(), String> {
        let workdir = self
            .current_workdir()
            .ok_or_else(|| "没有设置工作区路径".to_string())?;

        Self::run_git(&workdir, &["checkout", initial_branch])?;
        // 合并执行分支（FF 优先；需要合并提交时带上身份兜底）
        let mut merge_args = Self::identity_args(&workdir);
        merge_args.push("merge".into());
        merge_args.push("--no-edit".into());
        merge_args.push(exec_branch.into());
        let refs: Vec<&str> = merge_args.iter().map(|s| s.as_str()).collect();
        Self::run_git(&workdir, &refs)?;
        Self::run_git(&workdir, &["branch", "-D", exec_branch])?;
        tracing::info!(
            "[GitService] 采纳合并: {} → {} 并删除执行分支",
            exec_branch,
            initial_branch
        );
        Ok(())
    }

    /// 当仓库未配置 user.name / user.email 时回退一个工厂身份，避免合并提交失败
    fn identity_args(workdir: &PathBuf) -> Vec<String> {
        let has_name = Self::run_git(workdir, &["config", "user.name"])
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        let has_email = Self::run_git(workdir, &["config", "user.email"])
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        if has_name && has_email {
            Vec::new()
        } else {
            vec![
                "-c".into(),
                "user.name=NexusFlow Factory".into(),
                "-c".into(),
                "user.email=factory@nexusflow.local".into(),
            ]
        }
    }

    /// 获取执行分支的 commit 列表
    pub fn list_commits(
        &self,
        initial_branch: &str,
        exec_branch: &str,
    ) -> Result<Vec<CommitInfo>, String> {
        let workdir = self
            .current_workdir()
            .ok_or_else(|| "没有设置工作区路径".to_string())?;

        let range = format!("{}..{}", initial_branch, exec_branch);
        let log_output = Self::run_git(&workdir, &["log", &range, "--format=%H|%s|%ai"])?;

        let mut commits = Vec::new();
        for line in log_output.lines() {
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() == 3 {
                let hash = parts[0].to_string();
                let short_hash = if hash.len() >= 7 {
                    hash[..7].to_string()
                } else {
                    hash.clone()
                };
                let message = parts[1].to_string();
                let timestamp = parts[2].to_string();

                // 获取该 commit 的文件变更数
                let changed_files = Self::run_git(
                    &workdir,
                    &["diff-tree", "--no-commit-id", "--name-only", "-r", &hash],
                )
                .unwrap_or_default()
                .lines()
                .count();

                commits.push(CommitInfo {
                    hash: short_hash,
                    full_hash: hash,
                    message,
                    timestamp,
                    changed_files,
                });
            }
        }

        Ok(commits)
    }

    /// 获取指定 commit 的 diff
    pub fn get_commit_diff(&self, commit_hash: &str) -> Result<String, String> {
        let workdir = self
            .current_workdir()
            .ok_or_else(|| "没有设置工作区路径".to_string())?;

        Self::run_git(&workdir, &["show", "--stat", "-p", commit_hash])
    }

    /// 生成 PR 描述
    pub fn generate_pr_description(
        &self,
        initial_branch: &str,
        exec_branch: &str,
    ) -> Result<String, String> {
        let workdir = self
            .current_workdir()
            .ok_or_else(|| "没有设置工作区路径".to_string())?;

        let range = format!("{}..{}", initial_branch, exec_branch);

        // 获取 commit 列表
        let log_output = Self::run_git(&workdir, &["log", &range, "--format=%s"])?;

        // 获取文件变更统计
        let stat_output = Self::run_git(&workdir, &["diff", &range, "--stat"])?;

        let mut description = String::new();
        description.push_str("## Summary\n\n");

        for line in log_output.lines() {
            if !line.is_empty() {
                description.push_str(&format!("- {}\n", line));
            }
        }

        description.push_str("\n## Changed files\n\n");
        for line in stat_output.lines() {
            if !line.is_empty() {
                description.push_str(&format!("- {}\n", line));
            }
        }

        Ok(description)
    }

    /// 获取当前 git 分支和初始分支信息
    pub fn get_branch_info(&self, execution_id: &str) -> BranchInfo {
        let workdir = match self.current_workdir() {
            Some(w) => w,
            None => {
                return BranchInfo {
                    current_branch: None,
                    exec_branch: format!("ai-exec-{}", execution_id),
                    is_git_repo: false,
                }
            }
        };

        let is_git_repo = workdir.join(".git").exists();
        let current_branch = if is_git_repo {
            Self::run_git(&workdir, &["rev-parse", "--abbrev-ref", "HEAD"]).ok()
        } else {
            None
        };

        BranchInfo {
            current_branch,
            exec_branch: format!("ai-exec-{}", execution_id),
            is_git_repo,
        }
    }
}

/// Commit 信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct CommitInfo {
    pub hash: String,
    pub full_hash: String,
    pub message: String,
    pub timestamp: String,
    pub changed_files: usize,
}

/// 分支信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct BranchInfo {
    pub current_branch: Option<String>,
    pub exec_branch: String,
    pub is_git_repo: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_info_serialization() {
        let info = BranchInfo {
            current_branch: Some("main".to_string()),
            exec_branch: "ai-exec-test-123".to_string(),
            is_git_repo: true,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("main"));
        assert!(json.contains("ai-exec-test-123"));
    }

    #[test]
    fn test_commit_info_serialization() {
        let commit = CommitInfo {
            hash: "abc1234".to_string(),
            full_hash: "abc1234567890".to_string(),
            message: "stage: implement feature".to_string(),
            timestamp: "2026-05-03".to_string(),
            changed_files: 3,
        };
        let json = serde_json::to_string(&commit).unwrap();
        assert!(json.contains("abc1234"));
        assert!(json.contains("implement feature"));
    }
}
