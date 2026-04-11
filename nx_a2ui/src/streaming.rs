//! A2UI 消息流

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, broadcast};
use tokio_stream::wrappers::BroadcastStream;
use futures_util::{Stream, StreamExt};
use chrono::{DateTime, Utc};

use crate::message::{A2UMessage, MessageType, MessagePriority};
use crate::notification::Notification;
use crate::dialog::ConfirmationDialog;
use crate::progress::ProgressUpdate;

/// 流配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// 会话 ID
    pub session_id: String,
    /// 缓冲区大小
    pub buffer_size: usize,
    /// 是否启用历史消息
    pub enable_history: bool,
    /// 历史消息最大数量
    pub max_history: usize,
    /// 消息过期时间（秒）
    pub message_ttl_secs: Option<u64>,
}

impl StreamConfig {
    /// 创建新的流配置
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            buffer_size: 100,
            enable_history: true,
            max_history: 1000,
            message_ttl_secs: None,
        }
    }

    /// 设置缓冲区大小
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// 设置是否启用历史
    pub fn with_history(mut self, enable: bool, max: Option<usize>) -> Self {
        self.enable_history = enable;
        if let Some(m) = max {
            self.max_history = m;
        }
        self
    }

    /// 设置消息 TTL
    pub fn with_ttl(mut self, secs: u64) -> Self {
        self.message_ttl_secs = Some(secs);
        self
    }
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self::new("default")
    }
}

/// 流错误
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("通道已关闭")]
    ChannelClosed,

    #[error("流已取消")]
    Cancelled,

    #[error("发送失败: {0}")]
    SendFailed(String),

    #[error("接收失败: {0}")]
    ReceiveFailed(String),

    #[error("会话不存在: {0}")]
    SessionNotFound(String),

    #[error("会话已存在: {0}")]
    SessionAlreadyExists(String),

    #[error("接收广播错误")]
    BroadcastRecvError,
}

/// A2UI 流事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum A2uiStreamEvent {
    /// 新消息
    Message(A2UMessage),
    /// 通知
    Notification(Notification),
    /// 对话框
    Dialog(ConfirmationDialog),
    /// 进度更新
    Progress(ProgressUpdate),
    /// 心跳
    Heartbeat { timestamp: DateTime<Utc> },
    /// 错误
    Error { code: String, message: String },
}

/// 消息流
pub struct MessageStream {
    config: StreamConfig,
    /// 消息发送器
    message_tx: broadcast::Sender<A2uiStreamEvent>,
    /// 消息接收器
    message_rx: broadcast::Receiver<A2uiStreamEvent>,
    /// 历史消息
    history: std::sync::RwLock<Vec<A2uiStreamEvent>>,
}

impl Clone for MessageStream {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            message_tx: self.message_tx.clone(),
            message_rx: self.message_tx.subscribe(),
            history: std::sync::RwLock::new(self.history.read().unwrap().clone()),
        }
    }
}

impl MessageStream {
    /// 创建新的消息流
    pub fn new(config: StreamConfig) -> Self {
        let (message_tx, message_rx) = broadcast::channel(config.buffer_size);
        Self {
            config,
            message_tx,
            message_rx,
            history: std::sync::RwLock::new(Vec::new()),
        }
    }

    /// 获取会话 ID
    pub fn session_id(&self) -> &str {
        &self.config.session_id
    }

    /// 发送消息
    pub async fn send_message(&self, message: A2UMessage) -> Result<(), StreamError> {
        let event = A2uiStreamEvent::Message(message);
        self.send_event(event).await
    }

    /// 发送通知
    pub async fn send_notification(&self, notification: Notification) -> Result<(), StreamError> {
        let event = A2uiStreamEvent::Notification(notification);
        self.send_event(event).await
    }

    /// 发送对话框
    pub async fn send_dialog(&self, dialog: ConfirmationDialog) -> Result<(), StreamError> {
        let event = A2uiStreamEvent::Dialog(dialog);
        self.send_event(event).await
    }

    /// 发送进度更新
    pub async fn send_progress(&self, progress: ProgressUpdate) -> Result<(), StreamError> {
        let event = A2uiStreamEvent::Progress(progress);
        self.send_event(event).await
    }

    /// 发送事件
    async fn send_event(&self, event: A2uiStreamEvent) -> Result<(), StreamError> {
        // 存储到历史
        if self.config.enable_history {
            if let A2uiStreamEvent::Message(msg) = &event {
                let mut history = self.history.write().unwrap();
                if history.len() >= self.config.max_history {
                    history.remove(0);
                }
                history.push(event.clone());
            }
        }

        // 发送到广播通道
        self.message_tx
            .send(event)
            .map_err(|e| StreamError::SendFailed(e.to_string()))?;
        Ok(())
    }

    /// 获取事件流
    pub fn stream(&self) -> impl Stream<Item = Result<A2uiStreamEvent, StreamError>> + 'static {
        BroadcastStream::new(self.message_tx.subscribe())
            .map(|r| r.map_err(|_| StreamError::BroadcastRecvError))
    }

    /// 获取消息流（只接收消息）
    pub fn message_stream(&self) -> impl Stream<Item = A2UMessage> {
        self.stream()
            .filter_map(|event| async move {
                match event {
                    Ok(A2uiStreamEvent::Message(msg)) => Some(msg),
                    Ok(A2uiStreamEvent::Notification(_))
                    | Ok(A2uiStreamEvent::Dialog(_))
                    | Ok(A2uiStreamEvent::Progress(_))
                    | Ok(A2uiStreamEvent::Heartbeat { .. })
                    | Ok(A2uiStreamEvent::Error { .. }) => None,
                    Err(_) => None,
                }
            })
    }

    /// 获取历史消息
    pub fn get_history(&self) -> Vec<A2UMessage> {
        let history = self.history.read().unwrap();
        history.iter()
            .filter_map(|e| match e {
                A2uiStreamEvent::Message(msg) => Some(msg.clone()),
                _ => None,
            })
            .collect()
    }

    /// 获取历史事件
    pub fn get_all_history(&self) -> Vec<A2uiStreamEvent> {
        self.history.read().unwrap().clone()
    }

    /// 清空历史
    pub fn clear_history(&self) {
        let mut history = self.history.write().unwrap();
        history.clear();
    }

    /// 获取流统计
    pub fn stats(&self) -> StreamStats {
        let history = self.history.read().unwrap();
        StreamStats {
            session_id: self.config.session_id.clone(),
            history_size: history.len(),
            max_history: self.config.max_history,
            message_count: history.iter().filter(|e| matches!(e, A2uiStreamEvent::Message(_))).count(),
            notification_count: history.iter().filter(|e| matches!(e, A2uiStreamEvent::Notification(_))).count(),
            progress_count: history.iter().filter(|e| matches!(e, A2uiStreamEvent::Progress(_))).count(),
        }
    }
}

/// 流统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStats {
    /// 会话 ID
    pub session_id: String,
    /// 历史大小
    pub history_size: usize,
    /// 最大历史
    pub max_history: usize,
    /// 消息数量
    pub message_count: usize,
    /// 通知数量
    pub notification_count: usize,
    /// 进度数量
    pub progress_count: usize,
}

/// 流管理器
pub struct StreamManager {
    streams: std::sync::RwLock<std::collections::HashMap<String, MessageStream>>,
    default_config: StreamConfig,
}

impl StreamManager {
    /// 创建新的流管理器
    pub fn new() -> Self {
        Self {
            streams: std::sync::RwLock::new(std::collections::HashMap::new()),
            default_config: StreamConfig::default(),
        }
    }

    /// 创建新的流管理器（带默认配置）
    pub fn with_config(config: StreamConfig) -> Self {
        Self {
            streams: std::sync::RwLock::new(std::collections::HashMap::new()),
            default_config: config,
        }
    }

    /// 创建会话流
    pub fn create_stream(&self, session_id: impl Into<String>) -> Result<MessageStream, StreamError> {
        let session_id = session_id.into();

        let stream = {
            let mut streams = self.streams.write().unwrap();
            if streams.contains_key(&session_id) {
                return Err(StreamError::SessionAlreadyExists(session_id));
            }
            let config = StreamConfig::new(&session_id);
            let stream = MessageStream::new(config);
            streams.insert(session_id.clone(), stream.clone());
            stream
        };

        Ok(stream)
    }

    /// 获取或创建会话流
    pub fn get_or_create(&self, session_id: impl Into<String>) -> Result<MessageStream, StreamError> {
        let session_id = session_id.into();

        // 先尝试获取
        {
            let streams = self.streams.read().unwrap();
            if let Some(stream) = streams.get(&session_id) {
                return Ok(MessageStream::new(StreamConfig::new(session_id)));
            }
        }

        // 创建新的
        self.create_stream(session_id)
    }

    /// 获取流
    pub fn get(&self, session_id: &str) -> Option<MessageStream> {
        let streams = self.streams.read().unwrap();
        streams.get(session_id).cloned()
    }

    /// 删除流
    pub fn remove(&self, session_id: &str) -> bool {
        let mut streams = self.streams.write().unwrap();
        streams.remove(session_id).is_some()
    }

    /// 列出所有会话
    pub fn list_sessions(&self) -> Vec<String> {
        let streams = self.streams.read().unwrap();
        streams.keys().cloned().collect()
    }

    /// 获取所有流的统计
    pub fn get_all_stats(&self) -> Vec<StreamStats> {
        let streams = self.streams.read().unwrap();
        streams.values().map(|s| s.stats()).collect()
    }
}

impl Default for StreamManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 流构建器
pub struct MessageStreamBuilder {
    config: StreamConfig,
}

impl MessageStreamBuilder {
    /// 创建新的流构建器
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            config: StreamConfig::new(session_id),
        }
    }

    /// 设置会话 ID
    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.config.session_id = id.into();
        self
    }

    /// 设置缓冲区大小
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.config.buffer_size = size;
        self
    }

    /// 设置历史
    pub fn history(mut self, enable: bool, max: Option<usize>) -> Self {
        self.config.enable_history = enable;
        if let Some(m) = max {
            self.config.max_history = m;
        }
        self
    }

    /// 设置消息 TTL
    pub fn ttl(mut self, secs: u64) -> Self {
        self.config.message_ttl_secs = Some(secs);
        self
    }

    /// 构建流
    pub fn build(self) -> MessageStream {
        MessageStream::new(self.config)
    }
}
