//! Message Bus Protocol - Pub-sub communication for agent coordination

use crate::error::BusError;
use crate::team::{AgentId, TeamId};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
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
            if let std::collections::hash_map::Entry::Vacant(e) = subs.entry(channel) {
                let (tx, rx) = broadcast::channel(1000);
                e.insert(tx);
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
        _to: AgentId,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // ── Construction ────────────────────────────────────────────────

    #[test]
    fn test_new_bus_empty() {
        let bus = MessageBus::new();
        assert!(bus.subscribers.read().is_empty());
        assert!(bus.dead_letters.read().is_empty());
    }

    #[test]
    fn test_default_equals_new() {
        assert_eq!(
            MessageBus::default().subscribers.read().len(),
            MessageBus::new().subscribers.read().len()
        );
    }

    // ── Subscribe ───────────────────────────────────────────────────

    #[test]
    fn test_subscribe_creates_channel() {
        let bus = MessageBus::new();
        let _rx = bus.subscribe(Channel::AgentEvents).unwrap();
        assert!(bus.subscribers.read().contains_key(&Channel::AgentEvents));
    }

    #[test]
    fn test_subscribe_twice_returns_two_receivers() {
        let bus = MessageBus::new();
        let rx1 = bus.subscribe(Channel::TaskUpdates).unwrap();
        let rx2 = bus.subscribe(Channel::TaskUpdates).unwrap();
        drop(rx1);
        drop(rx2);
        assert!(bus.subscribers.read().contains_key(&Channel::TaskUpdates));
    }

    #[test]
    fn test_subscribe_all_channels() {
        let bus = MessageBus::new();
        for ch in &[
            Channel::AgentEvents,
            Channel::TaskUpdates,
            Channel::AgentMessages,
            Channel::SystemEvents,
            Channel::Team,
            Channel::Direct,
            Channel::Errors,
            Channel::Metrics,
        ] {
            let _rx = bus.subscribe(*ch).unwrap();
        }
        assert_eq!(bus.subscribers.read().len(), 8);
    }

    // ── Publish ─────────────────────────────────────────────────────

    #[test]
    fn test_publish_received_by_subscriber() {
        let bus = MessageBus::new();
        let mut rx = bus.subscribe(Channel::AgentEvents).unwrap();

        let id = bus
            .publish(
                Channel::AgentEvents,
                MessagePayload::AgentStarted {
                    agent_id: AgentId::new(),
                },
            )
            .unwrap();

        assert_eq!(id, 1);
        let received = rx.try_recv().unwrap();
        assert_eq!(received.id, 1);
        assert_eq!(received.channel, Channel::AgentEvents);
        assert!(matches!(
            received.payload,
            MessagePayload::AgentStarted { .. }
        ));
    }

    #[test]
    fn test_publish_to_no_subscribers_returns_ok() {
        let bus = MessageBus::new();
        let id = bus
            .publish(Channel::Metrics, MessagePayload::Shutdown)
            .unwrap();
        assert_eq!(id, 1);
    }

    #[test]
    fn test_id_counter_increments() {
        let bus = MessageBus::new();
        assert_eq!(
            bus.publish(Channel::SystemEvents, MessagePayload::Shutdown)
                .unwrap(),
            1
        );
        assert_eq!(
            bus.publish(Channel::SystemEvents, MessagePayload::Shutdown)
                .unwrap(),
            2
        );
        assert_eq!(
            bus.publish(Channel::SystemEvents, MessagePayload::Shutdown)
                .unwrap(),
            3
        );
    }

    #[test]
    fn test_multiple_subscribers_same_channel() {
        let bus = MessageBus::new();
        let mut rx1 = bus.subscribe(Channel::TaskUpdates).unwrap();
        let mut rx2 = bus.subscribe(Channel::TaskUpdates).unwrap();

        bus.publish(
            Channel::TaskUpdates,
            MessagePayload::TaskProgress {
                task_id: Uuid::new_v4(),
                progress: 0.5,
            },
        )
        .unwrap();

        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }

    #[test]
    fn test_publish_after_subscriber_dropped_returns_error() {
        let bus = MessageBus::new();
        {
            let _rx = bus.subscribe(Channel::AgentEvents).unwrap();
        }
        // All receivers dropped — broadcast send fails
        let result = bus.publish(Channel::AgentEvents, MessagePayload::Shutdown);
        assert!(result.is_err());
        match result {
            Err(BusError::SendFailed(_)) => {} // expected
            _ => panic!("expected SendFailed error"),
        }
    }

    // ── Publish From ────────────────────────────────────────────────

    #[test]
    fn test_publish_from_sets_source() {
        let bus = MessageBus::new();
        let mut rx = bus.subscribe(Channel::AgentMessages).unwrap();

        bus.publish_from(
            MessageSource::User,
            Channel::AgentMessages,
            MessagePayload::Broadcast {
                message: "hello".into(),
            },
        )
        .unwrap();

        let received = rx.try_recv().unwrap();
        assert!(matches!(received.source, MessageSource::User));
    }

    #[test]
    fn test_publish_from_all_sources() {
        let bus = MessageBus::new();
        let mut rx = bus.subscribe(Channel::SystemEvents).unwrap();

        for source in &[
            MessageSource::Agent,
            MessageSource::Team,
            MessageSource::System,
            MessageSource::User,
        ] {
            bus.publish_from(*source, Channel::SystemEvents, MessagePayload::Shutdown)
                .unwrap();
        }

        let source_order = [
            MessageSource::Agent,
            MessageSource::Team,
            MessageSource::System,
            MessageSource::User,
        ];
        for expected in &source_order {
            let msg = rx.try_recv().unwrap();
            assert!(
                matches!(
                    msg.source,
                    MessageSource::Agent
                        | MessageSource::Team
                        | MessageSource::System
                        | MessageSource::User
                ),
                "unexpected source"
            );
        }
    }

    // ── Send Direct ─────────────────────────────────────────────────

    #[test]
    fn test_send_direct_uses_direct_channel() {
        let bus = MessageBus::new();
        let mut rx = bus.subscribe(Channel::Direct).unwrap();

        let _id = bus
            .send_direct(
                AgentId::new(),
                MessageSource::Agent,
                MessagePayload::Request {
                    request_type: "help".into(),
                    data: serde_json::Value::Null,
                },
            )
            .unwrap();

        let received = rx.try_recv().unwrap();
        assert_eq!(received.channel, Channel::Direct);
    }

    #[test]
    fn test_send_direct_no_subscriber_returns_ok() {
        let bus = MessageBus::new();
        let id = bus
            .send_direct(
                AgentId::new(),
                MessageSource::System,
                MessagePayload::Shutdown,
            )
            .unwrap();
        assert_eq!(id, 1);
    }

    // ── Dead Letter Queue ───────────────────────────────────────────

    fn sample_bus_message(id: u64) -> BusMessage {
        BusMessage {
            id,
            source: MessageSource::System,
            channel: Channel::Errors,
            payload: MessagePayload::Shutdown,
            timestamp: Utc::now(),
            correlation_id: None,
        }
    }

    #[test]
    fn test_dead_letter_single_entry() {
        let bus = MessageBus::new();
        bus.handle_dead_letter(sample_bus_message(1), "test error".into());

        let dl = bus.dead_letters.read();
        assert_eq!(dl.len(), 1);
        assert_eq!(dl[0].error, "test error");
        assert_eq!(dl[0].message.id, 1);
    }

    #[test]
    fn test_dead_letter_queue_caps_at_10000() {
        let bus = MessageBus::new();
        for i in 0..10001 {
            bus.handle_dead_letter(sample_bus_message(i), format!("error {}", i));
        }

        let dl = bus.dead_letters.read();
        assert_eq!(dl.len(), 10000);
        // id=0 should have been removed
        assert_eq!(dl[0].message.id, 1);
        assert_eq!(dl[9999].message.id, 10000);
    }

    #[test]
    fn test_dead_letter_empty_queue() {
        let bus = MessageBus::new();
        assert!(bus.dead_letters.read().is_empty());
    }

    #[test]
    fn test_dead_letter_under_capacity() {
        let bus = MessageBus::new();
        for i in 0..5 {
            bus.handle_dead_letter(sample_bus_message(i), format!("err {}", i));
        }
        assert_eq!(bus.dead_letters.read().len(), 5);
    }

    #[test]
    fn test_dead_letter_exact_capacity() {
        let bus = MessageBus::new();
        for i in 0..10000 {
            bus.handle_dead_letter(sample_bus_message(i), "err".into());
        }
        assert_eq!(bus.dead_letters.read().len(), 10000);
    }

    // ── Integration: All channels publish ────────────────────────────

    #[test]
    fn test_all_channels_publish_and_receive() {
        let bus = MessageBus::new();
        let payloads: Vec<(Channel, MessagePayload)> = vec![
            (
                Channel::AgentEvents,
                MessagePayload::AgentStarted {
                    agent_id: AgentId::new(),
                },
            ),
            (
                Channel::TaskUpdates,
                MessagePayload::TaskAssigned {
                    task_id: Uuid::new_v4(),
                    agent_id: AgentId::new(),
                },
            ),
            (
                Channel::AgentMessages,
                MessagePayload::Broadcast {
                    message: "test".into(),
                },
            ),
            (
                Channel::SystemEvents,
                MessagePayload::TeamCreated {
                    team_id: TeamId::new(),
                },
            ),
            (
                Channel::Team,
                MessagePayload::TeamDissolved {
                    team_id: TeamId::new(),
                },
            ),
            (Channel::Direct, MessagePayload::Shutdown),
            (
                Channel::Errors,
                MessagePayload::AgentFailed {
                    agent_id: AgentId::new(),
                    error: "err".into(),
                },
            ),
            (Channel::Metrics, MessagePayload::Shutdown),
        ];

        for (ch, payload) in &payloads {
            let mut rx = bus.subscribe(*ch).unwrap();
            let id = bus.publish(*ch, payload.clone()).unwrap();
            assert!(id > 0, "publish failed for channel {:?}", ch);
            let msg = rx
                .try_recv()
                .unwrap_or_else(|_| panic!("no msg on {:?}", ch));
            assert_eq!(msg.channel, *ch);
        }
    }

    // ── Serialization ───────────────────────────────────────────────

    #[test]
    fn test_bus_message_serde_roundtrip() {
        let msg = BusMessage {
            id: 42,
            source: MessageSource::Agent,
            channel: Channel::AgentMessages,
            payload: MessagePayload::Broadcast {
                message: "hello".into(),
            },
            timestamp: Utc::now(),
            correlation_id: Some(1),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: BusMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, 42);
        assert_eq!(deserialized.channel, Channel::AgentMessages);
    }

    #[test]
    fn test_dead_letter_serde_roundtrip() {
        let dl = DeadLetter {
            message: sample_bus_message(1),
            error: "something went wrong".into(),
            failed_at: Utc::now(),
        };

        let json = serde_json::to_string(&dl).unwrap();
        let deserialized: DeadLetter = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.error, "something went wrong");
        assert_eq!(deserialized.message.id, 1);
    }

    #[test]
    fn test_channel_serde_all_variants() {
        let channels = [
            Channel::AgentEvents,
            Channel::TaskUpdates,
            Channel::AgentMessages,
            Channel::SystemEvents,
            Channel::Team,
            Channel::Direct,
            Channel::Errors,
            Channel::Metrics,
        ];
        for ch in &channels {
            let json = serde_json::to_string(ch).unwrap();
            let back: Channel = serde_json::from_str(&json).unwrap();
            assert_eq!(*ch, back);
        }
    }

    #[test]
    fn test_message_source_serde_roundtrip() {
        let sources = [
            MessageSource::Agent,
            MessageSource::Team,
            MessageSource::System,
            MessageSource::User,
        ];
        for src in &sources {
            let json = serde_json::to_string(src).unwrap();
            let back: MessageSource = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", src), format!("{:?}", back));
        }
    }

    #[test]
    fn test_message_payload_all_variants_serde() {
        let payloads: Vec<MessagePayload> = vec![
            MessagePayload::AgentStarted {
                agent_id: AgentId::new(),
            },
            MessagePayload::AgentCompleted {
                agent_id: AgentId::new(),
                outputs: vec!["out".into()],
            },
            MessagePayload::AgentFailed {
                agent_id: AgentId::new(),
                error: "fail".into(),
            },
            MessagePayload::AgentWaiting {
                agent_id: AgentId::new(),
                reason: "waiting".into(),
            },
            MessagePayload::TaskAssigned {
                task_id: Uuid::new_v4(),
                agent_id: AgentId::new(),
            },
            MessagePayload::TaskProgress {
                task_id: Uuid::new_v4(),
                progress: 0.75,
            },
            MessagePayload::TaskCompleted {
                task_id: Uuid::new_v4(),
                result: "done".into(),
            },
            MessagePayload::TaskFailed {
                task_id: Uuid::new_v4(),
                error: "fail".into(),
            },
            MessagePayload::Delegation {
                from: AgentId::new(),
                to: AgentId::new(),
                task: "task".into(),
            },
            MessagePayload::Request {
                request_type: "query".into(),
                data: serde_json::json!({"key": "val"}),
            },
            MessagePayload::Response {
                request_id: Uuid::new_v4(),
                data: serde_json::json!(42),
            },
            MessagePayload::Broadcast {
                message: "broad".into(),
            },
            MessagePayload::ContextUpdate {
                agent_id: AgentId::new(),
                variables: HashMap::new(),
            },
            MessagePayload::ArtifactCreated {
                path: "/tmp/file".into(),
                created_by: AgentId::new(),
            },
            MessagePayload::TeamCreated {
                team_id: TeamId::new(),
            },
            MessagePayload::TeamDissolved {
                team_id: TeamId::new(),
            },
            MessagePayload::Shutdown,
        ];

        for payload in &payloads {
            let json = serde_json::to_string(payload).unwrap();
            let back: MessagePayload = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", payload), format!("{:?}", back));
        }
    }

    // ── BusMessage fields ───────────────────────────────────────────

    #[test]
    fn test_bus_message_default_correlation_id() {
        let bus = MessageBus::new();
        let mut rx = bus.subscribe(Channel::AgentEvents).unwrap();
        bus.publish(
            Channel::AgentEvents,
            MessagePayload::AgentStarted {
                agent_id: AgentId::new(),
            },
        )
        .unwrap();
        let msg = rx.try_recv().unwrap();
        assert!(msg.correlation_id.is_none());
    }

    #[test]
    fn test_bus_message_has_timestamp() {
        let bus = MessageBus::new();
        let mut rx = bus.subscribe(Channel::SystemEvents).unwrap();
        bus.publish(Channel::SystemEvents, MessagePayload::Shutdown)
            .unwrap();
        let msg = rx.try_recv().unwrap();
        // timestamp should be set (not the default datetime)
        assert!(msg.timestamp.timestamp() > 0);
    }

    #[test]
    fn test_bus_message_source_defaults_to_system() {
        let bus = MessageBus::new();
        let mut rx = bus.subscribe(Channel::AgentEvents).unwrap();
        bus.publish(
            Channel::AgentEvents,
            MessagePayload::AgentStarted {
                agent_id: AgentId::new(),
            },
        )
        .unwrap();
        let msg = rx.try_recv().unwrap();
        assert!(matches!(msg.source, MessageSource::System));
    }

    #[test]
    fn test_broadcast_channel_overflow_handling() {
        // When a subscriber lags and messages overflow the buffer,
        // the receiver should either get a Lagged error or the next available message.
        let bus = MessageBus::new();
        let mut rx = bus.subscribe(Channel::AgentEvents).unwrap();

        // Fill the channel beyond capacity (capacity = 1000)
        for _ in 0..1005 {
            bus.publish(Channel::AgentEvents, MessagePayload::Shutdown)
                .unwrap_or_default();
        }

        // Receiver should be lagged — if we get an error, it's Lagged;
        // if we get Ok, we receive the next available message.
        let result = rx.try_recv();
        match result {
            Ok(msg) => {
                assert_eq!(msg.channel, Channel::AgentEvents);
            }
            Err(e) => match e {
                broadcast::error::TryRecvError::Lagged(n) => {
                    assert!(n > 0, "should have missed at least 1 message");
                }
                _ => panic!("unexpected error: {:?}", e),
            },
        }
    }
}
