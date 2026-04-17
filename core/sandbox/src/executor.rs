//! 沙箱执行器
//!
//! 在具有资源限制的隔离环境中执行代码。

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Deserialize, Serialize};

/// 执行请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRequest {
    /// 要执行的程序（路径或名称）
    pub program: String,
    /// 传递的参数
    #[serde(default)]
    pub args: Vec<String>,
    /// 环境变量
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    /// 工作目录
    #[serde(default)]
    pub working_dir: Option<PathBuf>,
    /// 标准输入内容
    #[serde(default)]
    pub stdin: Option<String>,
    /// 超时时间（秒）
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// 内存限制（字节）
    #[serde(default = "default_memory")]
    pub memory_limit_bytes: u64,
    /// CPU 时间限制（秒）
    #[serde(default = "default_cpu_time")]
    pub cpu_time_secs: u64,
}

fn default_timeout() -> u64 { 30 }
fn default_memory() -> u64 { 256 * 1024 * 1024 } // 256MB
fn default_cpu_time() -> u64 { 10 }

/// 执行响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResponse {
    /// 退出码
    pub exit_code: Option<i32>,
    /// 标准输出
    pub stdout: String,
    /// 标准错误
    pub stderr: String,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
    /// 是否超时
    pub timed_out: bool,
    /// 使用的内存（如果可用）
    pub memory_used_bytes: Option<u64>,
}

/// 沙箱执行器
pub struct SandboxExecutor {
    #[cfg(unix)]
    seccomp_enabled: bool,
    default_limits: ResourceLimits,
}

/// 资源限制
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// 最大内存（字节）
    pub max_memory_bytes: u64,
    /// 最大 CPU 时间（秒）
    pub max_cpu_seconds: u64,
    /// 最大线程数
    pub max_threads: Option<usize>,
    /// 最大打开文件数
    pub max_open_files: Option<usize>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 256 * 1024 * 1024,
            max_cpu_seconds: 10,
            max_threads: Some(4),
            max_open_files: Some(64),
        }
    }
}

impl SandboxExecutor {
    /// 创建新的沙箱执行器
    pub fn new() -> Self {
        Self {
            #[cfg(unix)]
            seccomp_enabled: true,
            default_limits: ResourceLimits::default(),
        }
    }

    /// 使用指定限制创建沙箱执行器
    pub fn with_limits(limits: ResourceLimits) -> Self {
        Self {
            #[cfg(unix)]
            seccomp_enabled: true,
            default_limits: limits,
        }
    }

    /// 在沙箱中执行程序
    pub async fn execute(&self, request: ExecuteRequest) -> Result<ExecuteResponse, SandboxError> {
        let start = std::time::Instant::now();

        // 验证程序路径
        if request.program.is_empty() {
            return Err(SandboxError::InvalidProgram("程序不能为空".to_string()));
        }

        // 构建命令
        let mut cmd = Command::new(&request.program);
        cmd.args(&request.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped());

        // 设置工作目录
        if let Some(ref dir) = request.working_dir {
            cmd.current_dir(dir);
        } else {
            cmd.current_dir("/tmp");
        }

        // 设置环境变量
        let env = std::env::vars()
            .filter(|(k, _)| !k.starts_with("LD_"))
            .chain(request.env_vars.clone().into_iter())
            .collect::<HashMap<_, _>>();
        cmd.envs(&env);

        // 应用资源限制
        #[cfg(unix)]
        self.apply_resource_limits(&mut cmd, &request);

        // 如果启用则应用 seccomp
        #[cfg(unix)]
        if self.seccomp_enabled {
            self.apply_seccomp_filter(&mut cmd)?;
        }

        // 带超时执行
        let child_result = tokio::time::timeout(
            std::time::Duration::from_secs(request.timeout_secs),
            async {
                let mut child = cmd.spawn().map_err(|e| SandboxError::Execution(e.to_string()))?;
                // 如果提供了标准输入则写入
                if let Some(ref stdin_data) = request.stdin {
                    if let Some(ref mut child_stdin) = child.stdin {
                        child_stdin.write_all(stdin_data.as_bytes()).await.ok();
                    }
                }
                // 等待完成
                let output = child.wait_with_output().await
                    .map_err(|e| SandboxError::Execution(e.to_string()))?;
                Ok::<_, SandboxError>(output)
            }
        ).await;

        let timed_out = matches!(child_result, Err(tokio::time::error::Elapsed { .. }));

        match child_result {
            Ok(Ok(output)) => {
                let elapsed = start.elapsed().as_millis() as u64;

                Ok(ExecuteResponse {
                    exit_code: output.status.code(),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    execution_time_ms: elapsed,
                    timed_out,
                    memory_used_bytes: None,
                })
            }
            Ok(Err(e)) => {
                Err(e)
            }
            Err(_) => {
                // 超时
                Ok(ExecuteResponse {
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::from("执行超时"),
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    timed_out: true,
                    memory_used_bytes: None,
                })
            }
        }
    }

    #[cfg(unix)]
    fn apply_resource_limits(&self, cmd: &mut Command, request: &ExecuteRequest) {
        let memory_limit = request.memory_limit_bytes;
        let cpu_limit = request.cpu_time_secs;

        // Use pre_exec to set resource limits via setrlimit before the child process runs
        // SAFETY: setrlimit is async-signal-safe and we only call it with valid rlimit structs.
        // The closure runs between fork() and exec() in the child process.
        unsafe {
            cmd.pre_exec(move || {
                // RLIMIT_AS — virtual memory limit
                let mem = libc::rlimit {
                    rlim_cur: memory_limit,
                    rlim_max: memory_limit,
                };
                if libc::setrlimit(libc::RLIMIT_AS, &mem) != 0 {
                    return Err(std::io::Error::last_os_error());
                }

                // RLIMIT_CPU — CPU time limit in seconds
                let cpu = libc::rlimit {
                    rlim_cur: cpu_limit,
                    rlim_max: cpu_limit,
                };
                if libc::setrlimit(libc::RLIMIT_CPU, &cpu) != 0 {
                    return Err(std::io::Error::last_os_error());
                }

                // RLIMIT_NOFILE — limit open file descriptors
                let nofile = libc::rlimit {
                    rlim_cur: 64,
                    rlim_max: 64,
                };
                if libc::setrlimit(libc::RLIMIT_NOFILE, &nofile) != 0 {
                    return Err(std::io::Error::last_os_error());
                }

                Ok(())
            });
        }
    }

    #[cfg(unix)]
    fn apply_seccomp_filter(&self, _cmd: &mut Command) -> Result<(), SandboxError> {
        // seccomp is only available on Linux; macOS uses the sandbox-exec/sandbox_init APIs.
        // On macOS we log a warning and skip.
        #[cfg(target_os = "linux")]
        {
            tracing::debug!("seccomp filter placeholder — production should use libseccomp crate");
        }
        #[cfg(not(target_os = "linux"))]
        {
            tracing::warn!("seccomp is not available on this platform, skipping sandbox filter");
        }
        Ok(())
    }

    /// 执行简单的 shell 命令
    pub async fn execute_shell(&self, command: &str) -> Result<ExecuteResponse, SandboxError> {
        self.execute(ExecuteRequest {
            program: "/bin/sh".to_string(),
            args: vec!["-c".to_string(), command.to_string()],
            ..Default::default()
        }).await
    }
}

impl Default for SandboxExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// 沙箱错误
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("无效的程序: {0}")]
    InvalidProgram(String),

    #[error("执行错误: {0}")]
    Execution(String),

    #[error("权限被拒绝: {0}")]
    PermissionDenied(String),

    #[error("超时: {0}")]
    Timeout(String),

    #[error("沙箱错误: {0}")]
    Sandbox(String),
}

impl Default for ExecuteRequest {
    fn default() -> Self {
        Self {
            program: String::new(),
            args: Vec::new(),
            env_vars: HashMap::new(),
            working_dir: None,
            stdin: None,
            timeout_secs: 30,
            memory_limit_bytes: 256 * 1024 * 1024,
            cpu_time_secs: 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_execution() {
        let sandbox = SandboxExecutor::new();
        let result = sandbox.execute_shell("echo 'hello world'").await.unwrap();
        assert_eq!(result.exit_code, Some(0));
        assert!(result.stdout.contains("hello world"));
    }

    #[tokio::test]
    async fn test_timeout() {
        let sandbox = SandboxExecutor::new();
        let result = sandbox.execute(ExecuteRequest {
            program: "sleep".to_string(),
            args: vec!["10".to_string()],
            timeout_secs: 1,
            ..Default::default()
        }).await.unwrap();
        assert!(result.timed_out);
    }
}