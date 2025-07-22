//! Simplified Interrupt Management - Claude Code Style
//!
//! This module provides a lightweight interrupt system that mirrors Claude Code's
//! approach: simple, effective, without complex global state management.

use tokio_util::sync::CancellationToken;

/// Simple interrupt manager for reactive agents
#[derive(Debug, Clone)]
pub struct SimpleInterruptManager {
    /// Current task cancellation token
    cancellation_token: CancellationToken,
}

impl SimpleInterruptManager {
    /// Create a new simple interrupt manager
    pub fn new() -> Self {
        Self {
            cancellation_token: CancellationToken::new(),
        }
    }
    
    /// Get the current cancellation token
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }
    
    /// Cancel the current operation
    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }
    
    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }
    
    /// Reset for a new operation
    pub fn reset(&mut self) {
        self.cancellation_token = CancellationToken::new();
    }
    
    /// Create a child token for scoped operations
    pub fn child_token(&self) -> CancellationToken {
        self.cancellation_token.child_token()
    }
}

impl Default for SimpleInterruptManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-local interrupt manager for Claude Code style operations
thread_local! {
    static LOCAL_INTERRUPT_MANAGER: std::cell::RefCell<SimpleInterruptManager> = 
        std::cell::RefCell::new(SimpleInterruptManager::new());
}

/// Get the thread-local interrupt manager
pub fn local_interrupt_manager() -> SimpleInterruptManager {
    LOCAL_INTERRUPT_MANAGER.with(|manager| {
        manager.borrow().clone()
    })
}

/// Cancel the current thread-local operation
pub fn cancel_current_operation() {
    LOCAL_INTERRUPT_MANAGER.with(|manager| {
        manager.borrow().cancel();
    });
}

/// Check if the current operation is cancelled
pub fn is_current_operation_cancelled() -> bool {
    LOCAL_INTERRUPT_MANAGER.with(|manager| {
        manager.borrow().is_cancelled()
    })
}

/// Reset the thread-local interrupt manager
pub fn reset_local_interrupt_manager() {
    LOCAL_INTERRUPT_MANAGER.with(|manager| {
        manager.borrow_mut().reset();
    });
}

/// Get a cancellation token for the current operation
pub fn current_operation_token() -> CancellationToken {
    LOCAL_INTERRUPT_MANAGER.with(|manager| {
        manager.borrow().cancellation_token()
    })
}