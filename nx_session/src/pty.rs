//! PTY 终端集成
//!
//! 支持 PTY (Pseudo-Terminal) 用于交互式终端会话。

use parking_lot::RwLock;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[cfg(unix)]
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtyPair, PtySize};

/// PTY 错误类型
#[derive(Error, Debug)]
pub enum PtyError {
    #[error("PTY 创建失败: {0}")]
    CreateFailed(String),

    #[error("进程启动失败: {0}")]
    ProcessStartFailed(String),

    #[error("读写错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("会话不存在: {0}")]
    SessionNotFound(String),

    #[error("不支持的平台")]
    UnsupportedPlatform,
}

/// PTY 会话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtySessionState {
    /// 已创建
    Created,
    /// 运行中
    Running,
    /// 已暂停
    Paused,
    /// 已退出
    Exited,
    /// 已终止
    Terminated,
}

/// PTY 会话
#[derive(Debug, Clone)]
pub struct PtySession {
    /// 会话 ID
    pub id: String,
    /// 状态
    pub state: PtySessionState,
    /// 进程 ID（如果运行中）
    pub pid: Option<u32>,
    /// 退出码（如果已退出）
    pub exit_code: Option<i32>,
    /// 创建时间
    pub created_at: std::time::Instant,
    /// 最后活动事件
    pub last_activity: std::time::Instant,
}

impl PtySession {
    /// 创建新的 PTY 会话
    pub fn new() -> Self {
        let now = std::time::Instant::now();
        Self {
            id: Uuid::new_v4().to_string(),
            state: PtySessionState::Created,
            pid: None,
            exit_code: None,
            created_at: now,
            last_activity: now,
        }
    }

    /// 检查是否运行中
    pub fn is_running(&self) -> bool {
        self.state == PtySessionState::Running
    }

    /// 更新活动时间
    pub fn touch(&mut self) {
        self.last_activity = std::time::Instant::now();
    }
}

impl Default for PtySession {
    fn default() -> Self {
        Self::new()
    }
}

/// PTY 会话输出
#[derive(Debug, Clone)]
pub struct PtyOutput {
    /// 会话 ID
    pub session_id: String,
    /// 输出数据
    pub data: Vec<u8>,
    /// 是否为标准输出
    pub is_stdout: bool,
    /// 时间戳
    pub timestamp: std::time::Instant,
}

/// PTY 会话管理器
pub struct PtyManager {
    /// PTY 会话存储
    sessions: Arc<RwLock<HashMap<String, PtySession>>>,
    /// PTY 主端
    #[cfg(unix)]
    masters: Arc<RwLock<HashMap<String, PtyPair>>>,
    /// 子进程
    #[cfg(unix)]
    children: Arc<RwLock<HashMap<String, Box<dyn Child + Send>>>>,
    /// 输出缓冲
    outputs: Arc<RwLock<HashMap<String, Vec<PtyOutput>>>>,
}

impl std::fmt::Debug for PtyManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PtyManager")
            .field("sessions", &self.sessions)
            .field("outputs", &self.outputs)
            .finish()
    }
}

impl PtyManager {
    /// 创建新的 PTY 管理器
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(unix)]
            masters: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(unix)]
            children: Arc::new(RwLock::new(HashMap::new())),
            outputs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建新的 PTY 会话
    #[cfg(unix)]
    pub fn create_session(
        &self,
        command: Option<&str>,
        args: Option<&[&str]>,
        cwd: Option<&str>,
        env_vars: Option<&HashMap<String, String>>,
    ) -> Result<String, PtyError> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::CreateFailed(e.to_string()))?;

        let mut session = PtySession::new();
        let session_id = session.id.clone();

        let mut cmd = CommandBuilder::new(command.unwrap_or("bash"));

        if let Some(args) = args {
            for arg in args {
                cmd.arg(arg);
            }
        }

        if let Some(cwd) = cwd {
            cmd.cwd(cwd);
        }

        if let Some(env_vars) = env_vars {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::ProcessStartFailed(e.to_string()))?;

        session.state = PtySessionState::Running;
        session.pid = child.process_id();

        // 保存会话
        {
            let mut sessions = self.sessions.write();
            sessions.insert(session_id.clone(), session);
        }

        // 保存 PTY 对
        {
            let mut masters = self.masters.write();
            masters.insert(session_id.clone(), pair);
        }

        // 保存子进程
        {
            let mut children = self.children.write();
            children.insert(session_id.clone(), child);
        }

        // 初始化输出缓冲
        {
            let mut outputs = self.outputs.write();
            outputs.insert(session_id.clone(), Vec::new());
        }

        Ok(session_id)
    }

    /// 创建新的 PTY 会话（不支持的平台）
    #[cfg(not(unix))]
    pub fn create_session(
        &self,
        _command: Option<&str>,
        _args: Option<&[&str]>,
        _cwd: Option<&str>,
        _env_vars: Option<&HashMap<String, String>>,
    ) -> Result<String, PtyError> {
        Err(PtyError::UnsupportedPlatform)
    }

    /// 写入 PTY
    #[cfg(unix)]
    pub fn write(&self, session_id: &str, data: &[u8]) -> Result<(), PtyError> {
        let mut masters = self.masters.write();

        if let Some(pair) = masters.get_mut(session_id) {
            let mut writer = pair.master.take_writer().map_err(|e| {
                PtyError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("writer not available: {}", e),
                ))
            })?;
            writer.write_all(data).map_err(|e| PtyError::IoError(e))?;

            // 更新活动时间
            let mut sessions = self.sessions.write();
            if let Some(session) = sessions.get_mut(session_id) {
                session.touch();
            }

            Ok(())
        } else {
            Err(PtyError::SessionNotFound(session_id.to_string()))
        }
    }

    /// 写入 PTY（不支持的平台）
    #[cfg(not(unix))]
    pub fn write(&self, _session_id: &str, _data: &[u8]) -> Result<(), PtyError> {
        Err(PtyError::UnsupportedPlatform)
    }

    /// 读取 PTY 输出
    #[cfg(unix)]
    pub fn read(&self, session_id: &str, timeout_ms: u64) -> Result<Vec<PtyOutput>, PtyError> {
        use std::time::{Duration, Instant};

        let start = Instant::now();
        let deadline = Duration::from_millis(timeout_ms);

        let mut outputs = self.outputs.write();

        if let Some(buffer) = outputs.get_mut(session_id) {
            let mut result = Vec::new();

            // 从缓冲区读取已有数据
            while !buffer.is_empty() && start.elapsed() < deadline {
                if let Some(output) = buffer.first() {
                    if output.timestamp <= start {
                        result.push(buffer.remove(0));
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            // 如果还没有数据，等待一下
            if result.is_empty() {
                drop(outputs);
                std::thread::sleep(Duration::from_millis(10));
                let mut outputs = self.outputs.write();
                if let Some(buffer) = outputs.get_mut(session_id) {
                    while !buffer.is_empty() {
                        result.push(buffer.remove(0));
                    }
                }
            }

            Ok(result)
        } else {
            Err(PtyError::SessionNotFound(session_id.to_string()))
        }
    }

    /// 读取 PTY 输出（不支持的平台）
    #[cfg(not(unix))]
    pub fn read(&self, _session_id: &str, _timeout_ms: u64) -> Result<Vec<PtyOutput>, PtyError> {
        Err(PtyError::UnsupportedPlatform)
    }

    /// 调整 PTY 大小
    #[cfg(unix)]
    pub fn resize(&self, session_id: &str, rows: u16, cols: u16) -> Result<(), PtyError> {
        let mut masters = self.masters.write();

        if let Some(pair) = masters.get_mut(session_id) {
            pair.master
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| PtyError::CreateFailed(e.to_string()))?;

            Ok(())
        } else {
            Err(PtyError::SessionNotFound(session_id.to_string()))
        }
    }

    /// 调整 PTY 大小（不支持的平台）
    #[cfg(not(unix))]
    pub fn resize(&self, _session_id: &str, _rows: u16, _cols: u16) -> Result<(), PtyError> {
        Err(PtyError::UnsupportedPlatform)
    }

    /// 暂停会话
    #[cfg(unix)]
    pub fn pause(&self, session_id: &str) -> Result<(), PtyError> {
        let mut sessions = self.sessions.write();

        if let Some(session) = sessions.get_mut(session_id) {
            if session.state == PtySessionState::Running {
                session.state = PtySessionState::Paused;
                return Ok(());
            }
        }

        Err(PtyError::SessionNotFound(session_id.to_string()))
    }

    /// 暂停会话（不支持的平台）
    #[cfg(not(unix))]
    pub fn pause(&self, _session_id: &str) -> Result<(), PtyError> {
        Err(PtyError::UnsupportedPlatform)
    }

    /// 恢复会话
    #[cfg(unix)]
    pub fn resume(&self, session_id: &str) -> Result<(), PtyError> {
        let mut sessions = self.sessions.write();

        if let Some(session) = sessions.get_mut(session_id) {
            if session.state == PtySessionState::Paused {
                session.state = PtySessionState::Running;
                return Ok(());
            }
        }

        Err(PtyError::SessionNotFound(session_id.to_string()))
    }

    /// 恢复会话（不支持的平台）
    #[cfg(not(unix))]
    pub fn resume(&self, _session_id: &str) -> Result<(), PtyError> {
        Err(PtyError::UnsupportedPlatform)
    }

    /// 终止会话
    #[cfg(unix)]
    pub fn terminate(&self, session_id: &str) -> Result<Option<i32>, PtyError> {
        // 杀掉子进程
        {
            let mut children = self.children.write();
            if let Some(mut child) = children.remove(session_id) {
                let _ = child.kill();
                match child.wait() {
                    Ok(exit_status) => {
                        let exit_code = exit_status.exit_code() as i32;

                        // 更新会话状态
                        let mut sessions = self.sessions.write();
                        if let Some(session) = sessions.get_mut(session_id) {
                            session.state = PtySessionState::Terminated;
                            session.exit_code = Some(exit_code);
                        }

                        return Ok(Some(exit_code));
                    }
                    Err(_) => {
                        // 进程已经结束
                    }
                }
            }
        }

        // 清理
        {
            let mut masters = self.masters.write();
            masters.remove(session_id);
        }

        {
            let mut outputs = self.outputs.write();
            outputs.remove(session_id);
        }

        {
            let mut sessions = self.sessions.write();
            sessions.remove(session_id);
        }

        Ok(None)
    }

    /// 终止会话（不支持的平台）
    #[cfg(not(unix))]
    pub fn terminate(&self, _session_id: &str) -> Result<Option<i32>, PtyError> {
        Err(PtyError::UnsupportedPlatform)
    }

    /// 获取会话状态
    #[cfg(unix)]
    pub fn get_session(&self, session_id: &str) -> Option<PtySession> {
        let sessions = self.sessions.read();
        sessions.get(session_id).cloned()
    }

    /// 获取会话状态（不支持的平台）
    #[cfg(not(unix))]
    pub fn get_session(&self, _session_id: &str) -> Option<PtySession> {
        None
    }

    /// 列出所有会话
    pub fn list_sessions(&self) -> Vec<PtySession> {
        let sessions = self.sessions.read();
        sessions.values().cloned().collect()
    }

    /// 检查会话是否有输出
    pub fn has_output(&self, session_id: &str) -> bool {
        let outputs = self.outputs.read();
        outputs
            .get(session_id)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }

    /// 获取空闲会话数
    pub fn idle_session_count(&self) -> usize {
        let sessions = self.sessions.read();
        let now = std::time::Instant::now();

        sessions
            .values()
            .filter(|s| s.is_running() && now.duration_since(s.last_activity).as_secs() > 300)
            .count()
    }
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_session_creation() {
        let session = PtySession::new();
        assert_eq!(session.state, PtySessionState::Created);
        assert!(session.pid.is_none());
    }

    #[test]
    fn test_pty_manager_empty() {
        let manager = PtyManager::new();
        let sessions = manager.list_sessions();
        assert!(sessions.is_empty());
    }
}
