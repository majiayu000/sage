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

use crossterm::terminal;
use parking_lot::RwLock;
use rnk::prelude::*;
use sage_core::agent::{ExecutionMode, ExecutionOptions, UnifiedExecutor};
use sage_core::config::load_config;
use sage_core::error::SageResult;
use sage_core::input::InputChannel;
use sage_core::output::OutputMode;
use sage_core::types::TaskMetadata;
use sage_core::ui::bridge::state::{ExecutionPhase, Message, MessageContent, Role, SessionState};
use sage_core::ui::bridge::{emit_event, set_global_adapter, AgentEvent, EventAdapter};
use sage_tools::get_default_tools;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

// Alias rnk's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as RnkBox;

/// Permission mode for the UI
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionMode {
    Normal,
    Bypass,
    Plan,
}

impl PermissionMode {
    pub fn cycle(self) -> Self {
        match self {
            PermissionMode::Normal => PermissionMode::Bypass,
            PermissionMode::Bypass => PermissionMode::Plan,
            PermissionMode::Plan => PermissionMode::Normal,
        }
    }

    pub fn display_text(self) -> &'static str {
        match self {
            PermissionMode::Normal => "permissions required",
            PermissionMode::Bypass => "bypass permissions",
            PermissionMode::Plan => "plan mode",
        }
    }

    pub fn color(self) -> Color {
        match self {
            PermissionMode::Normal => Color::Yellow,
            PermissionMode::Bypass => Color::Red,
            PermissionMode::Plan => Color::Cyan,
        }
    }
}

/// UI state shared between render loop and background tasks
pub struct UiState {
    /// Current input text
    pub input_text: String,
    /// Permission mode
    pub permission_mode: PermissionMode,
    /// Whether agent is busy
    pub is_busy: bool,
    /// Status text
    pub status_text: String,
    /// Should quit
    pub should_quit: bool,
    /// Number of messages already printed
    pub printed_count: usize,
    /// Header already printed
    pub header_printed: bool,
    /// Session info
    pub session: SessionState,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            input_text: String::new(),
            permission_mode: PermissionMode::Normal,
            is_busy: false,
            status_text: String::new(),
            should_quit: false,
            printed_count: 0,
            header_printed: false,
            session: SessionState {
                session_id: None,
                model: "unknown".to_string(),
                provider: "unknown".to_string(),
                working_dir: std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                git_branch: None,
                step: 0,
                max_steps: None,
            },
        }
    }
}

/// Shared state wrapper
pub type SharedState = Arc<RwLock<UiState>>;

/// Command from UI to executor
#[derive(Debug)]
pub enum UiCommand {
    Submit(String),
    Cancel,
    Quit,
}

/// Global state for the app component
static GLOBAL_STATE: std::sync::OnceLock<SharedState> = std::sync::OnceLock::new();
static GLOBAL_CMD_TX: std::sync::OnceLock<mpsc::Sender<UiCommand>> = std::sync::OnceLock::new();
static GLOBAL_ADAPTER: std::sync::OnceLock<EventAdapter> = std::sync::OnceLock::new();

/// Format a message for printing via rnk::println
fn format_message(msg: &Message) -> Element {
    let term_width = terminal::size().map(|(w, _)| w as usize).unwrap_or(80);

    match &msg.content {
        MessageContent::Text(text) => {
            let (prefix, color) = match msg.role {
                Role::User => ("user: ", Color::Blue),
                Role::Assistant => ("assistant: ", Color::Green),
                Role::System => ("system: ", Color::Cyan),
            };

            let mut container = RnkBox::new().flex_direction(FlexDirection::Column);
            let lines = wrap_text_with_prefix(prefix, text, term_width);

            for (i, line) in lines.iter().enumerate() {
                let text_elem = if i == 0 {
                    Text::new(line.as_str()).color(color).bold()
                } else {
                    Text::new(line.as_str()).color(color)
                };
                container = container.child(text_elem.into_element());
            }

            container.into_element()
        }
        MessageContent::Thinking(text) => {
            let preview: String = text.lines().take(3).collect::<Vec<_>>().join(" ");
            Text::new(format!(
                "thinking: {}...",
                truncate_to_width(&preview, term_width.saturating_sub(12))
            ))
            .color(Color::BrightBlack)
            .italic()
            .into_element()
        }
        MessageContent::ToolCall {
            tool_name,
            params,
            result,
        } => {
            let mut container = RnkBox::new().flex_direction(FlexDirection::Column);

            // Tool header
            container = container.child(
                Text::new(format!("● {}", tool_name))
                    .color(Color::Magenta)
                    .bold()
                    .into_element(),
            );

            // Params
            if !params.trim().is_empty() {
                let param_lines = wrap_text_with_prefix("  args: ", params, term_width);
                for line in param_lines {
                    container =
                        container.child(Text::new(line).color(Color::Magenta).into_element());
                }
            }

            // Result
            if let Some(r) = result {
                let (label, color, content) = if r.success {
                    ("  ⎿ ", Color::Ansi256(245), r.output.as_deref().unwrap_or(""))
                } else {
                    (
                        "  ✗ ",
                        Color::Red,
                        r.error.as_deref().unwrap_or("Unknown error"),
                    )
                };
                if !content.is_empty() {
                    let result_lines = wrap_text_with_prefix(label, content, term_width);
                    for line in result_lines {
                        container = container.child(Text::new(line).color(color).into_element());
                    }
                }
            }

            container.into_element()
        }
    }
}

/// Render header banner
fn render_header(session: &SessionState) -> Element {
    let version = env!("CARGO_PKG_VERSION");
    let term_width = terminal::size().map(|(w, _)| w as usize).unwrap_or(80);

    let title = format!("▐▛███▜▌   Sage Code v{}", version);
    let model_info = format!("{} · {}", session.model, session.provider);
    let model_line = format!("▝▜█████▛▘  {}", model_info);
    let cwd_line = format!("  ▘▘ ▝▝    {}", session.working_dir);
    let hint_line = "  /model to try another model";

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new(truncate_to_width(&title, term_width))
                .color(Color::Cyan)
                .bold()
                .into_element(),
        )
        .child(
            Text::new(truncate_to_width(&model_line, term_width))
                .color(Color::Blue)
                .into_element(),
        )
        .child(
            Text::new(truncate_to_width(&cwd_line, term_width))
                .color(Color::BrightBlack)
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(
            Text::new(truncate_to_width(hint_line, term_width))
                .color(Color::BrightBlack)
                .into_element(),
        )
        .into_element()
}

/// The main app component - renders fixed bottom UI (separator + input/spinner + status bar)
fn app() -> Element {
    let app_ctx = use_app();

    // Get shared state
    let state = GLOBAL_STATE.get().expect("State not initialized");
    let cmd_tx = GLOBAL_CMD_TX.get().expect("Command channel not initialized");

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
                let _ = cmd_tx.blocking_send(UiCommand::Quit);
                app_ctx.exit();
                return;
            }

            // Shift+Tab to cycle permission mode
            if key.tab && key.shift {
                let mut s = state.write();
                s.permission_mode = s.permission_mode.cycle();
                return;
            }

            // Enter to submit
            if key.return_key {
                let mut s = state.write();
                if !s.is_busy && !s.input_text.is_empty() {
                    let text = std::mem::take(&mut s.input_text);
                    // Print user message via rnk::println
                    rnk::println(
                        RnkBox::new()
                            .flex_direction(FlexDirection::Row)
                            .child(Text::new("user: ").color(Color::Blue).bold().into_element())
                            .child(Text::new(&text).color(Color::Blue).into_element())
                            .into_element(),
                    );
                    rnk::println(""); // Empty line
                    s.printed_count += 1;
                    drop(s);
                    let _ = cmd_tx.blocking_send(UiCommand::Submit(text));
                }
                return;
            }

            // ESC to cancel
            if key.escape {
                let s = state.read();
                if s.is_busy {
                    drop(s);
                    let _ = cmd_tx.blocking_send(UiCommand::Cancel);
                }
                return;
            }

            // Backspace
            if key.backspace {
                let mut s = state.write();
                if !s.is_busy {
                    s.input_text.pop();
                }
                return;
            }

            // Regular character input
            if !ch.is_empty() && !key.ctrl && !key.alt {
                let mut s = state.write();
                if !s.is_busy {
                    s.input_text.push_str(ch);
                }
            }
        }
    });

    // Read state for rendering
    let ui_state = state.read();
    let separator = "─".repeat(term_width as usize);

    // Build the bottom UI
    let input_or_spinner = if ui_state.is_busy {
        render_spinner(&ui_state.status_text)
    } else {
        render_input(&ui_state.input_text)
    };

    let status_bar = render_status_bar(ui_state.permission_mode);

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        // Separator line
        .child(
            Text::new(separator)
                .color(Color::Ansi256(240))
                .into_element(),
        )
        // Input or spinner
        .child(input_or_spinner)
        // Status bar
        .child(status_bar)
        .into_element()
}

/// Render input line
fn render_input(input_text: &str) -> Element {
    let display_text = if input_text.is_empty() {
        "Try \"edit base.rs to...\""
    } else {
        input_text
    };
    let text_color = if input_text.is_empty() {
        Color::BrightBlack
    } else {
        Color::White
    };

    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("❯ ").color(Color::Green).bold().into_element())
        .child(Text::new(display_text).color(text_color).into_element())
        .into_element()
}

/// Render spinner line
fn render_spinner(status_text: &str) -> Element {
    // Use time-based frame selection for animation
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner = spinner_frames[(now_ms / 80 % spinner_frames.len() as u128) as usize];

    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new(spinner).color(Color::Yellow).into_element())
        .child(
            Text::new(format!(" {} (ESC to cancel)", status_text))
                .color(Color::Yellow)
                .into_element(),
        )
        .into_element()
}

/// Render status bar
fn render_status_bar(permission_mode: PermissionMode) -> Element {
    let mode_color = permission_mode.color();
    let mode_text = permission_mode.display_text();

    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("⏵⏵ ").color(mode_color).into_element())
        .child(Text::new(mode_text).color(mode_color).into_element())
        .child(
            Text::new(" (shift+tab to cycle)")
                .color(Color::BrightBlack)
                .into_element(),
        )
        .into_element()
}

// === Text wrapping utilities ===

fn wrap_text_with_prefix(prefix: &str, text: &str, max_width: usize) -> Vec<String> {
    let prefix_width = unicode_width::UnicodeWidthStr::width(prefix);
    let text_width = max_width.saturating_sub(prefix_width);

    if text_width == 0 {
        return vec![truncate_to_width(prefix, max_width)];
    }

    let mut result = Vec::new();
    let mut first_line = true;

    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            if first_line {
                result.push(prefix.to_string());
                first_line = false;
            } else {
                result.push(String::new());
            }
            continue;
        }

        let wrapped = wrap_single_line(paragraph, text_width);
        for line in wrapped {
            if first_line {
                result.push(format!("{}{}", prefix, line));
                first_line = false;
            } else {
                let indent = " ".repeat(prefix_width);
                result.push(format!("{}{}", indent, line));
            }
        }
    }

    if result.is_empty() {
        result.push(prefix.to_string());
    }

    result
}

fn wrap_single_line(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![];
    }

    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for ch in text.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);

        if ch == '\t' {
            for _ in 0..2 {
                if current_width + 1 > max_width && current_width > 0 {
                    result.push(current_line);
                    current_line = String::new();
                    current_width = 0;
                }
                current_line.push(' ');
                current_width += 1;
            }
            continue;
        }

        if ch_width == 0 {
            continue;
        }

        if current_width + ch_width > max_width && current_width > 0 {
            result.push(current_line);
            current_line = String::new();
            current_width = 0;
        }

        current_line.push(ch);
        current_width += ch_width;
    }

    if !current_line.is_empty() || result.is_empty() {
        result.push(current_line);
    }

    result
}

fn truncate_to_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if unicode_width::UnicodeWidthStr::width(text) <= max_width {
        return text.to_string();
    }

    let mut trimmed = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width + 3 > max_width {
            break;
        }
        trimmed.push(ch);
        width += ch_width;
    }
    trimmed.push_str("...");
    trimmed
}

/// Create executor
async fn create_executor() -> SageResult<UnifiedExecutor> {
    let config = load_config()?;
    let working_dir = std::env::current_dir().unwrap_or_default();
    let mode = ExecutionMode::interactive();
    let options = ExecutionOptions::default()
        .with_mode(mode)
        .with_working_directory(&working_dir);

    let mut executor = UnifiedExecutor::with_options(config, options)?;
    executor.set_output_mode(OutputMode::Rnk);
    executor.register_tools(get_default_tools());
    let _ = executor.init_subagent_support();
    Ok(executor)
}

/// Executor loop in background
async fn executor_loop(state: SharedState, mut rx: mpsc::Receiver<UiCommand>, input_channel: InputChannel) {
    // Create executor
    let mut executor = match create_executor().await {
        Ok(e) => e,
        Err(e) => {
            rnk::println(
                Text::new(format!("Failed to create executor: {}", e))
                    .color(Color::Red)
                    .into_element(),
            );
            state.write().should_quit = true;
            rnk::request_render();
            return;
        }
    };
    executor.set_input_channel(input_channel);

    // Process commands
    while let Some(cmd) = rx.recv().await {
        match cmd {
            UiCommand::Submit(task) => {
                {
                    let mut s = state.write();
                    s.is_busy = true;
                    s.status_text = "Thinking...".to_string();
                }
                rnk::request_render();

                emit_event(AgentEvent::UserInputReceived { input: task.clone() });
                emit_event(AgentEvent::ThinkingStarted);

                // Execute task
                let working_dir = std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let task_meta = TaskMetadata::new(&task, &working_dir);

                match executor.execute(task_meta).await {
                    Ok(_) => {}
                    Err(e) => {
                        emit_event(AgentEvent::error("execution", e.to_string()));
                    }
                }

                {
                    let mut s = state.write();
                    s.is_busy = false;
                    s.status_text.clear();
                }
                rnk::request_render();
            }
            UiCommand::Cancel => {
                emit_event(AgentEvent::ThinkingStopped);
                rnk::println(
                    Text::new("⦻ Cancelled")
                        .color(Color::Yellow)
                        .dim()
                        .into_element(),
                );
                {
                    let mut s = state.write();
                    s.is_busy = false;
                    s.status_text.clear();
                }
                rnk::request_render();
            }
            UiCommand::Quit => {
                state.write().should_quit = true;
                rnk::request_render();
                break;
            }
        }
    }
}

/// Run the rnk-based app (async version)
pub async fn run_rnk_app() -> io::Result<()> {
    // Initialize event adapter
    let adapter = EventAdapter::with_default_state();
    set_global_adapter(adapter.clone());
    let _ = GLOBAL_ADAPTER.set(adapter.clone());

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

    // Background thread for:
    // 1. Printing header once
    // 2. Printing new messages from adapter
    // 3. Updating spinner animation
    let bg_state = Arc::clone(&state);
    let bg_adapter = adapter;
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(80));

            // Check if should quit
            {
                let s = bg_state.read();
                if s.should_quit {
                    break;
                }
            }

            // Print header once
            {
                let s = bg_state.read();
                if !s.header_printed {
                    drop(s);
                    let mut s = bg_state.write();
                    if !s.header_printed {
                        rnk::println(render_header(&s.session));
                        rnk::println(""); // Empty line
                        s.header_printed = true;
                    }
                }
            }

            // Check for new messages and print them
            {
                let app_state = bg_adapter.get_state();
                let messages = app_state.display_messages();
                let new_count = messages.len();

                let mut ui_state = bg_state.write();

                // Update busy state from adapter
                ui_state.is_busy = !matches!(app_state.phase, ExecutionPhase::Idle);
                if ui_state.is_busy && ui_state.status_text.is_empty() {
                    ui_state.status_text = app_state.status_text();
                }

                // Print new messages
                if new_count > ui_state.printed_count {
                    for msg in messages.iter().skip(ui_state.printed_count) {
                        drop(ui_state);
                        rnk::println(format_message(msg));
                        rnk::println(""); // Empty line
                        ui_state = bg_state.write();
                    }
                    ui_state.printed_count = new_count;
                }
            }

            // Request render to update spinner animation
            rnk::request_render();
        }
        running_clone.store(false, Ordering::Relaxed);
    });

    // Run rnk app in inline mode (preserves terminal history)
    render(app).run()
}

#[cfg(test)]
mod tests {
    use super::*;

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
