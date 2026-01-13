//! Thinking Indicator - Spinner with elapsed time
//!
//! Displays a spinner animation with "Thinking..." message and elapsed time.
//! Supports ESC key cancellation.

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal,
};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tokio::sync::watch;

/// Spinner animation for thinking/loading states
pub struct ThinkingIndicator {
    running: Arc<AtomicBool>,
    cancel_tx: watch::Sender<bool>,
    handle: Option<JoinHandle<()>>,
}

impl ThinkingIndicator {
    /// Start a new thinking indicator with the given message
    pub fn start(message: &str) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let (cancel_tx, _cancel_rx) = watch::channel(false);
        let cancel_tx_clone = cancel_tx.clone();
        let message = message.to_string();

        let handle = std::thread::spawn(move || {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let start = Instant::now();
            let mut i = 0;

            // Enable raw mode for key detection
            let _ = terminal::enable_raw_mode();

            while running_clone.load(Ordering::Relaxed) {
                // Check for ESC key (non-blocking)
                if event::poll(Duration::from_millis(80)).unwrap_or(false) {
                    if let Ok(Event::Key(KeyEvent {
                        code: KeyCode::Esc, ..
                    })) = event::read()
                    {
                        let _ = cancel_tx_clone.send(true);
                        running_clone.store(false, Ordering::Relaxed);
                        break;
                    }
                }

                let elapsed = start.elapsed().as_secs_f32();
                // Magenta spinner with elapsed time
                print!(
                    "\x1b[2K\r\x1b[35m{} {} ({:.1}s)\x1b[0m \x1b[2m(ESC to cancel)\x1b[0m",
                    frames[i], message, elapsed
                );
                io::stdout().flush().unwrap();
                i = (i + 1) % frames.len();
            }

            let _ = terminal::disable_raw_mode();
            // Clear the spinner line
            print!("\x1b[2K\r");
            io::stdout().flush().unwrap();
        });

        Self {
            running,
            cancel_tx,
            handle: Some(handle),
        }
    }

    /// Get a receiver to check if cancellation was requested
    pub fn cancel_receiver(&self) -> watch::Receiver<bool> {
        self.cancel_tx.subscribe()
    }

    /// Stop the indicator and return whether it was cancelled
    pub fn stop(mut self) -> bool {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        // Check if cancelled
        *self.cancel_tx.subscribe().borrow()
    }
}

impl Drop for ThinkingIndicator {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

/// Tool execution indicator - similar to thinking but with different styling
pub struct ToolIndicator {
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl ToolIndicator {
    /// Start a new tool execution indicator
    pub fn start(tool_name: &str) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let tool_name = tool_name.to_string();

        let handle = std::thread::spawn(move || {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let start = Instant::now();
            let mut i = 0;

            while running_clone.load(Ordering::Relaxed) {
                let elapsed = start.elapsed().as_secs_f32();
                // Cyan for tool execution
                print!(
                    "\x1b[2K\r\x1b[36m{} {} ({:.1}s)\x1b[0m",
                    frames[i], tool_name, elapsed
                );
                io::stdout().flush().unwrap();
                i = (i + 1) % frames.len();
                std::thread::sleep(Duration::from_millis(80));
            }

            // Clear the line
            print!("\x1b[2K\r");
            io::stdout().flush().unwrap();
        });

        Self {
            running,
            handle: Some(handle),
        }
    }

    /// Stop the indicator
    pub fn stop(mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for ToolIndicator {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}
