//! 会话管理器
//!
//! 负责会话的创建、存储、查找和生命周期管理。

use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::persistence::{PersistenceError, SessionStore};
use super::pty::PtyManager;
use super::{CheckpointManager, Session, SessionId, SessionMetadata, SessionStatus};

/// 会话管理器
///
/// 负责会话的创建、存储、查找和生命周期管理。
#[derive(Debug)]
pub struct SessionManager {
    /// 活跃会话存储（内存）
    sessions: Arc<RwLock<HashMap<SessionId, Session>>>,
    /// Resume Key -> SessionId 映射
    resume_keys: Arc<RwLock<HashMap<String, SessionId>>>,
    /// 检查点管理器
    checkpoint_manager: Arc<CheckpointManager>,
    /// PTY 管理器
    pty_manager: Arc<PtyManager>,
    /// 持久化存储
    store: Option<Arc<SessionStore>>,
    /// 最大活跃会话数
    max_active_sessions: usize,
}

impl SessionManager {
    /// 创建新的会话管理器
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            resume_keys: Arc::new(RwLock::new(HashMap::new())),
            checkpoint_manager: Arc::new(CheckpointManager::new()),
            pty_manager: Arc::new(PtyManager::new()),
            store: None,
            max_active_sessions: 100,
        }
    }

    /// 创建带持久化的会话管理器
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn with_persistence(store: SessionStore) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            resume_keys: Arc::new(RwLock::new(HashMap::new())),
            checkpoint_manager: Arc::new(CheckpointManager::new()),
            pty_manager: Arc::new(PtyManager::new()),
            store: Some(Arc::new(store)),
            max_active_sessions: 100,
        }
    }

    /// 创建新会话
    pub async fn create_session(&self, metadata: SessionMetadata) -> Session {
        let mut session = Session::new();
        session.metadata = metadata;

        // 如果有 resume_key，注册它
        if let Some(ref key) = session.metadata.resume_key {
            let mut resume_keys = self.resume_keys.write().await;
            resume_keys.insert(key.clone(), session.id.clone());
        }

        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id.clone(), session.clone());

        // 持久化
        if let Some(ref store) = self.store {
            let _ = store.save(&session);
        }

        session
    }

    /// 创建带工作流的会话
    pub async fn create_workflow_session(&self, workflow_id: String) -> Session {
        let metadata = SessionMetadata {
            workflow_id: Some(workflow_id),
            ..Default::default()
        };
        self.create_session(metadata).await
    }

    /// 获取会话
    pub async fn get_session(&self, id: &SessionId) -> Option<Session> {
        // 先从内存获取
        {
            let sessions = self.sessions.read().await;
            if let Some(session) = sessions.get(id) {
                return Some(session.clone());
            }
        }

        // 从持久化存储加载
        if let Some(ref store) = self.store {
            if let Ok(session) = store.get(id) {
                let mut sessions = self.sessions.write().await;
                sessions.insert(id.clone(), session.clone());
                return Some(session);
            }
        }

        None
    }

    /// 通过 resume_key 获取会话
    pub async fn get_session_by_resume_key(&self, resume_key: &str) -> Option<Session> {
        let resume_keys = self.resume_keys.read().await;
        let session_id = resume_keys.get(resume_key)?.clone();

        drop(resume_keys);
        self.get_session(&session_id).await
    }

    /// 更新会话状态
    pub async fn update_session(&self, id: &SessionId, f: impl FnOnce(&mut Session)) -> Option<()> {
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(id)?;

        f(session);
        session.updated_at = Utc::now();

        // 持久化
        if let Some(ref store) = self.store {
            let _ = store.save(session);
        }

        Some(())
    }

    /// 激活会话
    pub async fn activate_session(&self, id: &SessionId) -> Option<()> {
        self.update_session(id, |s| s.activate()).await
    }

    /// 标记会话为空闲
    pub async fn idle_session(&self, id: &SessionId) -> Option<()> {
        self.update_session(id, |s| s.idle()).await
    }

    /// 暂停会话
    pub async fn pause_session(&self, id: &SessionId) -> Option<()> {
        self.update_session(id, |s| s.pause()).await
    }

    /// 终止会话
    pub async fn terminate_session(&self, id: &SessionId) -> Option<()> {
        // 获取 resume_key 以便清理
        let resume_key = {
            let sessions = self.sessions.read().await;
            sessions.get(id).and_then(|s| s.metadata.resume_key.clone())
        };

        // 从 resume_keys 移除
        if let Some(key) = resume_key {
            let mut resume_keys = self.resume_keys.write().await;
            resume_keys.remove(&key);
        }

        // 终止会话
        self.update_session(id, |s| s.terminate()).await
    }

    /// 删除会话
    pub async fn delete_session(&self, id: &SessionId) -> bool {
        // 先终止（清理 resume_key）
        self.terminate_session(id).await;

        let mut sessions = self.sessions.write().await;
        sessions.remove(id).is_some()
    }

    /// 同步会话到持久化存储
    pub async fn sync_session(&self, id: &SessionId) -> Result<(), PersistenceError> {
        let sessions = self.sessions.read().await;

        if let Some(session) = sessions.get(id) {
            if let Some(ref store) = self.store {
                store.save(session)?;
            }
        }

        Ok(())
    }

    /// 从持久化存储恢复会话
    pub async fn restore_session(
        &self,
        id: &SessionId,
    ) -> Result<Option<Session>, PersistenceError> {
        if let Some(ref store) = self.store {
            match store.get(id) {
                Ok(session) => {
                    let mut sessions = self.sessions.write().await;
                    sessions.insert(id.clone(), session.clone());

                    // 恢复 resume_key
                    if let Some(ref key) = session.metadata.resume_key {
                        let mut resume_keys = self.resume_keys.write().await;
                        resume_keys.insert(key.clone(), session.id.clone());
                    }

                    return Ok(Some(session));
                }
                Err(e) => return Err(e),
            }
        }

        Ok(None)
    }

    /// 完成会话（特殊终止）
    pub async fn complete_session(&self, id: &SessionId) -> Option<()> {
        self.update_session(id, |s| {
            s.status = SessionStatus::Terminated;
            s.terminated_at = Some(Utc::now());
            s.updated_at = Utc::now();
        })
        .await
    }

    /// 列出会话
    pub async fn list_sessions(&self) -> Vec<Session> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }

    /// 按状态列出会话
    pub async fn list_sessions_by_status(&self, status: SessionStatus) -> Vec<Session> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|s| s.status == status)
            .cloned()
            .collect()
    }

    /// 获取活动会话数
    pub async fn active_session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.values().filter(|s| s.is_active()).count()
    }

    /// 检查是否可以创建新会话
    pub async fn can_create_session(&self) -> bool {
        self.active_session_count().await < self.max_active_sessions
    }

    /// 清理已终止的会话
    pub async fn cleanup_terminated(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let mut resume_keys = self.resume_keys.write().await;

        let terminated_ids: Vec<_> = sessions
            .iter()
            .filter(|(_, s)| s.status == SessionStatus::Terminated)
            .map(|(id, _)| id.clone())
            .collect();

        let count = terminated_ids.len();

        for id in terminated_ids {
            // 清理 resume_key
            if let Some(s) = sessions.get(&id) {
                if let Some(ref key) = s.metadata.resume_key {
                    resume_keys.remove(key);
                }
            }
            sessions.remove(&id);
        }

        count
    }

    /// 获取检查点管理器
    pub fn checkpoint_manager(&self) -> Arc<CheckpointManager> {
        self.checkpoint_manager.clone()
    }

    /// 获取 PTY 管理器
    pub fn pty_manager(&self) -> Arc<PtyManager> {
        self.pty_manager.clone()
    }

    /// 获取会话统计
    pub async fn get_stats(&self) -> SessionStats {
        let sessions = self.sessions.read().await;

        let mut stats = SessionStats::default();

        for session in sessions.values() {
            match session.status {
                SessionStatus::Pending => stats.pending_count += 1,
                SessionStatus::Running => stats.running_count += 1,
                SessionStatus::Active => stats.active_count += 1,
                SessionStatus::Idle => stats.idle_count += 1,
                SessionStatus::Paused => stats.paused_count += 1,
                SessionStatus::Terminated => stats.terminated_count += 1,
            }
        }

        stats.total_count = sessions.len();
        stats
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 会话统计信息
#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    pub total_count: usize,
    pub pending_count: usize,
    pub running_count: usize,
    pub active_count: usize,
    pub idle_count: usize,
    pub paused_count: usize,
    pub terminated_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_get_session() {
        let manager = SessionManager::new();

        let session = manager.create_session(SessionMetadata::default()).await;
        assert_eq!(session.status, SessionStatus::Pending);

        let retrieved = manager.get_session(&session.id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, session.id);
    }

    #[tokio::test]
    async fn test_resume_key() {
        let manager = SessionManager::new();

        // create_session uses Session::new() internally which generates a resume_key,
        // but passing default() metadata overwrites it with None.
        // Use Session::new() metadata which has a resume_key.
        let session = manager
            .create_session(SessionMetadata {
                resume_key: Some(uuid::Uuid::new_v4().to_string()),
                ..Default::default()
            })
            .await;
        let resume_key = session
            .metadata
            .resume_key
            .as_ref()
            .expect("resume_key should be set");

        let resumed = manager.get_session_by_resume_key(resume_key).await;
        assert!(resumed.is_some());
        assert_eq!(resumed.unwrap().id, session.id);
    }

    #[tokio::test]
    async fn test_terminate_session() {
        let manager = SessionManager::new();

        let session = manager.create_session(SessionMetadata::default()).await;
        manager.activate_session(&session.id).await;

        manager.terminate_session(&session.id).await;

        let retrieved = manager.get_session(&session.id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().status, SessionStatus::Terminated);
    }
}
