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
mod theme;

pub use state::{SharedState, UiCommand, UiState};

use super::adapters::RnkEventSink;
use crate::args::Cli;
use components::{
    count_matching_commands, get_selected_command, render_command_suggestions, render_input,
    render_model_selector, render_separator, render_status_bar, render_thinking_indicator,
};
use crossterm::terminal;
use executor::{background_loop, executor_loop};
use parking_lot::RwLock;
use rnk::prelude::*;
use sage_core::input::InputChannel;
#[allow(deprecated)]
use sage_core::ui::bridge::{set_global_adapter, set_refresh_callback};
use sage_core::ui::traits::UiContext;
use std::io;
use std::sync::Arc;
use theme::current_theme;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

// Alias rnk's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as RnkBox;

/// Global state for the app component
static GLOBAL_STATE: std::sync::OnceLock<SharedState> = std::sync::OnceLock::new();
static GLOBAL_CMD_TX: std::sync::OnceLock<mpsc::Sender<UiCommand>> = std::sync::OnceLock::new();

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

    // Header is now printed in background_loop when session info is available

    // Get terminal width each render so resize is handled naturally
    let (term_width, _) = terminal::size().unwrap_or((80, 24));

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

            // Check if in model select mode
            let in_model_mode = state.read().model_select_mode;

            // ESC to cancel or exit model select mode
            if key.escape {
                let mut s = state.write();
                if s.model_select_mode {
                    // Exit model selection mode
                    s.model_select_mode = false;
                    s.model_select_index = 0;
                    s.available_models.clear();
                } else if s.is_busy {
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

            // Model selection mode - handle separately
            if in_model_mode {
                if key.up_arrow {
                    let mut s = state.write();
                    if s.model_select_index > 0 {
                        s.model_select_index -= 1;
                    }
                    return;
                }

                if key.down_arrow {
                    let mut s = state.write();
                    let max = s.available_models.len().saturating_sub(1);
                    if s.model_select_index < max {
                        s.model_select_index += 1;
                    }
                    return;
                }

                if key.return_key || (key.tab && !key.shift) {
                    // Select the model and switch
                    let model = {
                        let mut s = state.write();
                        let model = s.available_models.get(s.model_select_index).cloned();
                        s.model_select_mode = false;
                        s.model_select_index = 0;
                        s.available_models.clear();
                        model
                    };
                    if let Some(m) = model {
                        let _ = cmd_tx.try_send(UiCommand::Submit(format!("/model {}", m)));
                    }
                    return;
                }

                // Any other key exits model select mode
                return;
            }

            // Normal mode - Up arrow - move selection up
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
    let animation_frame = ui_state.animation_frame;
    let model_name = ui_state.session.model.clone();
    let model_select_mode = ui_state.model_select_mode;
    let available_models = ui_state.available_models.clone();
    let model_select_index = ui_state.model_select_index;
    drop(ui_state);

    let theme = current_theme();
    let term_width_usize = term_width as usize;

    // Bottom section contains all fixed UI elements (no header - it's printed via rnk::println)
    let mut bottom = RnkBox::new().flex_direction(FlexDirection::Column);

    // Thinking indicator above separator (in message area)
    if is_busy {
        bottom = bottom.child(render_thinking_indicator(
            &status_text,
            animation_frame,
            theme,
        ));
        bottom = bottom.child(Text::new("").into_element()); // Empty line
    }

    // Top separator line
    bottom = bottom.child(render_separator(term_width_usize, theme));

    // Input line
    let input = render_input(&input_text, theme, animation_frame);
    bottom = bottom.child(input);

    // Bottom separator line
    bottom = bottom.child(render_separator(term_width_usize, theme));

    // Show model selector or command suggestions below input
    if model_select_mode && !available_models.is_empty() {
        // Model selection mode
        bottom = bottom.child(render_model_selector(
            &available_models,
            model_select_index,
            theme,
        ));
    } else if !is_busy {
        // Show command suggestions when typing /
        if let Some((element, _)) = render_command_suggestions(&input_text, suggestion_index, theme)
        {
            bottom = bottom.child(element);
        }
    }

    // Status bar
    let status_bar = render_status_bar(permission_mode, Some(&model_name), theme);
    bottom = bottom.child(status_bar);

    bottom.into_element()
}

/// Run the rnk-based app (async version)
pub async fn run_rnk_app(cli: &Cli) -> io::Result<()> {
    // Load config to get model/provider info for header
    let (model, provider) = match if std::path::Path::new(&cli.config_file).exists() {
        sage_core::config::load_config_from_file(&cli.config_file)
    } else {
        sage_core::config::load_config()
    } {
        Ok(config) => {
            let provider = config.default_provider.clone();
            let keys: Vec<_> = config.model_providers.keys().cloned().collect();
            let model = config
                .model_providers
                .get(&provider)
                .map(|p| p.model.clone())
                .unwrap_or_else(|| format!("no-provider-{}-keys:{:?}", provider, keys));
            (model, provider)
        }
        Err(e) => (format!("err:{}", e), "config-error".to_string()),
    };

    // Header will be printed via rnk::println() in background_loop

    // Create the RnkEventSink adapter (implements EventSink trait)
    let (rnk_sink, adapter) = RnkEventSink::with_default_adapter();

    // Set up the global adapter for backward compatibility
    // This will be deprecated in favor of UiContext
    #[allow(deprecated)]
    set_global_adapter((*adapter).clone());

    // Set up the refresh callback (replaces direct rnk::request_render() in sage-core)
    set_refresh_callback(|| {
        rnk::request_render();
    });

    // Create UiContext for dependency injection (new approach)
    let ui_context = UiContext::new(Arc::new(rnk_sink));

    // Create shared state with config info
    let mut initial_state = UiState::default();
    initial_state.session.model = model.clone();
    initial_state.session.provider = provider.clone();

    let state: SharedState = Arc::new(RwLock::new(initial_state));
    let _ = GLOBAL_STATE.set(Arc::clone(&state));

    // Create command channel
    let (cmd_tx, cmd_rx) = mpsc::channel::<UiCommand>(16);
    let _ = GLOBAL_CMD_TX.set(cmd_tx);

    // Create input channel for executor
    let (input_channel, input_handle) = InputChannel::new(16);

    // Spawn input handler for tool/user interaction requests
    let input_handle_task = tokio::spawn(async move {
        crate::commands::unified::handle_user_input(input_handle, false).await;
    });

    // Spawn executor task with UI context
    let executor_state = Arc::clone(&state);
    let executor_ui_context = ui_context.clone();
    let config_file = cli.config_file.clone();
    let working_dir = cli.working_dir.clone();
    let max_steps = cli.max_steps;
    let executor_task = tokio::spawn(async move {
        executor_loop(
            executor_state,
            cmd_rx,
            input_channel,
            executor_ui_context,
            config_file,
            working_dir,
            max_steps,
        )
        .await;
    });

    // Background task for printing messages and updating spinner
    let bg_state = Arc::clone(&state);
    let bg_adapter = (*adapter).clone();
    let background_task = tokio::spawn(async move {
        background_loop(bg_state, bg_adapter).await;
    });

    // Small delay to ensure background thread starts and prints header
    sleep(Duration::from_millis(100)).await;

    // Run rnk app in inline mode (preserves terminal history)
    let result = render(app).run();

    // Clean up spawned tasks when the UI exits
    input_handle_task.abort();
    executor_task.abort();
    background_task.abort();

    result
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
