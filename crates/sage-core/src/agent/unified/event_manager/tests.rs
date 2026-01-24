//! Tests for EventManager

use super::*;
use crate::ui::traits::testing::MockEventSink;
use std::sync::Arc;

#[tokio::test]
async fn test_event_manager_creation() {
    let manager = EventManager::new();
    assert_eq!(manager.current_step, 0);
    assert!(!manager.is_animating);
}

#[tokio::test]
async fn test_emit_step_started() {
    let mut manager = EventManager::new();
    manager
        .emit(ExecutionEvent::StepStarted { step_number: 5 })
        .await;
    assert_eq!(manager.current_step, 5);
}

#[tokio::test]
async fn test_emit_thinking_started_and_stopped() {
    let mut manager = EventManager::new();

    // Start thinking
    manager
        .emit(ExecutionEvent::ThinkingStarted { step_number: 1 })
        .await;

    assert!(manager.is_animating);

    // Stop thinking
    manager.emit(ExecutionEvent::ThinkingStopped).await;

    // Animation should be stopped
    assert!(!manager.is_animating);
}

#[tokio::test]
async fn test_event_manager_with_ui_context() {
    let sink = Arc::new(MockEventSink::new());
    let ctx = UiContext::new(sink.clone());
    let mut manager = EventManager::with_ui_context(ctx);

    // Emit thinking started
    manager
        .emit(ExecutionEvent::ThinkingStarted { step_number: 1 })
        .await;

    // Verify event was captured
    let events = sink.events();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], AgentEvent::ThinkingStarted));
    assert_eq!(sink.refresh_count(), 1);
}

#[tokio::test]
async fn test_event_manager_session_events() {
    let sink = Arc::new(MockEventSink::new());
    let ctx = UiContext::new(sink.clone());
    let mut manager = EventManager::with_ui_context(ctx);

    // Emit session started
    manager
        .emit(ExecutionEvent::SessionStarted {
            session_id: "test-session".to_string(),
            model: "test-model".to_string(),
            provider: "test-provider".to_string(),
        })
        .await;

    // Emit session ended
    manager
        .emit(ExecutionEvent::SessionEnded {
            session_id: "test-session".to_string(),
        })
        .await;

    let events = sink.events();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], AgentEvent::SessionStarted { .. }));
    assert!(matches!(events[1], AgentEvent::SessionEnded { .. }));
}

#[tokio::test]
async fn test_event_manager_tool_events() {
    let sink = Arc::new(MockEventSink::new());
    let ctx = UiContext::new(sink.clone());
    let mut manager = EventManager::with_ui_context(ctx);

    // Emit tool started
    manager
        .emit(ExecutionEvent::ToolExecutionStarted {
            tool_name: "bash".to_string(),
            tool_id: "tool-123".to_string(),
        })
        .await;

    assert!(manager.is_animating);

    // Emit tool completed
    manager
        .emit(ExecutionEvent::ToolExecutionCompleted {
            tool_name: "bash".to_string(),
            tool_id: "tool-123".to_string(),
            success: true,
            duration_ms: 100,
        })
        .await;

    assert!(!manager.is_animating);

    let events = sink.events();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], AgentEvent::ToolExecutionStarted { .. }));
    assert!(matches!(events[1], AgentEvent::ToolExecutionCompleted { .. }));
}
