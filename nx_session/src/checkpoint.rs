//! 检查点管理
//!
//! 支持会话状态的保存和恢复。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::SessionId;

/// 检查点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// 检查点 ID
    pub id: String,
    /// 会话 ID
    pub session_id: SessionId,
    /// 检查点创建时间
    pub created_at: DateTime<Utc>,
    /// 状态快照（JSON）
    pub state_json: String,
    /// 检查点类型
    pub checkpoint_type: CheckpointType,
    /// 描述
    pub description: Option<String>,
}

/// 检查点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointType {
    /// 自动保存
    AutoSave,
    /// 手动保存
    Manual,
    /// 阶段完成
    StageComplete,
    /// 暂停前
    PrePause,
}

/// 检查点管理器
#[derive(Debug)]
pub struct CheckpointManager {
    /// 检查点存储
    checkpoints: std::sync::Arc<RwLock<Vec<Checkpoint>>>,
}

impl CheckpointManager {
    /// 创建新的检查点管理器
    pub fn new() -> Self {
        Self {
            checkpoints: std::sync::Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 创建检查点
    pub async fn create_checkpoint(
        &self,
        session_id: SessionId,
        state_json: String,
        checkpoint_type: CheckpointType,
        description: Option<String>,
    ) -> Checkpoint {
        let checkpoint = Checkpoint {
            id: Uuid::new_v4().to_string(),
            session_id,
            created_at: Utc::now(),
            state_json,
            checkpoint_type,
            description,
        };

        let mut checkpoints = self.checkpoints.write().await;
        checkpoints.push(checkpoint.clone());

        checkpoint
    }

    /// 获取会话的最新检查点
    pub async fn get_latest_checkpoint(&self, session_id: &SessionId) -> Option<Checkpoint> {
        let checkpoints = self.checkpoints.read().await;
        checkpoints
            .iter()
            .filter(|c| c.session_id == *session_id)
            .max_by_key(|c| c.created_at)
            .cloned()
    }

    /// 获取会话的所有检查点
    pub async fn get_checkpoints(&self, session_id: &SessionId) -> Vec<Checkpoint> {
        let checkpoints = self.checkpoints.read().await;
        checkpoints
            .iter()
            .filter(|c| c.session_id == *session_id)
            .cloned()
            .collect()
    }

    /// 删除会话的检查点
    pub async fn delete_checkpoints(&self, session_id: &SessionId) {
        let mut checkpoints = self.checkpoints.write().await;
        checkpoints.retain(|c| c.session_id != *session_id);
    }

    /// 清理旧检查点（保留最新 N 个）
    pub async fn cleanup_old_checkpoints(&self, session_id: &SessionId, keep_count: usize) {
        let mut checkpoints = self.checkpoints.write().await;

        // 按时间倒序排列
        checkpoints.sort_by_key(|b| std::cmp::Reverse(b.created_at));

        // 保留最新的 keep_count 个
        let to_keep: Vec<_> = checkpoints
            .iter()
            .filter(|c| c.session_id == *session_id)
            .take(keep_count)
            .map(|c| c.id.clone())
            .collect();

        checkpoints.retain(|c| {
            if c.session_id != *session_id {
                true
            } else {
                to_keep.contains(&c.id)
            }
        });
    }
}

impl Default for CheckpointManager {
    fn default() -> Self {
        Self::new()
    }
}

use tokio::sync::RwLock;
