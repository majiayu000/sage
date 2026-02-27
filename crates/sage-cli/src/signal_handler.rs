//! Signal Handler for CLI
//!
//! Manages Ctrl+C interrupts and application state transitions.
//! API is tested but not all methods are used in production yet (binary crate).

#![allow(dead_code)]

use futures::stream::StreamExt;
use parking_lot::Mutex;
use sage_core::interrupt::{InterruptReason, global_interrupt_manager};
use signal_hook::consts::SIGINT;
use signal_hook_tokio::Signals;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

/// Application state for signal handling
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SignalHandlerState {
    /// Waiting for user input at prompt
    WaitingForInput,
    /// Executing a task
    ExecutingTask,
}

/// Signal handler for managing Ctrl+C interrupts
pub struct SignalHandler {
    /// Flag to track if signal handling is active
    is_active: Arc<AtomicBool>,
    /// Handle to the signal handling task
    task_handle: Option<JoinHandle<()>>,
    /// Current application state (using parking_lot::Mutex for async safety)
    app_state: Arc<Mutex<SignalHandlerState>>,
    /// Counter for consecutive Ctrl+C presses
    ctrl_c_count: Arc<AtomicU32>,
    /// Timestamp of last Ctrl+C press (using parking_lot::Mutex for async safety)
    last_ctrl_c_time: Arc<Mutex<Option<Instant>>>,
}

impl SignalHandler {
    /// Create a new signal handler
    pub fn new() -> Self {
        Self {
            is_active: Arc::new(AtomicBool::new(false)),
            task_handle: None,
            app_state: Arc::new(Mutex::new(SignalHandlerState::WaitingForInput)),
            ctrl_c_count: Arc::new(AtomicU32::new(0)),
            last_ctrl_c_time: Arc::new(Mutex::new(None)),
        }
    }

    fn update_ctrl_c_exit_state(
        last_ctrl_c_time: &Arc<Mutex<Option<Instant>>>,
        ctrl_c_count: &Arc<AtomicU32>,
    ) -> bool {
        let now = Instant::now();
        let mut should_exit = false;

        let mut last_time = last_ctrl_c_time.lock();
        if let Some(last) = *last_time {
            if now.duration_since(last) < Duration::from_secs(2) {
                should_exit = true;
            } else {
                ctrl_c_count.store(1, Ordering::Relaxed);
            }
        } else {
            ctrl_c_count.store(1, Ordering::Relaxed);
        }
        *last_time = Some(now);

        should_exit
    }

    fn interrupt_current_task() {
        global_interrupt_manager()
            .lock()
            .interrupt(InterruptReason::UserInterrupt);
    }

    /// Start signal handling
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_active.load(Ordering::Relaxed) {
            return Ok(()); // Already started
        }

        // Create signal stream for SIGINT (Ctrl+C)
        let mut signals = Signals::new([SIGINT])?;
        let is_active = self.is_active.clone();
        let app_state = self.app_state.clone();
        let ctrl_c_count = self.ctrl_c_count.clone();
        let last_ctrl_c_time = self.last_ctrl_c_time.clone();

        // Mark as active
        is_active.store(true, Ordering::Relaxed);

        // Spawn signal handling task
        let handle = tokio::spawn(async move {
            while let Some(signal) = signals.next().await {
                match signal {
                    SIGINT => {
                        if is_active.load(Ordering::Relaxed) {
                            // Check current application state
                            // Using parking_lot::Mutex - lock() returns guard directly
                            let state = *app_state.lock();
                            match state {
                                SignalHandlerState::WaitingForInput => {
                                    // During input prompt - implement double Ctrl+C to exit
                                    let should_exit =
                                        Self::update_ctrl_c_exit_state(&last_ctrl_c_time, &ctrl_c_count);

                                    if should_exit {
                                        eprintln!("\nGoodbye!");
                                        std::process::exit(0);
                                    } else {
                                        eprintln!(
                                            "\n💡 Press Ctrl+C again within 2 seconds to exit, or continue typing..."
                                        );
                                    }
                                }
                                SignalHandlerState::ExecutingTask => {
                                    // During task execution - interrupt the task
                                    // parking_lot::Mutex is used in sage-core, .lock() returns guard directly
                                    Self::interrupt_current_task();

                                    // Print a message to let user know the task was interrupted
                                    eprintln!("\n🛑 Interrupting current task... (Ctrl+C)");
                                    eprintln!("   Task will stop gracefully. Please wait...");
                                }
                            }
                        }
                    }
                    _ => {
                        // Handle other signals if needed
                    }
                }
            }
        });

        self.task_handle = Some(handle);
        Ok(())
    }

    /// Stop signal handling
    pub async fn stop(&mut self) {
        self.is_active.store(false, Ordering::Relaxed);

        if let Some(handle) = self.task_handle.take() {
            handle.abort();
            // Give it a moment to clean up
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    }

    /// Check if signal handling is active
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Relaxed)
    }

    /// Enable signal handling (allows interrupts to be processed)
    pub fn enable(&self) {
        self.is_active.store(true, Ordering::Relaxed);
    }

    /// Disable signal handling (ignores interrupts)
    pub fn disable(&self) {
        self.is_active.store(false, Ordering::Relaxed);
    }

    /// Set the application state for signal handling
    pub fn set_app_state(&self, state: SignalHandlerState) {
        // parking_lot::Mutex - lock() returns guard directly
        *self.app_state.lock() = state;
    }

    /// Get the current application state
    pub fn get_app_state(&self) -> SignalHandlerState {
        // parking_lot::Mutex - lock() returns guard directly
        *self.app_state.lock()
    }
}

impl Default for SignalHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SignalHandler {
    fn drop(&mut self) {
        self.is_active.store(false, Ordering::Relaxed);
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }
}

/// Global signal handler instance (using parking_lot::Mutex for async safety)
static GLOBAL_SIGNAL_HANDLER: std::sync::OnceLock<Mutex<SignalHandler>> =
    std::sync::OnceLock::new();

/// Get the global signal handler
pub fn global_signal_handler() -> &'static Mutex<SignalHandler> {
    GLOBAL_SIGNAL_HANDLER.get_or_init(|| Mutex::new(SignalHandler::new()))
}

/// Start global signal handling
pub async fn start_global_signal_handling() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let mut handler = {
        let guard = global_signal_handler().lock();
        // Clone the handler to avoid holding lock across await
        SignalHandler {
            is_active: guard.is_active.clone(),
            task_handle: None,
            app_state: guard.app_state.clone(),
            ctrl_c_count: guard.ctrl_c_count.clone(),
            last_ctrl_c_time: guard.last_ctrl_c_time.clone(),
        }
    };

    let result = handler.start().await;

    // Update the global handler with the new task handle
    if result.is_ok() {
        global_signal_handler().lock().task_handle = handler.task_handle.take();
    }

    result
}

/// Stop global signal handling
pub async fn stop_global_signal_handling() {
    let mut handler = {
        let mut guard = global_signal_handler().lock();
        // Extract the handler to avoid holding lock across await
        SignalHandler {
            is_active: guard.is_active.clone(),
            task_handle: guard.task_handle.take(),
            app_state: guard.app_state.clone(),
            ctrl_c_count: guard.ctrl_c_count.clone(),
            last_ctrl_c_time: guard.last_ctrl_c_time.clone(),
        }
    };

    handler.stop().await;
}

/// Enable global signal handling
pub fn enable_global_signal_handling() {
    global_signal_handler().lock().enable();
}

/// Disable global signal handling
pub fn disable_global_signal_handling() {
    global_signal_handler().lock().disable();
}

/// Check if global signal handling is active
pub fn is_global_signal_handling_active() -> bool {
    global_signal_handler().lock().is_active()
}

/// Set the global application state for signal handling
pub fn set_global_app_state(state: SignalHandlerState) {
    global_signal_handler().lock().set_app_state(state);
}

/// Get the global application state
pub fn get_global_app_state() -> SignalHandlerState {
    global_signal_handler().lock().get_app_state()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_signal_handler_creation() {
        let handler = SignalHandler::new();
        assert!(!handler.is_active());
    }

    #[tokio::test]
    async fn test_signal_handler_start_stop() {
        let mut handler = SignalHandler::new();

        // Start signal handling
        assert!(handler.start().await.is_ok());
        assert!(handler.is_active());

        // Stop signal handling
        handler.stop().await;
        assert!(!handler.is_active());
    }

    #[tokio::test]
    async fn test_global_signal_handler() {
        // Test global signal handler functions
        assert!(start_global_signal_handling().await.is_ok());
        assert!(is_global_signal_handling_active());

        disable_global_signal_handling();
        assert!(!is_global_signal_handling_active());

        enable_global_signal_handling();
        assert!(is_global_signal_handling_active());

        stop_global_signal_handling().await;
        assert!(!is_global_signal_handling_active());
    }
}
