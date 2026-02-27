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

    // Handle keyboard input
    use_input({
        let state = Arc::clone(state);
        let cmd_tx = cmd_tx.clone();

        move |ch, key| {
            let mut command: Option<UiCommand> = None;

            {
                let mut s = state.write();

                // Ctrl+C to quit
                if key.ctrl && ch == "c" {
                    s.should_quit = true;
                    command = Some(UiCommand::Quit);
                } else if key.escape {
                    // ESC to cancel or exit model select mode
                    if s.model_select_mode {
                        s.model_select_mode = false;
                        s.model_select_index = 0;
                        s.available_models.clear();
                    } else if s.is_busy {
                        command = Some(UiCommand::Cancel);
                    } else {
                        s.suggestion_index = 0;
                    }
                } else if key.shift && ch == "\t" {
                    // Shift+Tab to cycle permission mode
                    s.permission_mode = s.permission_mode.cycle();
                } else if s.is_busy {
                    // Don't accept input while busy
                    return;
                } else if s.model_select_mode {
                    // Model selection mode
                    if key.up_arrow {
                        if s.model_select_index > 0 {
                            s.model_select_index -= 1;
                        }
                    } else if key.down_arrow {
                        let max = s.available_models.len().saturating_sub(1);
                        if s.model_select_index < max {
                            s.model_select_index += 1;
                        }
                    } else if key.return_key || (key.tab && !key.shift) {
                        if let Some(m) = s.available_models.get(s.model_select_index).cloned() {
                            command = Some(UiCommand::Submit(format!("/model {}", m)));
                        }
                        s.model_select_mode = false;
                        s.model_select_index = 0;
                        s.available_models.clear();
                    }
                    // Ignore other keys in model mode
                } else if key.up_arrow {
                    // Normal mode - Up arrow
                    if s.input_text.starts_with('/') && s.suggestion_index > 0 {
                        s.suggestion_index -= 1;
                    }
                } else if key.down_arrow {
                    // Down arrow - move selection down (with clamping)
                    if s.input_text.starts_with('/') {
                        let max_count = count_matching_commands(&s.input_text);
                        if s.suggestion_index < max_count.saturating_sub(1) {
                            s.suggestion_index += 1;
                        }
                    }
                } else if key.tab && !key.shift {
                    // Tab - auto-complete selected command
                    if let Some(cmd) = get_selected_command(&s.input_text, s.suggestion_index) {
                        s.input_text = cmd;
                        s.suggestion_index = 0;
                    }
                } else if key.backspace {
                    // Backspace
                    s.input_text.pop();
                    s.suggestion_index = 0;
                } else if key.return_key {
                    // Enter to submit
                    let text = if s.input_text.starts_with('/') {
                        get_selected_command(&s.input_text, s.suggestion_index)
                            .unwrap_or_else(|| s.input_text.clone())
                    } else {
                        s.input_text.clone()
                    };
                    s.input_text.clear();
                    s.suggestion_index = 0;
                    if !text.is_empty() {
                        command = Some(UiCommand::Submit(text));
                    }
                } else if !ch.is_empty() && !key.ctrl && !key.alt {
                    // Regular character input
                    s.input_text.push_str(ch);
                    s.suggestion_index = 0;
                }
            }

            if let Some(cmd) = command {
                if let Err(e) = cmd_tx.try_send(cmd) {
                    tracing::warn!("Failed to send UI command: {}", e);
                }
            }
        }
    });

    // Read state snapshot and check if should quit
    let (
        is_busy,
        input_text,
        status_text,
        permission_mode,
        suggestion_index,
        animation_frame,
        model_name,
        model_select_mode,
        available_models,
        model_select_index,
    ) = {
        let ui_state = state.read();
        if ui_state.should_quit {
            drop(ui_state);
            app_ctx.exit();
            return Text::new("Goodbye!").into_element();
        }

        (
            ui_state.is_busy,
            ui_state.input_text.clone(),
            ui_state.status_text.clone(),
            ui_state.permission_mode,
            ui_state.suggestion_index,
            ui_state.animation_frame,
            ui_state.session.model.clone(),
            ui_state.model_select_mode,
            ui_state.available_models.clone(),
            ui_state.model_select_index,
        )
    };

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
