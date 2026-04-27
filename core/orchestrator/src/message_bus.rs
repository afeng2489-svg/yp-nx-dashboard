//! Message Bus Protocol - Pub-sub communication for agent coordination

use crate::error::BusError;
use crate::team::{AgentId, TeamId};
use crate::CliTokenUsage;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

/// Message bus instance for agent communication
pub struct MessageBus {
    /// Subscribers: channel -> broadcast sender
    subscribers: Arc<RwLock<HashMap<Channel, broadcast::Sender<BusMessage>>>>,
    /// Dead letter queue for failed messages
    dead_letters: Arc<RwLock<Vec<DeadLetter>>>,
    /// Message ID counter
    id_counter: Arc<parking_lot::Mutex<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetter {
    pub message: BusMessage,
    pub error: String,
    pub failed_at: DateTime<Utc>,
}

/// Channel/topic for message routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// Agent lifecycle events
    AgentEvents,
    /// Task status updates
    TaskUpdates,
    /// Inter-agent messages
    AgentMessages,
    /// System events
    SystemEvents,
    /// Team-specific channel
    Team,
    /// Direct agent communication
    Direct,
    /// Error events
    Errors,
    /// Metrics
    Metrics,
}

/// Wrapper message for the bus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusMessage {
    pub id: u64,
    pub source: MessageSource,
    pub channel: Channel,
    pub payload: MessagePayload,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: Option<u64>,
}

/// Source of a message
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageSource {
    Agent,
    Team,
    System,
    User,
}

/// Message payload types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MessagePayload {
    // Agent events
    AgentStarted {
        agent_id: AgentId,
    },
    AgentCompleted {
        agent_id: AgentId,
        outputs: Vec<String>,
    },
    AgentFailed {
        agent_id: AgentId,
        error: String,
    },
    AgentWaiting {
        agent_id: AgentId,
        reason: String,
    },

    // Task events
    TaskAssigned {
        task_id: Uuid,
        agent_id: AgentId,
    },
    TaskProgress {
        task_id: Uuid,
        progress: f32,
    },
    TaskCompleted {
        task_id: Uuid,
        result: String,
    },
    TaskFailed {
        task_id: Uuid,
        error: String,
    },

    // Inter-agent messages
    Delegation {
        from: AgentId,
        to: AgentId,
        task: String,
    },
    Request {
        request_type: String,
        data: serde_json::Value,
    },
    Response {
        request_id: Uuid,
        data: serde_json::Value,
    },
    Broadcast {
        message: String,
    },

    // Context sharing
    ContextUpdate {
        agent_id: AgentId,
        variables: HashMap<String, serde_json::Value>,
    },
    ArtifactCreated {
        path: String,
        created_by: AgentId,
    },

    // System events
    TeamCreated {
        team_id: TeamId,
    },
    TeamDissolved {
        team_id: TeamId,
    },
    Shutdown,
}

impl MessageBus {
    /// Create a new message bus
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            dead_letters: Arc::new(RwLock::new(Vec::new())),
            id_counter: Arc::new(parking_lot::Mutex::new(0)),
        }
    }

    /// Subscribe to a channel
    pub fn subscribe(&self, channel: Channel) -> Result<broadcast::Receiver<BusMessage>, BusError> {
        let sender = {
            let mut subs = self.subscribers.write();
            if !subs.contains_key(&channel) {
                let (tx, rx) = broadcast::channel(1000);
                subs.insert(channel, tx);
                return Ok(rx);
            }
            subs.get(&channel).cloned().unwrap()
        };
        Ok(sender.subscribe())
    }

    /// Publish a message to a channel
    pub fn publish(&self, channel: Channel, payload: MessagePayload) -> Result<u64, BusError> {
        let id = self.next_id();
        let message = BusMessage {
            id,
            source: MessageSource::System,
            channel,
            payload,
            timestamp: Utc::now(),
            correlation_id: None,
        };

        let sender = self.subscribers.read().get(&channel).cloned();
        match sender {
            Some(tx) => {
                tx.send(message)
                    .map_err(|e| BusError::SendFailed(e.to_string()))?;
            }
            None => {
                tracing::warn!("No subscribers for channel {:?}", channel);
            }
        }
        Ok(id)
    }

    /// Publish from a specific source
    pub fn publish_from(
        &self,
        source: MessageSource,
        channel: Channel,
        payload: MessagePayload,
    ) -> Result<u64, BusError> {
        let id = self.next_id();
        let message = BusMessage {
            id,
            source,
            channel,
            payload,
            timestamp: Utc::now(),
            correlation_id: None,
        };

        let sender = self.subscribers.read().get(&channel).cloned();
        if let Some(tx) = sender {
            let _ = tx.send(message);
        }
        Ok(id)
    }

    /// Send direct message to an agent
    pub fn send_direct(
        &self,
        to: AgentId,
        from: MessageSource,
        payload: MessagePayload,
    ) -> Result<u64, BusError> {
        let channel = Channel::Direct;
        let id = self.next_id();
        let message = BusMessage {
            id,
            source: from,
            channel,
            payload,
            timestamp: Utc::now(),
            correlation_id: None,
        };

        let sender = self.subscribers.read().get(&channel).cloned();
        if let Some(tx) = sender {
            let _ = tx.send(message);
        }
        Ok(id)
    }

    /// Handle dead letter
    pub fn handle_dead_letter(&self, message: BusMessage, error: String) {
        let dl = DeadLetter {
            message,
            error,
            failed_at: Utc::now(),
        };
        let mut queue = self.dead_letters.write();
        queue.push(dl);
        if queue.len() > 10000 {
            queue.remove(0);
        }
    }

    fn next_id(&self) -> u64 {
        let mut counter = self.id_counter.lock();
        *counter += 1;
        *counter
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}
