use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

pub mod simple;

/// Global interrupt manager for handling task cancellation
#[derive(Debug, Clone)]
pub struct InterruptManager {
    /// Cancellation token for the current task
    cancellation_token: CancellationToken,
    /// Broadcast sender for interrupt events
    interrupt_sender: broadcast::Sender<InterruptReason>,
    /// Flag to track if interruption is enabled
    interruption_enabled: Arc<AtomicBool>,
}

/// Reason for task interruption
#[derive(Debug, Clone, PartialEq)]
pub enum InterruptReason {
    /// User pressed Ctrl+C
    UserInterrupt,
    /// Task timeout
    Timeout,
    /// System shutdown
    Shutdown,
    /// Manual cancellation
    Manual,
    /// API Error
    ApiError,
}

impl Default for InterruptManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InterruptManager {
    /// Create a new interrupt manager
    pub fn new() -> Self {
        let (interrupt_sender, _) = broadcast::channel(16);

        Self {
            cancellation_token: CancellationToken::new(),
            interrupt_sender,
            interruption_enabled: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Get the cancellation token for the current task
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    /// Create a new task scope with its own cancellation token
    pub fn create_task_scope(&self) -> TaskScope {
        let child_token = self.cancellation_token.child_token();
        TaskScope {
            token: child_token,
            interrupt_receiver: self.interrupt_sender.subscribe(),
        }
    }

    /// Interrupt the current task with the given reason
    pub fn interrupt(&self, reason: InterruptReason) {
        if self.interruption_enabled.load(Ordering::Relaxed) {
            // Cancel the current task
            self.cancellation_token.cancel();

            // Send interrupt notification
            let _ = self.interrupt_sender.send(reason);
        }
    }

    /// Check if the current task should be interrupted
    pub fn is_interrupted(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }

    /// Reset the interrupt manager for a new task
    pub fn reset(&mut self) {
        // Create a new cancellation token
        self.cancellation_token = CancellationToken::new();

        // Keep the same broadcast channel for consistency
        // Subscribers will need to resubscribe for new tasks
    }

    /// Enable or disable interruption
    pub fn set_interruption_enabled(&self, enabled: bool) {
        self.interruption_enabled.store(enabled, Ordering::Relaxed);
    }

    /// Check if interruption is enabled
    pub fn is_interruption_enabled(&self) -> bool {
        self.interruption_enabled.load(Ordering::Relaxed)
    }

    /// Subscribe to interrupt events
    pub fn subscribe(&self) -> broadcast::Receiver<InterruptReason> {
        self.interrupt_sender.subscribe()
    }
}

/// A task scope that can be cancelled
#[derive(Debug)]
pub struct TaskScope {
    token: CancellationToken,
    interrupt_receiver: broadcast::Receiver<InterruptReason>,
}

impl TaskScope {
    /// Get the cancellation token for this task scope
    pub fn token(&self) -> &CancellationToken {
        &self.token
    }

    /// Check if this task scope has been cancelled
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    /// Wait for cancellation or return immediately if already cancelled
    pub async fn cancelled(&self) {
        self.token.cancelled().await
    }

    /// Try to receive an interrupt reason without blocking
    pub fn try_recv_interrupt(
        &mut self,
    ) -> Result<InterruptReason, broadcast::error::TryRecvError> {
        self.interrupt_receiver.try_recv()
    }

    /// Wait for an interrupt reason
    pub async fn recv_interrupt(&mut self) -> Result<InterruptReason, broadcast::error::RecvError> {
        self.interrupt_receiver.recv().await
    }
}

/// Global interrupt manager instance
///
/// Uses `parking_lot::Mutex` instead of `std::sync::Mutex` for:
/// - Better performance (no poisoning overhead)
/// - No panic on lock (no poison state handling needed)
/// - Faster lock acquisition in async contexts
static GLOBAL_INTERRUPT_MANAGER: std::sync::OnceLock<Mutex<InterruptManager>> =
    std::sync::OnceLock::new();

/// Get the global interrupt manager
pub fn global_interrupt_manager() -> &'static Mutex<InterruptManager> {
    GLOBAL_INTERRUPT_MANAGER.get_or_init(|| Mutex::new(InterruptManager::new()))
}

/// Convenience function to interrupt the current task
///
/// This function acquires the lock briefly to call interrupt().
/// The lock is released immediately after the call.
pub fn interrupt_current_task(reason: InterruptReason) {
    global_interrupt_manager().lock().interrupt(reason);
}

/// Convenience function to check if the current task is interrupted
///
/// Returns `true` if the current task has been interrupted, `false` otherwise.
pub fn is_current_task_interrupted() -> bool {
    global_interrupt_manager().lock().is_interrupted()
}

/// Convenience function to get a cancellation token for the current task
///
/// Returns the cancellation token that can be used to monitor for task cancellation.
pub fn current_task_cancellation_token() -> CancellationToken {
    global_interrupt_manager().lock().cancellation_token()
}

/// Convenience function to create a new task scope
///
/// Creates a child task scope that inherits cancellation from the parent.
pub fn create_task_scope() -> TaskScope {
    global_interrupt_manager().lock().create_task_scope()
}

/// Convenience function to reset the global interrupt manager
///
/// Resets the interrupt state for a new task. This should be called
/// before starting a new top-level task.
pub fn reset_global_interrupt_manager() {
    global_interrupt_manager().lock().reset();
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod basic_tests {
    use super::*;
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn test_interrupt_manager_basic() {
        let manager = InterruptManager::new();

        assert!(!manager.is_interrupted());

        manager.interrupt(InterruptReason::UserInterrupt);

        assert!(manager.is_interrupted());
    }

    #[tokio::test]
    async fn test_task_scope() {
        let manager = InterruptManager::new();
        let scope = manager.create_task_scope();

        assert!(!scope.is_cancelled());

        manager.interrupt(InterruptReason::UserInterrupt);

        // Give a moment for the cancellation to propagate
        sleep(Duration::from_millis(10)).await;

        assert!(scope.is_cancelled());
    }

    #[tokio::test]
    async fn test_global_interrupt_manager() {
        reset_global_interrupt_manager();

        assert!(!is_current_task_interrupted());

        interrupt_current_task(InterruptReason::UserInterrupt);

        assert!(is_current_task_interrupted());
    }
}
