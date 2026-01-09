//! Tests for EventManager

use super::*;

#[tokio::test]
async fn test_event_manager_creation() {
    let manager = EventManager::new();
    assert_eq!(manager.current_step, 0);
    assert!(!manager.is_animating());
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

    // Small delay to let animation start
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Stop thinking
    manager.emit(ExecutionEvent::ThinkingStopped).await;

    // Animation should be stopped
    let state = manager.animation_state().await;
    assert_eq!(state, AnimationState::Idle);
}
