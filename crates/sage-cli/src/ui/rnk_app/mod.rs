//! rnk App Mode - Claude Code-style UI with terminal native scrolling
//!
//! This module implements a UI similar to Claude Code using rnk's inline mode:
//! - Messages are printed using rnk::println() (persists in terminal scrollback)
//! - Fixed bottom UI with separator, input, and status bar
//! - Terminal native scrolling for message history
//!
//! Key architecture:
//! - render(app).run() for inline mode with fixed bottom UI
//! - rnk::println() for messages that persist in scrollback
//! - Background thread polls for new messages and prints them

mod components;
mod executor;
mod formatting;
mod state;

pub use state::{SharedState, UiCommand, UiState};

use super::adapters::RnkEventSink;
use components::{count_matching_commands, get_selected_command, render_command_suggestions, render_header, render_input, render_status_bar, render_thinking_indicator};
use crossterm::terminal;
use executor::{background_loop, executor_loop};
use parking_lot::RwLock;
use rnk::prelude::*;
use sage_core::input::InputChannel;
#[allow(deprecated)]
use sage_core::ui::bridge::{set_global_adapter, set_refresh_callback, EventAdapter};
use sage_core::ui::traits::UiContext;
use std::io;
use std::sync::Arc;
use tokio::sync::mpsc;

// Alias rnk's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as RnkBox;

/// Global state for the app component
static GLOBAL_STATE: std::sync::OnceLock<SharedState> = std::sync::OnceLock::new();
static GLOBAL_CMD_TX: std::sync::OnceLock<mpsc::Sender<UiCommand>> = std::sync::OnceLock::new();
static GLOBAL_ADAPTER: std::sync::OnceLock<EventAdapter> = std::sync::OnceLock::new();
/// Track if header has been printed (for startup)
static HEADER_PRINTED: std::sync::OnceLock<()> = std::sync::OnceLock::new();

/// The main app component - renders fixed bottom UI (separator + input/spinner + status bar)
fn app() -> Element {
    let app_ctx = use_app();

    // Get shared state (return error UI if not initialized)
    let state = match GLOBAL_STATE.get() {
        Some(s) => s,
        None => {
            tracing::error!("Global state not initialized");
            return Text::new("Error: State not initialized")
                .color(Color::Red)
                .into_element();
        }
    };
    let cmd_tx = match GLOBAL_CMD_TX.get() {
        Some(tx) => tx,
        None => {
            tracing::error!("Command channel not initialized");
            return Text::new("Error: Command channel not initialized")
                .color(Color::Red)
                .into_element();
        }
    };

    // Print header on first render (rnk is now initialized)
    if HEADER_PRINTED.set(()).is_ok() {
        let ui_state = state.read();
        rnk::println(render_header(&ui_state.session));
        rnk::println(""); // Empty line after header
    }

    // Get terminal size
    let term_width = terminal::size().map(|(w, _)| w).unwrap_or(80);

    // Check if should quit
    {
        let ui_state = state.read();
        if ui_state.should_quit {
            drop(ui_state);
            app_ctx.exit();
            return Text::new("Goodbye!").into_element();
        }
    }

    // Handle keyboard input
    use_input({
        let state = Arc::clone(state);
        let cmd_tx = cmd_tx.clone();
        let app_ctx = app_ctx.clone();

        move |ch, key| {
            // Ctrl+C to quit
            if key.ctrl && ch == "c" {
                let _ = cmd_tx.try_send(UiCommand::Quit);
                app_ctx.exit();
                return;
            }

            // ESC to cancel or clear suggestions
            if key.escape {
                let mut s = state.write();
                if s.is_busy {
                    drop(s);
                    let _ = cmd_tx.try_send(UiCommand::Cancel);
                } else {
                    // Reset suggestion index
                    s.suggestion_index = 0;
                }
                return;
            }

            // Shift+Tab to cycle permission mode
            if key.shift && ch == "\t" {
                let mut s = state.write();
                s.permission_mode = s.permission_mode.cycle();
                return;
            }

            // Don't accept input while busy
            {
                let s = state.read();
                if s.is_busy {
                    return;
                }
            }

            // Up arrow - move selection up
            if key.up_arrow {
                let mut s = state.write();
                if s.input_text.starts_with('/') && s.suggestion_index > 0 {
                    s.suggestion_index -= 1;
                }
                return;
            }

            // Down arrow - move selection down (with clamping)
            if key.down_arrow {
                let mut s = state.write();
                if s.input_text.starts_with('/') {
                    let max_count = count_matching_commands(&s.input_text);
                    if s.suggestion_index < max_count.saturating_sub(1) {
                        s.suggestion_index += 1;
                    }
                }
                return;
            }

            // Tab - auto-complete selected command
            if key.tab && !key.shift {
                let mut s = state.write();
                if let Some(cmd) = get_selected_command(&s.input_text, s.suggestion_index) {
                    s.input_text = cmd;
                    s.suggestion_index = 0;
                }
                return;
            }

            // Backspace
            if key.backspace {
                let mut s = state.write();
                s.input_text.pop();
                // Reset suggestion index when input changes
                s.suggestion_index = 0;
                return;
            }

            // Enter to submit
            if key.return_key {
                let text = {
                    let mut s = state.write();
                    // If showing suggestions and a command is selected, use that
                    let text = if s.input_text.starts_with('/') {
                        if let Some(cmd) = get_selected_command(&s.input_text, s.suggestion_index) {
                            cmd
                        } else {
                            s.input_text.clone()
                        }
                    } else {
                        s.input_text.clone()
                    };
                    s.input_text.clear();
                    s.suggestion_index = 0;
                    text
                };
                if !text.is_empty() {
                    let _ = cmd_tx.try_send(UiCommand::Submit(text));
                }
                return;
            }

            // Regular character input
            if !ch.is_empty() && !key.ctrl && !key.alt {
                let mut s = state.write();
                s.input_text.push_str(ch);
                // Reset suggestion index when input changes
                s.suggestion_index = 0;
            }
        }
    });

    // Read current state
    let ui_state = state.read();
    let is_busy = ui_state.is_busy;
    let input_text = ui_state.input_text.clone();
    let status_text = ui_state.status_text.clone();
    let permission_mode = ui_state.permission_mode;
    let suggestion_index = ui_state.suggestion_index;
    drop(ui_state);

    // Build UI components
    let mut layout = RnkBox::new().flex_direction(FlexDirection::Column);

    // Thinking indicator above separator (in message area)
    if is_busy {
        layout = layout.child(render_thinking_indicator(&status_text));
        layout = layout.child(Text::new("").into_element()); // Empty line
    }

    // Separator line
    let separator = Text::new("─".repeat(term_width as usize))
        .color(Color::BrightBlack)
        .dim()
        .into_element();
    layout = layout.child(separator);

    // Show command suggestions when typing /
    // Note: suggestion_index is clamped in input handler, not here (pure render)
    let suggestions = if !is_busy {
        render_command_suggestions(&input_text, suggestion_index).map(|(element, _)| element)
    } else {
        None
    };

    if let Some(sugg) = suggestions {
        layout = layout.child(sugg);
    }

    // Input always below separator
    let input = render_input(&input_text);
    let status_bar = render_status_bar(permission_mode);

    layout
        .child(input)
        .child(status_bar)
        .into_element()
}

/// Run the rnk-based app (async version)
pub async fn run_rnk_app() -> io::Result<()> {
    // Create the RnkEventSink adapter (implements EventSink trait)
    let (rnk_sink, adapter) = RnkEventSink::with_default_adapter();

    // Set up the global adapter for backward compatibility
    // This will be deprecated in favor of UiContext
    #[allow(deprecated)]
    set_global_adapter((*adapter).clone());
    let _ = GLOBAL_ADAPTER.set((*adapter).clone());

    // Set up the refresh callback (replaces direct rnk::request_render() in sage-core)
    set_refresh_callback(|| {
        rnk::request_render();
    });

    // Create UiContext for dependency injection (new approach)
    let _ui_context = UiContext::new(Arc::new(rnk_sink));

    // Create shared state
    let state: SharedState = Arc::new(RwLock::new(UiState::default()));
    let _ = GLOBAL_STATE.set(Arc::clone(&state));

    // Create command channel
    let (cmd_tx, cmd_rx) = mpsc::channel::<UiCommand>(16);
    let _ = GLOBAL_CMD_TX.set(cmd_tx);

    // Create input channel for executor
    let (input_channel, _input_handle) = InputChannel::new(16);

    // Spawn executor task
    let executor_state = Arc::clone(&state);
    tokio::spawn(async move {
        executor_loop(executor_state, cmd_rx, input_channel).await;
    });

    // Background thread for printing messages and updating spinner
    let bg_state = Arc::clone(&state);
    let bg_adapter = (*adapter).clone();
    std::thread::spawn(move || {
        background_loop(bg_state, bg_adapter);
    });

    // Run rnk app in inline mode (preserves terminal history)
    render(app).run()
}

#[cfg(test)]
mod tests {
    use super::formatting::{wrap_single_line, wrap_text_with_prefix};

    #[test]
    fn wrap_text_basic() {
        let lines = wrap_single_line("hello world", 20);
        assert_eq!(lines, vec!["hello world"]);
    }

    #[test]
    fn wrap_text_long() {
        let lines = wrap_single_line("hello world this is a long line", 10);
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(unicode_width::UnicodeWidthStr::width(line.as_str()) <= 10);
        }
    }

    #[test]
    fn wrap_with_prefix() {
        let lines = wrap_text_with_prefix("user: ", "hello world", 20);
        assert!(!lines.is_empty());
        assert!(lines[0].starts_with("user: "));
    }
}
