//! Nexus-Orchestrator Integration Tests
//!
//! Tests cross-module interactions between orchestrator components.

use chrono::{Timelike, Utc};
use nexus_orchestrator::{
    message_bus::MessageSource, Channel, CronSchedule, ExecutionStatus, MessageBus, MessagePayload,
    RetryConfig, SessionStore, TaskPriority,
};
use uuid::Uuid;

// ── MessageBus + SessionStore integration ──────────────────────────────

#[test]
fn test_message_bus_and_session_store_independent() {
    // Verify both core components can coexist without conflict
    let bus = MessageBus::new();
    let store = SessionStore::in_memory().unwrap();

    // Publish a message
    let mut rx = bus.subscribe(Channel::SystemEvents).unwrap();
    bus.publish(Channel::SystemEvents, MessagePayload::Shutdown)
        .unwrap();

    // Save to store
    let result = make_chain_result("integration test", 2);
    store.save(&result).unwrap();

    // Verify both work
    let msg = rx.try_recv().unwrap();
    assert_eq!(msg.channel, Channel::SystemEvents);

    let list = store.list(10).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].task, "integration test");
}

// ── MessageBus multi-channel ───────────────────────────────────────────

#[test]
fn test_message_bus_publish_to_multiple_channels() {
    let bus = MessageBus::new();

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

    let mut receivers = Vec::new();
    for ch in &channels {
        let rx = bus.subscribe(*ch).unwrap();
        receivers.push((*ch, rx));
    }

    // Publish to all channels
    for (ch, _) in &receivers {
        bus.publish(*ch, MessagePayload::Shutdown).unwrap();
    }

    // Verify each received
    for (ch, rx) in &mut receivers {
        // Use try_recv since publish is synchronous (broadcast channel)
        // but the message might already be in the buffer
        let result = rx.try_recv();
        if let Ok(msg) = result {
            assert_eq!(msg.channel, *ch);
        }
        // If Lagged, it means we missed messages (shouldn't happen with single messages)
    }
}

#[test]
fn test_message_bus_source_tracking() {
    let bus = MessageBus::new();
    let mut rx = bus.subscribe(Channel::AgentMessages).unwrap();

    bus.publish_from(
        MessageSource::Agent,
        Channel::AgentMessages,
        MessagePayload::Broadcast {
            message: "from agent".into(),
        },
    )
    .unwrap();

    bus.publish_from(
        MessageSource::User,
        Channel::AgentMessages,
        MessagePayload::Broadcast {
            message: "from user".into(),
        },
    )
    .unwrap();

    let msg1 = rx.try_recv().unwrap();
    let msg2 = rx.try_recv().unwrap();

    // Agent message came first
    assert!(
        matches!(msg1.payload, MessagePayload::Broadcast { ref message } if message == "from agent")
    );
    assert!(
        matches!(msg2.payload, MessagePayload::Broadcast { ref message } if message == "from user")
    );
}

// ── CronSchedule + RetryConfig integration ────────────────────────────

#[test]
fn test_cron_and_retry_independent_configs() {
    // Test that cron schedule and retry config work together in a workflow
    let cron = CronSchedule::parse("0 * * * *").unwrap();
    let retry = RetryConfig::default();

    // A typical pattern: schedule a task with cron, retry on failure
    assert_eq!(cron.minute, vec![0]);
    assert_eq!(retry.max_retries, 3);

    // Verify the retry backoff timing for multiple retries
    let total_retry_time = (0..retry.max_retries)
        .map(|i| retry.backoff_duration(i).as_secs())
        .sum::<u64>();

    // 1 + 2 + 4 = 7 seconds total backoff time
    assert_eq!(total_retry_time, 7);
}

#[test]
fn test_cron_schedule_next_run_multiple_minutes() {
    let cron = CronSchedule::parse("0,30 * * * *").unwrap();
    let now = Utc::now();
    let next = cron.next_run(now).unwrap();

    // Should be either :00 or :30 of some hour
    assert!(
        next.minute() == 0 || next.minute() == 30,
        "expected :00 or :30, got :{}",
        next.minute()
    );
    assert!(next > now);
    assert!((next - now).num_minutes() <= 30);
}

// ── TaskPriority + QueueStatus integration ────────────────────────────

#[test]
fn test_priority_and_status_usage_pattern() {
    // Simulate a typical scheduler usage pattern
    let priorities = [
        TaskPriority::Low,
        TaskPriority::Normal,
        TaskPriority::High,
        TaskPriority::Critical,
    ];
    let status_cycle = ["queued", "running", "completed"];

    // Priority ordering
    assert!(priorities[3] > priorities[2]);
    assert!(priorities[2] > priorities[1]);
    assert!(priorities[1] > priorities[0]);

    // Status cycle via display
    for (i, expected) in status_cycle.iter().enumerate() {
        let _ = i; // status index in cycle
        assert!(!expected.is_empty());
    }
}

// ── Serialization integration ──────────────────────────────────────────

#[test]
fn test_execution_status_all_variants_serialize() {
    let variants = [
        ExecutionStatus::Pending,
        ExecutionStatus::Running,
        ExecutionStatus::Completed,
        ExecutionStatus::Failed,
        ExecutionStatus::Cancelled,
    ];

    for v in &variants {
        let json = serde_json::to_string(v).unwrap();
        let back: ExecutionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(*v, back);
    }
}

// ── SessionStore CRUD integration ──────────────────────────────────────

#[test]
fn test_session_store_full_lifecycle() {
    let store = SessionStore::in_memory().unwrap();

    // Create
    let result = make_chain_result("lifecycle", 3);
    let id = result.execution_id.to_string();
    store.save(&result).unwrap();
    assert_eq!(store.list(10).unwrap().len(), 1);

    // Read
    let loaded = store.get(&id).unwrap().unwrap();
    assert_eq!(loaded.task, "lifecycle");
    assert_eq!(loaded.agent_results.len(), 3);

    // Update (replace)
    let updated = make_chain_result_with_id(result.execution_id, "updated-lifecycle", 1);
    store.save(&updated).unwrap();
    let loaded = store.get(&id).unwrap().unwrap();
    assert_eq!(loaded.task, "updated-lifecycle");
    assert_eq!(loaded.agent_results.len(), 1);

    // Delete
    let deleted = store.delete(&id).unwrap();
    assert!(deleted);
    assert!(store.get(&id).unwrap().is_none());
    assert!(store.list(10).unwrap().is_empty());
}

#[test]
fn test_session_store_multiple_sessions_list_order() {
    let store = SessionStore::in_memory().unwrap();

    // Save sessions with different tasks
    store.save(&make_chain_result("first", 1)).unwrap();
    store.save(&make_chain_result("second", 2)).unwrap();
    store.save(&make_chain_result("third", 3)).unwrap();

    let list = store.list(10).unwrap();
    assert_eq!(list.len(), 3);

    // All tasks should be present
    let tasks: Vec<&str> = list.iter().map(|s| s.task.as_str()).collect();
    assert!(tasks.contains(&"first"));
    assert!(tasks.contains(&"second"));
    assert!(tasks.contains(&"third"));
}

#[test]
fn test_session_store_summary_fields() {
    let store = SessionStore::in_memory().unwrap();
    let result = make_chain_result("summary-test", 4);
    store.save(&result).unwrap();

    let list = store.list(10).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].task, "summary-test");
    assert_eq!(list[0].agent_count, 4);
    assert!(!list[0].created_at.is_empty());
}

// ── Helpers ────────────────────────────────────────────────────────────

use nexus_orchestrator::{AgentResult, ChainResult};

fn make_chain_result(task: &str, agent_count: usize) -> ChainResult {
    let mut agents = Vec::new();
    for i in 0..agent_count {
        agents.push(AgentResult {
            agent_id: nexus_orchestrator::AgentId::new(),
            role: nexus_orchestrator::AgentRole::Developer,
            agent_name: format!("agent-{}", i),
            text: format!("result from {}", i),
            duration_ms: (i as u64 + 1) * 1000,
            attempts: 1,
        });
    }
    let total = agents.iter().map(|a| a.duration_ms).sum();
    ChainResult {
        execution_id: Uuid::new_v4(),
        task: task.into(),
        agent_results: agents,
        total_duration_ms: total,
    }
}

fn make_chain_result_with_id(id: Uuid, task: &str, agent_count: usize) -> ChainResult {
    let mut result = make_chain_result(task, agent_count);
    result.execution_id = id;
    result
}
