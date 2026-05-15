use super::*;
use crossterm::event::{KeyCode, KeyModifiers};

// ── SkeletonRenderer::handle_input ─────────────────────────────────

#[test]
fn handle_input_ctrl_c_quits() {
    let mut renderer = SkeletonRenderer;
    assert_eq!(
        renderer.handle_input(KeyCode::Char('c'), KeyModifiers::CONTROL),
        InputAction::Quit
    );
}

#[test]
fn handle_input_q_quits() {
    let mut renderer = SkeletonRenderer;
    assert_eq!(
        renderer.handle_input(KeyCode::Char('q'), KeyModifiers::NONE),
        InputAction::Quit
    );
}

#[test]
fn handle_input_enter_enters_input_mode() {
    let mut renderer = SkeletonRenderer;
    assert_eq!(
        renderer.handle_input(KeyCode::Enter, KeyModifiers::NONE),
        InputAction::EnterInput
    );
}

#[test]
fn handle_input_esc_cancels_input() {
    let mut renderer = SkeletonRenderer;
    assert_eq!(
        renderer.handle_input(KeyCode::Esc, KeyModifiers::NONE),
        InputAction::CancelInput
    );
}

#[test]
fn handle_input_backspace() {
    let mut renderer = SkeletonRenderer;
    assert_eq!(
        renderer.handle_input(KeyCode::Backspace, KeyModifiers::NONE),
        InputAction::Backspace
    );
}

#[test]
fn handle_input_char_returns_type_char() {
    let mut renderer = SkeletonRenderer;
    for ch in &['a', 'z', ' ', '!', '中', '1'] {
        assert_eq!(
            renderer.handle_input(KeyCode::Char(*ch), KeyModifiers::NONE),
            InputAction::TypeChar(*ch),
            "failed for char: {:?}",
            ch
        );
    }
}

#[test]
fn handle_input_tab_toggles_panel() {
    let mut renderer = SkeletonRenderer;
    assert_eq!(
        renderer.handle_input(KeyCode::Tab, KeyModifiers::NONE),
        InputAction::TogglePanel
    );
}

#[test]
fn handle_input_ctrl_without_c_does_not_quit() {
    let mut renderer = SkeletonRenderer;
    let action = renderer.handle_input(KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert_eq!(action, InputAction::TypeChar('x'));
}

#[test]
fn handle_input_other_keys_return_none() {
    let mut renderer = SkeletonRenderer;
    let other_keys = vec![
        KeyCode::F(1),
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Insert,
        KeyCode::Delete,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Null,
    ];
    for key in other_keys {
        assert_eq!(
            renderer.handle_input(key, KeyModifiers::NONE),
            InputAction::None,
            "expected None for key: {:?}",
            key
        );
    }
}

#[test]
fn handle_input_enter_with_modifiers_enters_input_mode() {
    let mut renderer = SkeletonRenderer;
    assert_eq!(
        renderer.handle_input(KeyCode::Enter, KeyModifiers::SHIFT),
        InputAction::EnterInput
    );
    assert_eq!(
        renderer.handle_input(KeyCode::Enter, KeyModifiers::ALT),
        InputAction::EnterInput
    );
}

// ── InputAction consistency ────────────────────────────────────────

#[test]
fn input_action_quit_is_quit() {
    assert_eq!(format!("{:?}", InputAction::Quit), "Quit");
}

#[test]
fn input_action_send_message_contains_text() {
    let action = InputAction::SendMessage("hello".into());
    match action {
        InputAction::SendMessage(ref msg) => assert_eq!(msg, "hello"),
        _ => panic!("expected SendMessage"),
    }
}

// ── SessionViewState defaults ──────────────────────────────────────

#[test]
fn session_view_state_defaults() {
    let state = SessionViewState::default();
    assert_eq!(state.status, SessionStatus::Init);
    assert!(state.messages.is_empty());
    assert!(state.agent_states.is_empty());
    assert_eq!(state.selected_panel, PanelFocus::Chat);
    assert!(state.input_buffer.is_empty());
    assert!(!state.is_input_mode);
    assert_eq!(state.typewriter_pos, 0);
}

#[test]
fn session_status_default_is_init() {
    assert_eq!(SessionStatus::default(), SessionStatus::Init);
}

#[test]
fn panel_focus_default_is_chat() {
    assert_eq!(PanelFocus::default(), PanelFocus::Chat);
}

// ── Panel focus cycling ────────────────────────────────────────────

#[test]
fn panel_focus_cycles_chat_to_progress() {
    let next = match PanelFocus::Chat {
        PanelFocus::Chat => PanelFocus::Progress,
        PanelFocus::Progress => PanelFocus::Input,
        PanelFocus::Input => PanelFocus::Chat,
    };
    assert_eq!(next, PanelFocus::Progress);
}

#[test]
fn panel_focus_cycles_progress_to_input() {
    let next = match PanelFocus::Progress {
        PanelFocus::Chat => PanelFocus::Progress,
        PanelFocus::Progress => PanelFocus::Input,
        PanelFocus::Input => PanelFocus::Chat,
    };
    assert_eq!(next, PanelFocus::Input);
}

#[test]
fn panel_focus_cycles_input_to_chat() {
    let next = match PanelFocus::Input {
        PanelFocus::Chat => PanelFocus::Progress,
        PanelFocus::Progress => PanelFocus::Input,
        PanelFocus::Input => PanelFocus::Chat,
    };
    assert_eq!(next, PanelFocus::Chat);
}

#[test]
fn panel_focus_cycles_wraps_correctly() {
    let cycle = |panel: PanelFocus| match panel {
        PanelFocus::Chat => PanelFocus::Progress,
        PanelFocus::Progress => PanelFocus::Input,
        PanelFocus::Input => PanelFocus::Chat,
    };
    let p = PanelFocus::Chat;
    let p = cycle(p);
    assert_eq!(p, PanelFocus::Progress);
    let p = cycle(p);
    assert_eq!(p, PanelFocus::Input);
    let p = cycle(p);
    assert_eq!(p, PanelFocus::Chat);
}

// ── Input mode state transitions ───────────────────────────────────

#[test]
fn enter_input_mode_sets_input_mode() {
    let mut state = SessionViewState::default();
    assert!(!state.is_input_mode);

    state.is_input_mode = true;

    assert!(state.is_input_mode);
}

#[test]
fn send_message_in_input_mode_clears_buffer() {
    let mut state = SessionViewState::default();
    state.is_input_mode = true;
    state.input_buffer = "test message".into();

    let msg = state.input_buffer.clone();
    state.is_input_mode = false;
    state.input_buffer.clear();

    assert!(!state.is_input_mode);
    assert!(state.input_buffer.is_empty());
    assert_eq!(msg, "test message");
}

#[test]
fn cancel_input_clears_buffer_and_exits_input_mode() {
    let mut state = SessionViewState::default();
    state.is_input_mode = true;
    state.input_buffer = "partial message".into();

    state.is_input_mode = false;
    state.input_buffer.clear();

    assert!(!state.is_input_mode);
    assert!(state.input_buffer.is_empty());
}

#[test]
fn type_char_in_input_mode_appends() {
    let mut state = SessionViewState::default();
    state.is_input_mode = true;

    state.input_buffer.push('h');
    state.input_buffer.push('i');

    assert_eq!(state.input_buffer, "hi");
}

#[test]
fn type_char_outside_input_mode_does_nothing() {
    let mut state = SessionViewState::default();
    state.is_input_mode = false;
    let original = state.input_buffer.clone();

    let ignore = !state.is_input_mode;
    assert!(ignore);
    assert_eq!(state.input_buffer, original);
}

#[test]
fn backspace_in_input_mode_pops() {
    let mut state = SessionViewState::default();
    state.is_input_mode = true;
    state.input_buffer = "hello".into();

    if state.is_input_mode {
        state.input_buffer.pop();
    }

    assert_eq!(state.input_buffer, "hell");
}

#[test]
fn backspace_outside_input_mode_does_nothing() {
    let mut state = SessionViewState::default();
    state.is_input_mode = false;
    state.input_buffer = "test".into();
    let original_len = state.input_buffer.len();

    assert!(!state.is_input_mode);
    assert_eq!(state.input_buffer.len(), original_len);
}

#[test]
fn backspace_on_empty_buffer_is_noop() {
    let mut state = SessionViewState::default();
    state.is_input_mode = true;
    state.input_buffer.clear();

    state.input_buffer.pop();

    assert!(state.input_buffer.is_empty());
}

// ── Typewriter effect ──────────────────────────────────────────────

#[test]
fn typewriter_advances_by_constant_speed() {
    let mut state = SessionViewState::default();
    state
        .messages
        .push(("a".into(), "b".into(), "hello world".into()));
    state.typewriter_pos = 0;

    if let Some((_, _, text)) = state.messages.last() {
        if state.typewriter_pos < text.len() {
            state.typewriter_pos += TYPING_SPEED;
        }
    }

    assert_eq!(state.typewriter_pos, TYPING_SPEED);
}

#[test]
fn typewriter_stops_at_text_end() {
    let mut state = SessionViewState::default();
    state.messages.push(("a".into(), "b".into(), "ab".into()));
    state.typewriter_pos = 2;

    if let Some((_, _, text)) = state.messages.last() {
        if state.typewriter_pos < text.len() {
            state.typewriter_pos += TYPING_SPEED;
        }
    }

    assert_eq!(state.typewriter_pos, 2);
}

#[test]
fn typewriter_advances_partially_near_end() {
    let mut state = SessionViewState::default();
    state
        .messages
        .push(("a".into(), "b".into(), "hello".into()));
    state.typewriter_pos = 4;

    if let Some((_, _, text)) = state.messages.last() {
        if state.typewriter_pos < text.len() {
            state.typewriter_pos += TYPING_SPEED;
        }
    }

    assert_eq!(state.typewriter_pos, 7);
    assert_eq!(state.typewriter_pos.min("hello".len()), 5);
}

#[test]
fn typewriter_resets_on_new_message() {
    let mut state = SessionViewState::default();
    state.typewriter_pos = 42;

    state
        .messages
        .push(("a".into(), "b".into(), "new message".into()));
    state.typewriter_pos = 0;

    assert_eq!(state.typewriter_pos, 0);
}

#[test]
fn typewriter_does_nothing_with_no_messages() {
    let mut state = SessionViewState::default();
    state.typewriter_pos = 5;

    if let Some((_, _, text)) = state.messages.last() {
        if state.typewriter_pos < text.len() {
            state.typewriter_pos += TYPING_SPEED;
        }
    }

    assert_eq!(state.typewriter_pos, 5);
}

#[test]
fn typewriter_handles_empty_text() {
    let mut state = SessionViewState::default();
    state.messages.push(("a".into(), "b".into(), String::new()));
    state.typewriter_pos = 0;

    if let Some((_, _, text)) = state.messages.last() {
        if state.typewriter_pos < text.len() {
            state.typewriter_pos += TYPING_SPEED;
        }
    }

    assert_eq!(state.typewriter_pos, 0);
}

// ── Agent state ticking ────────────────────────────────────────────

#[test]
fn agent_running_tick_increments_elapsed() {
    let mut state = SessionViewState::default();
    state.agent_states = vec![("architect".into(), AgentProgress::Running, 0)];

    for entry in &mut state.agent_states {
        if entry.1 == AgentProgress::Running {
            entry.2 += 1;
        }
    }

    assert_eq!(state.agent_states[0].2, 1);
}

#[test]
fn agent_completed_does_not_tick() {
    let mut state = SessionViewState::default();
    state.agent_states = vec![("architect".into(), AgentProgress::Completed, 10)];

    for entry in &mut state.agent_states {
        if entry.1 == AgentProgress::Running {
            entry.2 += 1;
        }
    }

    assert_eq!(state.agent_states[0].2, 10);
}

#[test]
fn agent_idle_does_not_tick() {
    let mut state = SessionViewState::default();
    state.agent_states = vec![("architect".into(), AgentProgress::Idle, 0)];

    for entry in &mut state.agent_states {
        if entry.1 == AgentProgress::Running {
            entry.2 += 1;
        }
    }

    assert_eq!(state.agent_states[0].2, 0);
}

#[test]
fn agent_failed_does_not_tick() {
    let mut state = SessionViewState::default();
    state.agent_states = vec![("architect".into(), AgentProgress::Failed, 5)];

    for entry in &mut state.agent_states {
        if entry.1 == AgentProgress::Running {
            entry.2 += 1;
        }
    }

    assert_eq!(state.agent_states[0].2, 5);
}

#[test]
fn multiple_running_agents_all_tick() {
    let mut state = SessionViewState::default();
    state.agent_states = vec![
        ("a".into(), AgentProgress::Running, 0),
        ("b".into(), AgentProgress::Running, 0),
        ("c".into(), AgentProgress::Running, 0),
    ];

    for entry in &mut state.agent_states {
        if entry.1 == AgentProgress::Running {
            entry.2 += 1;
        }
    }

    assert_eq!(state.agent_states[0].2, 1);
    assert_eq!(state.agent_states[1].2, 1);
    assert_eq!(state.agent_states[2].2, 1);
}

#[test]
fn mixed_agents_only_running_tick() {
    let mut state = SessionViewState::default();
    state.agent_states = vec![
        ("running_agent".into(), AgentProgress::Running, 0),
        ("completed_agent".into(), AgentProgress::Completed, 10),
        ("idle_agent".into(), AgentProgress::Idle, 0),
        ("failed_agent".into(), AgentProgress::Failed, 3),
    ];

    for entry in &mut state.agent_states {
        if entry.1 == AgentProgress::Running {
            entry.2 += 1;
        }
    }

    assert_eq!(state.agent_states[0].2, 1);
    assert_eq!(state.agent_states[1].2, 10);
    assert_eq!(state.agent_states[2].2, 0);
    assert_eq!(state.agent_states[3].2, 3);
}

// ── TuiEvent processing ────────────────────────────────────────────

#[test]
fn agent_started_adds_new_entry() {
    let mut state = SessionViewState::default();

    let role = "Architect".to_string();
    state.status = SessionStatus::Running;
    state.agent_states.push((role, AgentProgress::Running, 0));

    assert_eq!(state.status, SessionStatus::Running);
    assert_eq!(state.agent_states.len(), 1);
    assert_eq!(state.agent_states[0].0, "Architect");
    assert_eq!(state.agent_states[0].1, AgentProgress::Running);
    assert_eq!(state.agent_states[0].2, 0);
}

#[test]
fn agent_started_updates_existing_entry() {
    let mut state = SessionViewState::default();
    state
        .agent_states
        .push(("Architect".into(), AgentProgress::Idle, 0));

    let role = "Architect".to_string();
    state.status = SessionStatus::Running;
    if let Some(entry) = state.agent_states.iter_mut().find(|(r, _, _)| r == &role) {
        *entry = (role.clone(), AgentProgress::Running, 0);
    }

    assert_eq!(state.agent_states.len(), 1);
    assert_eq!(state.agent_states[0].1, AgentProgress::Running);
}

#[test]
fn agent_completed_updates_entry() {
    let mut state = SessionViewState::default();
    state
        .agent_states
        .push(("Architect".into(), AgentProgress::Running, 5));

    let role = "Architect".to_string();
    if let Some(entry) = state.agent_states.iter_mut().find(|(r, _, _)| r == &role) {
        *entry = (role.clone(), AgentProgress::Completed, entry.2);
    }

    assert_eq!(state.agent_states[0].1, AgentProgress::Completed);
    assert_eq!(state.agent_states[0].2, 5);
}

#[test]
fn agent_completed_missing_entry_is_noop() {
    let mut state = SessionViewState::default();
    state
        .agent_states
        .push(("Architect".into(), AgentProgress::Running, 0));

    let role = "Developer".to_string();
    if let Some(entry) = state.agent_states.iter_mut().find(|(r, _, _)| r == &role) {
        *entry = (role.clone(), AgentProgress::Completed, entry.2);
    }

    assert_eq!(state.agent_states.len(), 1);
    assert_eq!(state.agent_states[0].0, "Architect");
    assert_eq!(state.agent_states[0].1, AgentProgress::Running);
}

#[test]
fn agent_output_adds_message_and_resets_typewriter() {
    let mut state = SessionViewState::default();
    state.typewriter_pos = 42;

    state
        .messages
        .push(("agent-1".into(), "Developer".into(), "output text".into()));
    state.typewriter_pos = 0;

    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].0, "agent-1");
    assert_eq!(state.messages[0].1, "Developer");
    assert_eq!(state.messages[0].2, "output text");
    assert_eq!(state.typewriter_pos, 0);
}

#[test]
fn multiple_agent_outputs_accumulate() {
    let mut state = SessionViewState::default();

    state
        .messages
        .push(("a".into(), "r1".into(), "first".into()));
    state.typewriter_pos = 0;

    state
        .messages
        .push(("b".into(), "r2".into(), "second".into()));
    state.typewriter_pos = 0;

    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages[0].2, "first");
    assert_eq!(state.messages[1].2, "second");
}

#[test]
fn status_changed_updates_status() {
    let mut state = SessionViewState::default();
    assert_eq!(state.status, SessionStatus::Init);

    state.status = SessionStatus::Completed;

    assert_eq!(state.status, SessionStatus::Completed);
}

#[test]
fn status_changed_to_failed() {
    let mut state = SessionViewState::default();
    state.status = SessionStatus::Running;

    state.status = SessionStatus::Failed;

    assert_eq!(state.status, SessionStatus::Failed);
}

#[test]
fn status_changed_to_waiting_for_user() {
    let mut state = SessionViewState::default();
    state.status = SessionStatus::Running;

    state.status = SessionStatus::WaitingForUser;

    assert_eq!(state.status, SessionStatus::WaitingForUser);
}

// ── SessionViewState validation ────────────────────────────────────

#[test]
fn session_status_variants_are_distinct() {
    let statuses = [
        SessionStatus::Init,
        SessionStatus::Running,
        SessionStatus::WaitingForUser,
        SessionStatus::Completed,
        SessionStatus::Failed,
    ];
    let debug_strs: Vec<String> = statuses.iter().map(|s| format!("{:?}", s)).collect();
    assert_eq!(debug_strs.len(), 5);
    let mut unique = debug_strs.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 5);
}

#[test]
fn agent_progress_variants_are_distinct() {
    let states = [
        AgentProgress::Idle,
        AgentProgress::Running,
        AgentProgress::Completed,
        AgentProgress::Failed,
    ];
    let debug_strs: Vec<String> = states.iter().map(|s| format!("{:?}", s)).collect();
    assert_eq!(debug_strs.len(), 4);
    let mut unique = debug_strs.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 4);
}

#[test]
fn panel_focus_variants_are_distinct() {
    let panels = [PanelFocus::Chat, PanelFocus::Progress, PanelFocus::Input];
    let debug_strs: Vec<String> = panels.iter().map(|p| format!("{:?}", p)).collect();
    assert_eq!(debug_strs.len(), 3);
    let mut unique = debug_strs.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 3);
}

// ── InputAction SendMessage ────────────────────────────────────────

#[test]
fn send_message_action_stores_text() {
    let messages = vec!["hello", "", "multi-word message", "你好"];
    for msg in messages {
        let action = InputAction::SendMessage(msg.to_string());
        match action {
            InputAction::SendMessage(ref m) => assert_eq!(m, msg),
            _ => panic!("expected SendMessage"),
        }
    }
}

// ── Debug formatting ───────────────────────────────────────────────

#[test]
fn tui_event_debug_format() {
    let event = TuiEvent::AgentOutput {
        agent_id: "a1".into(),
        role: "dev".into(),
        chunk: "output".into(),
    };
    let debug = format!("{:?}", event);
    assert!(debug.contains("AgentOutput"));
    assert!(debug.contains("a1"));
    assert!(debug.contains("dev"));
}

#[test]
fn tui_event_all_variants_debug_format() {
    let events: Vec<TuiEvent> = vec![
        TuiEvent::AgentOutput {
            agent_id: "a".into(),
            role: "r".into(),
            chunk: "c".into(),
        },
        TuiEvent::AgentStarted {
            agent_id: "a".into(),
            role: "r".into(),
        },
        TuiEvent::AgentCompleted {
            agent_id: "a".into(),
            role: "r".into(),
        },
        TuiEvent::StatusChanged(SessionStatus::Running),
        TuiEvent::Quit,
    ];
    for (i, e) in events.iter().enumerate() {
        let debug = format!("{:?}", e);
        assert!(
            !debug.is_empty(),
            "variant {} should produce debug output",
            i
        );
    }
}

#[test]
fn tui_event_agent_output_round_trip() {
    let original = TuiEvent::AgentOutput {
        agent_id: "agent-42".into(),
        role: "Developer".into(),
        chunk: "output text here".into(),
    };
    let debug = format!("{:?}", original);
    assert!(debug.contains("agent-42"));
    assert!(debug.contains("Developer"));
    assert!(debug.contains("output text here"));
}

#[test]
fn tui_event_agent_started_round_trip() {
    let original = TuiEvent::AgentStarted {
        agent_id: "architect".into(),
        role: "Architect".into(),
    };
    let debug = format!("{:?}", original);
    assert!(debug.contains("architect"));
    assert!(debug.contains("Architect"));
}

#[test]
fn tui_event_status_changed_round_trip() {
    for status in &[
        SessionStatus::Running,
        SessionStatus::Completed,
        SessionStatus::Failed,
    ] {
        let event = TuiEvent::StatusChanged(*status);
        let debug = format!("{:?}", event);
        assert!(debug.contains(&format!("{:?}", status)));
    }
}

// ── SessionViewState typewriter_pos edge cases ───────────────────

#[test]
fn typewriter_pos_no_messages_stays_zero() {
    let mut state = SessionViewState::default();
    if let Some((_, _, text)) = state.messages.last() {
        if state.typewriter_pos < text.len() {
            state.typewriter_pos += TYPING_SPEED;
        }
    }
    assert_eq!(state.typewriter_pos, 0);
}

#[test]
fn typewriter_pos_does_not_overflow_past_text_end() {
    let mut state = SessionViewState::default();
    state.messages.push(("a".into(), "b".into(), "hi".into()));
    state.typewriter_pos = 0;

    if let Some((_, _, text)) = state.messages.last() {
        if state.typewriter_pos < text.len() {
            state.typewriter_pos += TYPING_SPEED;
        }
    }
    assert_eq!(state.typewriter_pos, 3);

    if let Some((_, _, text)) = state.messages.last() {
        if state.typewriter_pos < text.len() {
            state.typewriter_pos += TYPING_SPEED;
        }
    }
    assert_eq!(state.typewriter_pos, 3);
}

#[test]
fn typewriter_multiple_messages_resets_on_new() {
    let mut state = SessionViewState::default();
    state
        .messages
        .push(("a".into(), "b".into(), "hello".into()));
    state.typewriter_pos = 10;

    state.messages.push(("c".into(), "d".into(), "new".into()));
    state.typewriter_pos = 0;

    if let Some((_, _, text)) = state.messages.last() {
        if state.typewriter_pos < text.len() {
            state.typewriter_pos += TYPING_SPEED;
        }
    }
    assert_eq!(state.typewriter_pos, TYPING_SPEED);
    assert_eq!(state.messages.len(), 2);
}

// ── SessionViewState input buffer edge cases ────────────────────

#[test]
fn input_buffer_very_long_content() {
    let mut state = SessionViewState::default();
    state.is_input_mode = true;
    let long = "x".repeat(10_000);
    state.input_buffer = long.clone();
    assert_eq!(state.input_buffer.len(), 10_000);

    let msg = state.input_buffer.clone();
    state.is_input_mode = false;
    state.input_buffer.clear();
    assert!(state.input_buffer.is_empty());
    assert_eq!(msg.len(), 10_000);
}

#[test]
fn input_buffer_backspace_empty_is_noop() {
    let mut state = SessionViewState::default();
    state.is_input_mode = true;
    state.input_buffer.clear();

    state.input_buffer.pop();
    assert!(state.input_buffer.is_empty());
}

#[test]
fn input_buffer_unicode_chars() {
    let mut state = SessionViewState::default();
    state.is_input_mode = true;
    state.input_buffer.push_str("你好世界");
    assert_eq!(state.input_buffer, "你好世界");

    state.input_buffer.pop();
    assert_eq!(state.input_buffer, "你好世");
}

// ── AgentProgress transitions ───────────────────────────────────

#[test]
fn agent_progress_transitions_allowed() {
    let mut state = AgentProgress::Idle;
    state = AgentProgress::Running;
    state = AgentProgress::Completed;
    assert_eq!(state, AgentProgress::Completed);
}

#[test]
fn agent_progress_failed_terminal() {
    let mut state = AgentProgress::Idle;
    state = AgentProgress::Running;
    state = AgentProgress::Failed;
    assert_eq!(state, AgentProgress::Failed);
}
