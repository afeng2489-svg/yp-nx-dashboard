//! 会话服务

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::session_repository::{RepositoryError, SessionRepository};

/// 会话服务
#[derive(Clone)]
pub struct SessionService {
    repo: Arc<dyn SessionRepository>,
}

impl std::fmt::Debug for SessionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionService").finish()
    }
}

impl SessionService {
    /// 创建新的会话服务
    pub fn new(repo: Arc<dyn SessionRepository>) -> Self {
        Self { repo }
    }

    /// 创建新会话
    pub async fn create_session(&self, workflow_id: String) -> Result<Session, RepositoryError> {
        let session = Session::new(workflow_id);
        self.repo.create(&session)?;
        Ok(session)
    }

    /// 获取会话
    pub async fn get_session(&self, id: &str) -> Result<Option<Session>, RepositoryError> {
        self.repo.find_by_id(id)
    }

    /// 通过 resume_key 获取会话
    pub async fn get_session_by_resume_key(
        &self,
        resume_key: &str,
    ) -> Result<Option<Session>, RepositoryError> {
        self.repo.find_by_resume_key(resume_key)
    }

    /// 更新会话状态
    pub async fn update_status(
        &self,
        id: &str,
        status: SessionStatus,
    ) -> Result<(), RepositoryError> {
        let mut session = self.repo.find_by_id(id)?;
        if let Some(ref mut s) = session {
            s.status = status;
            s.updated_at = Utc::now();
            self.repo.update(s)?;
        }
        Ok(())
    }

    /// 暂停会话
    pub async fn pause_session(&self, id: &str) -> Result<Session, RepositoryError> {
        let mut session = self
            .repo
            .find_by_id(id)?
            .ok_or_else(|| RepositoryError::NotFound(id.to_string()))?;

        session.pause();
        self.repo.update(&session)?;

        tracing::info!("暂停会话: {}", id);
        Ok(session)
    }

    /// 激活会话
    pub async fn activate_session(&self, id: &str) -> Result<Session, RepositoryError> {
        let mut session = self
            .repo
            .find_by_id(id)?
            .ok_or_else(|| RepositoryError::NotFound(id.to_string()))?;

        session.activate();
        self.repo.update(&session)?;

        tracing::info!("激活会话: {}", id);
        Ok(session)
    }

    /// 恢复会话
    pub async fn resume_session(&self, resume_key: &str) -> Result<Session, RepositoryError> {
        let mut session = self.repo.find_by_resume_key(resume_key)?.ok_or_else(|| {
            RepositoryError::NotFound(format!("No session with resume_key: {}", resume_key))
        })?;

        if !session.can_resume() {
            return Err(RepositoryError::NotFound(format!(
                "Session {} cannot be resumed (status: {}, resume_key: {:?})",
                session.id, session.status, session.resume_key
            )));
        }

        session.resume();
        self.repo.update(&session)?;

        tracing::info!("恢复会话: {} (resume_key: {})", session.id, resume_key);
        Ok(session)
    }

    /// 同步会话状态
    pub async fn sync_session(&self, id: &str) -> Result<Session, RepositoryError> {
        let session = self
            .repo
            .find_by_id(id)?
            .ok_or_else(|| RepositoryError::NotFound(id.to_string()))?;

        tracing::debug!("同步会话状态: {} (status: {})", id, session.status);
        Ok(session)
    }

    /// 删除会话
    pub async fn delete_session(&self, id: &str) -> Result<bool, RepositoryError> {
        self.repo.delete(id)
    }

    /// 列出会话
    pub async fn list_sessions(&self) -> Result<Vec<Session>, RepositoryError> {
        self.repo.find_all()
    }
}

/// 会话状态
///
/// State machine: Pending → Active → Idle → Paused → Active (resume)
///                Any state can transition to Terminated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// 等待中
    Pending,
    /// 活跃
    Active,
    /// 空闲
    Idle,
    /// 已暂停
    Paused,
    /// 已终止
    Terminated,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Pending => write!(f, "pending"),
            SessionStatus::Active => write!(f, "active"),
            SessionStatus::Idle => write!(f, "idle"),
            SessionStatus::Paused => write!(f, "paused"),
            SessionStatus::Terminated => write!(f, "terminated"),
        }
    }
}

/// 会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub workflow_id: String,
    pub status: SessionStatus,
    pub resume_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Session {
    /// 创建新会话
    pub fn new(workflow_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workflow_id,
            status: SessionStatus::Pending,
            resume_key: Some(uuid::Uuid::new_v4().to_string()),
            created_at: now,
            updated_at: now,
        }
    }

    /// 激活会话
    pub fn activate(&mut self) {
        self.status = SessionStatus::Active;
        self.updated_at = Utc::now();
    }

    /// 标记为空闲
    pub fn idle(&mut self) {
        self.status = SessionStatus::Idle;
        self.updated_at = Utc::now();
    }

    /// 暂停会话
    pub fn pause(&mut self) {
        self.status = SessionStatus::Paused;
        self.updated_at = Utc::now();
    }

    /// 恢复会话（从暂停状态恢复）
    pub fn resume(&mut self) {
        if self.status == SessionStatus::Paused {
            self.status = SessionStatus::Active;
            self.updated_at = Utc::now();
        }
    }

    /// 终止会话
    pub fn terminate(&mut self) {
        self.status = SessionStatus::Terminated;
        self.updated_at = Utc::now();
    }

    /// 检查会话是否可恢复
    pub fn can_resume(&self) -> bool {
        self.status == SessionStatus::Paused && self.resume_key.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::session_repository::SqliteSessionRepository;

    #[tokio::test]
    async fn test_create_and_get_session() {
        let repo = SqliteSessionRepository::in_memory().unwrap();
        let service = SessionService::new(Arc::new(repo));

        let session = service
            .create_session("workflow-1".to_string())
            .await
            .unwrap();
        assert_eq!(session.workflow_id, "workflow-1");
        assert_eq!(session.status, SessionStatus::Pending);

        let found = service.get_session(&session.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, session.id);
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let repo = SqliteSessionRepository::in_memory().unwrap();
        let service = SessionService::new(Arc::new(repo));

        service
            .create_session("workflow-1".to_string())
            .await
            .unwrap();
        service
            .create_session("workflow-2".to_string())
            .await
            .unwrap();

        let sessions = service.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_session() {
        let repo = SqliteSessionRepository::in_memory().unwrap();
        let service = SessionService::new(Arc::new(repo));

        let session = service
            .create_session("workflow-1".to_string())
            .await
            .unwrap();
        let deleted = service.delete_session(&session.id).await.unwrap();
        assert!(deleted);

        let found = service.get_session(&session.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_update_status() {
        let repo = SqliteSessionRepository::in_memory().unwrap();
        let service = SessionService::new(Arc::new(repo));

        let session = service
            .create_session("workflow-1".to_string())
            .await
            .unwrap();
        service
            .update_status(&session.id, SessionStatus::Active)
            .await
            .unwrap();

        let found = service.get_session(&session.id).await.unwrap().unwrap();
        assert_eq!(found.status, SessionStatus::Active);
    }
}
