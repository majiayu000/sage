use super::*;
use tokio::time::{Duration, sleep};

#[tokio::test]
async fn test_interrupt_manager_basic_functionality() {
    let manager = InterruptManager::new();

    // Initially not interrupted
    assert!(!manager.is_interrupted());

    // Interrupt the task
    manager.interrupt(InterruptReason::UserInterrupt);

    // Should now be interrupted
    assert!(manager.is_interrupted());
}

#[tokio::test]
async fn test_task_scope_cancellation() {
    let manager = InterruptManager::new();
    let scope = manager.create_task_scope();

    // Initially not cancelled
    assert!(!scope.is_cancelled());

    // Interrupt the manager
    manager.interrupt(InterruptReason::UserInterrupt);

    // Give a moment for cancellation to propagate
    sleep(Duration::from_millis(10)).await;

    // Scope should now be cancelled
    assert!(scope.is_cancelled());
}

#[tokio::test]
async fn test_global_interrupt_manager() {
    // Reset the global manager
    reset_global_interrupt_manager();

    // Initially not interrupted
    assert!(!is_current_task_interrupted());

    // Interrupt the current task
    interrupt_current_task(InterruptReason::UserInterrupt);

    // Should now be interrupted
    assert!(is_current_task_interrupted());

    // Reset again
    reset_global_interrupt_manager();

    // Should not be interrupted after reset
    assert!(!is_current_task_interrupted());
}

#[tokio::test]
async fn test_interrupt_with_select() {
    let manager = InterruptManager::new();
    let token = manager.cancellation_token();

    // Test that select! works with cancellation
    let result = tokio::select! {
        _ = sleep(Duration::from_secs(1)) => {
            "timeout"
        }
        _ = token.cancelled() => {
            "cancelled"
        }
    };

    // Should timeout since we haven't cancelled
    assert_eq!(result, "timeout");

    // Now test with cancellation
    let manager = InterruptManager::new();
    let token = manager.cancellation_token();

    // Start a task that will be cancelled
    let handle = tokio::spawn(async move {
        tokio::select! {
            _ = sleep(Duration::from_secs(10)) => {
                "timeout"
            }
            _ = token.cancelled() => {
                "cancelled"
            }
        }
    });

    // Give it a moment to start
    sleep(Duration::from_millis(10)).await;

    // Cancel the task
    manager.interrupt(InterruptReason::UserInterrupt);

    // Wait for the task to complete
    let result = handle.await.unwrap();
    assert_eq!(result, "cancelled");
}

#[tokio::test]
async fn test_interrupt_reason_broadcast() {
    let manager = InterruptManager::new();
    let mut receiver = manager.subscribe();

    // Interrupt with a specific reason
    manager.interrupt(InterruptReason::UserInterrupt);

    // Should receive the interrupt reason
    let reason = receiver.recv().await.unwrap();
    assert_eq!(reason, InterruptReason::UserInterrupt);
}

#[tokio::test]
async fn test_interruption_enable_disable() {
    let manager = InterruptManager::new();

    // Initially enabled
    assert!(manager.is_interruption_enabled());

    // Disable interruption
    manager.set_interruption_enabled(false);
    assert!(!manager.is_interruption_enabled());

    // Try to interrupt (should be ignored)
    manager.interrupt(InterruptReason::UserInterrupt);
    assert!(!manager.is_interrupted());

    // Re-enable interruption
    manager.set_interruption_enabled(true);
    assert!(manager.is_interruption_enabled());

    // Now interrupt should work
    manager.interrupt(InterruptReason::UserInterrupt);
    assert!(manager.is_interrupted());
}
