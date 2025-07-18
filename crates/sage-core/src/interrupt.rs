use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

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
    pub fn try_recv_interrupt(&mut self) -> Result<InterruptReason, broadcast::error::TryRecvError> {
        self.interrupt_receiver.try_recv()
    }

    /// Wait for an interrupt reason
    pub async fn recv_interrupt(&mut self) -> Result<InterruptReason, broadcast::error::RecvError> {
        self.interrupt_receiver.recv().await
    }
}

/// Global interrupt manager instance
static GLOBAL_INTERRUPT_MANAGER: std::sync::OnceLock<std::sync::Mutex<InterruptManager>> = std::sync::OnceLock::new();

/// Get the global interrupt manager
pub fn global_interrupt_manager() -> &'static std::sync::Mutex<InterruptManager> {
    GLOBAL_INTERRUPT_MANAGER.get_or_init(|| {
        std::sync::Mutex::new(InterruptManager::new())
    })
}

/// Convenience function to interrupt the current task
pub fn interrupt_current_task(reason: InterruptReason) {
    if let Ok(manager) = global_interrupt_manager().lock() {
        manager.interrupt(reason);
    }
}

/// Convenience function to check if the current task is interrupted
pub fn is_current_task_interrupted() -> bool {
    global_interrupt_manager()
        .lock()
        .map(|manager| manager.is_interrupted())
        .unwrap_or(false)
}

/// Convenience function to get a cancellation token for the current task
pub fn current_task_cancellation_token() -> Option<CancellationToken> {
    global_interrupt_manager()
        .lock()
        .ok()
        .map(|manager| manager.cancellation_token())
}

/// Convenience function to create a new task scope
pub fn create_task_scope() -> Option<TaskScope> {
    global_interrupt_manager()
        .lock()
        .ok()
        .map(|manager| manager.create_task_scope())
}

/// Convenience function to reset the global interrupt manager
pub fn reset_global_interrupt_manager() {
    if let Ok(mut manager) = global_interrupt_manager().lock() {
        manager.reset();
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod basic_tests {
    use super::*;
    use tokio::time::{sleep, Duration};

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
