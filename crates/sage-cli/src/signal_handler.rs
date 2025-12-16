use futures::stream::StreamExt;
use sage_core::interrupt::{InterruptReason, global_interrupt_manager};
use signal_hook::consts::SIGINT;
use signal_hook_tokio::Signals;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

/// Application state for signal handling
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppState {
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
    /// Current application state
    app_state: Arc<std::sync::Mutex<AppState>>,
    /// Counter for consecutive Ctrl+C presses
    ctrl_c_count: Arc<AtomicU32>,
    /// Timestamp of last Ctrl+C press
    last_ctrl_c_time: Arc<std::sync::Mutex<Option<Instant>>>,
}

impl SignalHandler {
    /// Create a new signal handler
    pub fn new() -> Self {
        Self {
            is_active: Arc::new(AtomicBool::new(false)),
            task_handle: None,
            app_state: Arc::new(std::sync::Mutex::new(AppState::WaitingForInput)),
            ctrl_c_count: Arc::new(AtomicU32::new(0)),
            last_ctrl_c_time: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// Start signal handling
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_active.load(Ordering::Relaxed) {
            return Ok(()); // Already started
        }

        // Create signal stream for SIGINT (Ctrl+C)
        let mut signals = Signals::new(&[SIGINT])?;
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
                            if let Ok(state) = app_state.lock() {
                                match *state {
                                    AppState::WaitingForInput => {
                                        // During input prompt - implement double Ctrl+C to exit
                                        let now = Instant::now();
                                        let mut should_exit = false;

                                        if let Ok(mut last_time) = last_ctrl_c_time.lock() {
                                            if let Some(last) = *last_time {
                                                // Check if this is within 2 seconds of the last Ctrl+C
                                                if now.duration_since(last) < Duration::from_secs(2)
                                                {
                                                    // Second Ctrl+C within 2 seconds - exit
                                                    should_exit = true;
                                                } else {
                                                    // Reset counter if too much time has passed
                                                    ctrl_c_count.store(1, Ordering::Relaxed);
                                                }
                                            } else {
                                                // First Ctrl+C
                                                ctrl_c_count.store(1, Ordering::Relaxed);
                                            }
                                            *last_time = Some(now);
                                        }

                                        if should_exit {
                                            eprintln!("\nGoodbye!");
                                            std::process::exit(0);
                                        } else {
                                            eprintln!(
                                                "\n💡 Press Ctrl+C again within 2 seconds to exit, or continue typing..."
                                            );
                                        }
                                    }
                                    AppState::ExecutingTask => {
                                        // During task execution - interrupt the task
                                        if let Ok(manager) = global_interrupt_manager().lock() {
                                            manager.interrupt(InterruptReason::UserInterrupt);
                                        }

                                        // Print a message to let user know the task was interrupted
                                        eprintln!("\n🛑 Interrupting current task... (Ctrl+C)");
                                        eprintln!("   Task will stop gracefully. Please wait...");
                                    }
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
    #[allow(dead_code)]
    pub async fn stop(&mut self) {
        self.is_active.store(false, Ordering::Relaxed);

        if let Some(handle) = self.task_handle.take() {
            handle.abort();
            // Give it a moment to clean up
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    }

    /// Check if signal handling is active
    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Relaxed)
    }

    /// Enable signal handling (allows interrupts to be processed)
    #[allow(dead_code)]
    pub fn enable(&self) {
        self.is_active.store(true, Ordering::Relaxed);
    }

    /// Disable signal handling (ignores interrupts)
    #[allow(dead_code)]
    pub fn disable(&self) {
        self.is_active.store(false, Ordering::Relaxed);
    }

    /// Set the application state for signal handling
    pub fn set_app_state(&self, state: AppState) {
        if let Ok(mut current_state) = self.app_state.lock() {
            *current_state = state;
        }
    }

    /// Get the current application state
    #[allow(dead_code)]
    pub fn get_app_state(&self) -> AppState {
        self.app_state
            .lock()
            .map(|state| *state)
            .unwrap_or(AppState::WaitingForInput)
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

/// Global signal handler instance
static GLOBAL_SIGNAL_HANDLER: std::sync::OnceLock<std::sync::Mutex<SignalHandler>> =
    std::sync::OnceLock::new();

/// Get the global signal handler
pub fn global_signal_handler() -> &'static std::sync::Mutex<SignalHandler> {
    GLOBAL_SIGNAL_HANDLER.get_or_init(|| std::sync::Mutex::new(SignalHandler::new()))
}

/// Start global signal handling
pub async fn start_global_signal_handling() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    if let Ok(mut handler) = global_signal_handler().lock() {
        handler.start().await
    } else {
        Err("Failed to acquire signal handler lock".into())
    }
}

/// Stop global signal handling
#[allow(dead_code)]
pub async fn stop_global_signal_handling() {
    if let Ok(mut handler) = global_signal_handler().lock() {
        handler.stop().await;
    }
}

/// Enable global signal handling
#[allow(dead_code)]
pub fn enable_global_signal_handling() {
    if let Ok(handler) = global_signal_handler().lock() {
        handler.enable();
    }
}

/// Disable global signal handling
#[allow(dead_code)]
pub fn disable_global_signal_handling() {
    if let Ok(handler) = global_signal_handler().lock() {
        handler.disable();
    }
}

/// Check if global signal handling is active
#[allow(dead_code)]
pub fn is_global_signal_handling_active() -> bool {
    global_signal_handler()
        .lock()
        .map(|handler| handler.is_active())
        .unwrap_or(false)
}

/// Set the global application state for signal handling
pub fn set_global_app_state(state: AppState) {
    if let Ok(handler) = global_signal_handler().lock() {
        handler.set_app_state(state);
    }
}

/// Get the global application state
#[allow(dead_code)]
pub fn get_global_app_state() -> AppState {
    global_signal_handler()
        .lock()
        .map(|handler| handler.get_app_state())
        .unwrap_or(AppState::WaitingForInput)
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
