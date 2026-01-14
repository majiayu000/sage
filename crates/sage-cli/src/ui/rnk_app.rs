//! rnk App Mode - Declarative UI with fixed-bottom layout
//!
//! This module implements the Claude Code-style UI using rnk for rendering.
//! Key architecture:
//! - rnk render().fullscreen().run() for declarative rendering
//! - Tokio runtime in background thread for async operations
//! - Shared state via Arc<RwLock<UiState>>
//! - Cross-thread updates via rnk::request_render()

use parking_lot::RwLock;
use rnk::prelude::*;
use sage_core::agent::{ExecutionMode, ExecutionOptions, UnifiedExecutor};
use sage_core::config::load_config;
use sage_core::error::SageResult;
use sage_core::input::InputChannel;
use sage_core::output::OutputMode;
use sage_core::types::TaskMetadata;
use sage_core::ui::bridge::state::{AppState, ExecutionPhase, Role};
use sage_tools::get_default_tools;
use std::io;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::mpsc;

// Alias rnk's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as RnkBox;

/// Simple file logger for debugging
fn log(msg: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/sage_debug.log")
    {
        let _ = writeln!(f, "[{}] {}", chrono::Local::now().format("%H:%M:%S%.3f"), msg);
    }
}

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
            PermissionMode::Bypass => "bypass permissions on",
            PermissionMode::Plan => "plan mode",
        }
    }
}

/// UI state shared between event loop and executor
pub struct UiState {
    /// Core app state
    pub app_state: AppState,
    /// Permission mode
    pub permission_mode: PermissionMode,
    /// Should quit
    pub should_quit: bool,
    /// Error message to display
    pub error: Option<String>,
    /// Current input text
    pub input_text: String,
    /// Scroll offset (line index of first visible message)
    pub scroll_offset: usize,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            app_state: AppState::default(),
            permission_mode: PermissionMode::Normal,
            should_quit: false,
            error: None,
            input_text: String::new(),
            scroll_offset: 0,
        }
    }
}

/// Shared state wrapper
pub type SharedState = Arc<RwLock<UiState>>;

/// Command from UI to executor
#[derive(Debug)]
pub enum UiCommand {
    /// Submit a task
    Submit(String),
    /// Cancel current operation
    Cancel,
    /// Quit
    Quit,
}

/// Global state for the app component
static GLOBAL_STATE: std::sync::OnceLock<SharedState> = std::sync::OnceLock::new();
static GLOBAL_CMD_TX: std::sync::OnceLock<mpsc::Sender<UiCommand>> = std::sync::OnceLock::new();

/// Create executor in background thread
async fn create_executor() -> SageResult<UnifiedExecutor> {
    let config = load_config()?;
    let working_dir = std::env::current_dir().unwrap_or_default();
    let mode = ExecutionMode::interactive();
    let options = ExecutionOptions::default()
        .with_mode(mode)
        .with_working_directory(&working_dir);

    let mut executor = UnifiedExecutor::with_options(config, options)?;
    executor.set_output_mode(OutputMode::Streaming);
    executor.register_tools(get_default_tools());
    let _ = executor.init_subagent_support();
    Ok(executor)
}

/// Run executor loop in background
async fn executor_loop(
    state: SharedState,
    mut rx: mpsc::Receiver<UiCommand>,
    input_channel: InputChannel,
) {
    log("executor_loop: starting");
    // Create executor
    let mut executor = match create_executor().await {
        Ok(e) => {
            log("executor_loop: executor created successfully");
            e
        }
        Err(e) => {
            log(&format!("executor_loop: FAILED to create executor: {}", e));
            {
                let mut s = state.write();
                s.error = Some(format!("Failed to create executor: {}", e));
            }
            rnk::request_render();
            return;
        }
    };
    executor.set_input_channel(input_channel);

    log("executor_loop: waiting for commands");
    // Process commands
    while let Some(cmd) = rx.recv().await {
        log(&format!("executor_loop: received command: {:?}", cmd));
        match cmd {
            UiCommand::Submit(task) => {
                // Update state to thinking
                {
                    let mut s = state.write();
                    s.app_state.start_thinking();
                    s.app_state.add_user_message(task.clone());
                }
                rnk::request_render();

                // Execute task
                let working_dir = std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let task_meta = TaskMetadata::new(&task, &working_dir);

                match executor.execute(task_meta).await {
                    Ok(_outcome) => {
                        let mut s = state.write();
                        s.app_state.phase = ExecutionPhase::Idle;
                    }
                    Err(e) => {
                        let mut s = state.write();
                        s.app_state.phase = ExecutionPhase::Error {
                            message: e.to_string(),
                        };
                        s.error = Some(e.to_string());
                    }
                }
                rnk::request_render();
            }
            UiCommand::Cancel => {
                state.write().app_state.phase = ExecutionPhase::Idle;
                rnk::request_render();
            }
            UiCommand::Quit => {
                state.write().should_quit = true;
                break;
            }
        }
    }
}

/// The main app component using rnk hooks
fn app() -> Element {
    log("app() called - rendering");
    let app_ctx = use_app();
    let scroll = use_scroll();

    // Get shared state
    let state = GLOBAL_STATE.get().expect("State not initialized");
    let cmd_tx = GLOBAL_CMD_TX.get().expect("Command channel not initialized");

    // Read current state
    let ui_state = state.read();

    // Check if should quit
    if ui_state.should_quit {
        drop(ui_state);
        app_ctx.exit();
        return Text::new("Goodbye!").into_element();
    }

    // Get terminal size
    let (term_width, term_height) = crossterm::terminal::size().unwrap_or((80, 24));
    let viewport_height = term_height.saturating_sub(3) as usize;

    // Get messages and calculate scroll
    let all_messages = ui_state.app_state.display_messages();
    let total_messages = all_messages.len();

    // Update scroll content size
    scroll.set_content_size(term_width as usize, total_messages);
    scroll.set_viewport_size(term_width as usize, viewport_height);

    let scroll_offset = scroll.offset_y();
    let max_scroll = total_messages.saturating_sub(viewport_height);
    let scroll_percent = if max_scroll > 0 {
        Some(((scroll_offset.min(max_scroll) as f32 / max_scroll as f32) * 100.0) as u8)
    } else {
        None
    };

    // Drop the read lock before setting up handlers
    drop(ui_state);

    // Handle keyboard input
    use_input({
        let state = Arc::clone(state);
        let cmd_tx = cmd_tx.clone();
        let app_ctx = app_ctx.clone();
        let scroll = scroll.clone();

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
                drop(s);
                rnk::request_render();
                return;
            }

            // Arrow keys for scrolling
            if key.up_arrow {
                scroll.scroll_up(1);
                return;
            }
            if key.down_arrow {
                scroll.scroll_down(1);
                return;
            }
            if key.page_up {
                scroll.page_up();
                return;
            }
            if key.page_down {
                scroll.page_down();
                return;
            }

            // Enter to submit
            if key.return_key {
                log("Enter pressed");
                let mut s = state.write();
                log(&format!("Phase: {:?}", s.app_state.phase));
                if matches!(s.app_state.phase, ExecutionPhase::Idle) {
                    let text = std::mem::take(&mut s.input_text);
                    log(&format!("Input text: '{}'", text));
                    if !text.is_empty() {
                        log("Sending submit command...");
                        // Auto-scroll to bottom
                        scroll.scroll_to_bottom();
                        drop(s); // Drop lock before send
                        // Use try_send instead of blocking_send to avoid blocking rnk's event loop
                        let result = cmd_tx.try_send(UiCommand::Submit(text));
                        log(&format!("try_send result: {:?}", result));
                        rnk::request_render();
                        log("request_render called");
                    } else {
                        log("Text is empty, not sending");
                    }
                } else {
                    log("Not in Idle phase, ignoring Enter");
                }
                return;
            }

            // ESC to cancel
            if key.escape {
                let s = state.read();
                if !matches!(s.app_state.phase, ExecutionPhase::Idle) {
                    drop(s);
                    let _ = cmd_tx.blocking_send(UiCommand::Cancel);
                    rnk::request_render();
                }
                return;
            }

            // Backspace
            if key.backspace {
                let mut s = state.write();
                if matches!(s.app_state.phase, ExecutionPhase::Idle) {
                    s.input_text.pop();
                }
                drop(s);
                rnk::request_render();
                return;
            }

            // Regular character input
            if !ch.is_empty() && !key.ctrl && !key.alt {
                let mut s = state.write();
                if matches!(s.app_state.phase, ExecutionPhase::Idle) {
                    s.input_text.push_str(ch);
                }
                drop(s);
                rnk::request_render();
            }
        }
    });

    // Handle mouse scroll
    use_mouse({
        let scroll = scroll.clone();
        move |mouse| {
            match mouse.action {
                MouseAction::ScrollUp => {
                    scroll.scroll_up(3);
                }
                MouseAction::ScrollDown => {
                    scroll.scroll_down(3);
                }
                _ => {}
            }
        }
    });

    // Re-read state for rendering
    let ui_state = state.read();
    let all_messages = ui_state.app_state.display_messages();

    // Build content area
    let content = if all_messages.is_empty() {
        render_welcome()
    } else {
        let visible_start = scroll_offset.min(total_messages.saturating_sub(viewport_height));
        let visible_end = (visible_start + viewport_height).min(total_messages);

        let mut content_box = RnkBox::new().flex_direction(FlexDirection::Column);
        for msg in all_messages.iter().skip(visible_start).take(visible_end - visible_start) {
            content_box = content_box.child(render_message(msg));
        }
        content_box.into_element()
    };

    // Build bottom area
    let separator = "─".repeat(term_width as usize);

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .width(term_width as i32)
        .height(term_height as i32)
        // Content area with flex_grow
        .child(
            RnkBox::new()
                .flex_grow(1.0)
                .flex_direction(FlexDirection::Column)
                .overflow_y(Overflow::Hidden)
                .child(content)
                .into_element(),
        )
        // Separator
        .child(Text::new(separator).dim().into_element())
        // Input line
        .child(render_input_or_status(&ui_state.input_text, &ui_state.app_state.phase))
        // Status bar
        .child(render_status_bar(ui_state.permission_mode, scroll_percent))
        .into_element()
}

/// Render a message
fn render_message(msg: &sage_core::ui::bridge::state::Message) -> Element {
    use sage_core::ui::bridge::state::MessageContent;

    match &msg.content {
        MessageContent::Text(text) => match msg.role {
            Role::User => Message::user(text).into_element(),
            Role::Assistant => Message::assistant(text).into_element(),
            Role::System => Message::system(text).into_element(),
        },
        MessageContent::ToolCall {
            tool_name,
            params,
            result,
        } => {
            let mut container = RnkBox::new().flex_direction(FlexDirection::Column);

            // Tool call
            let display_params = if params.len() > 50 {
                format!("{}...", &params[..47])
            } else {
                params.clone()
            };
            container = container.child(ToolCall::new(tool_name, &display_params).into_element());

            // Tool result
            if let Some(r) = result {
                let output = r.output.as_deref().unwrap_or("");
                let display = if output.len() > 100 {
                    format!("{}...", &output[..97])
                } else {
                    output.to_string()
                };
                if r.success {
                    container = container.child(Message::tool_result(display).into_element());
                } else {
                    let err = r.error.as_deref().unwrap_or("Error");
                    container = container.child(Message::error(err).into_element());
                }
            }

            container.into_element()
        }
        MessageContent::Thinking(text) => ThinkingBlock::new(text).into_element(),
    }
}

/// Render welcome message
fn render_welcome() -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new("Sage Agent")
                .color(Color::Cyan)
                .bold()
                .into_element(),
        )
        .child(
            Text::new("Rust-based LLM Agent for software engineering tasks")
                .dim()
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(
            Text::new("Type a message to get started, or use /help for commands")
                .dim()
                .into_element(),
        )
        .into_element()
}

/// Render spinner indicator (text-based)
fn render_spinner_indicator(message: &str, color: Color) -> Element {
    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let frame_idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        / 80) as usize
        % frames.len();

    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new(frames[frame_idx]).color(color).into_element())
        .child(Text::new(format!(" {}", message)).color(color).into_element())
        .into_element()
}

/// Render input line or current status
fn render_input_or_status(input_text: &str, phase: &ExecutionPhase) -> Element {
    match phase {
        ExecutionPhase::Idle => {
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("❯ ").color(Color::Yellow).bold().into_element())
                .child(
                    Text::new(if input_text.is_empty() {
                        "Type your message..."
                    } else {
                        input_text
                    })
                    .color(if input_text.is_empty() {
                        Color::BrightBlack
                    } else {
                        Color::White
                    })
                    .into_element(),
                )
                .into_element()
        }
        ExecutionPhase::Thinking => {
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(render_spinner_indicator("Thinking...", Color::Magenta))
                .child(Text::new(" (ESC to cancel)").dim().into_element())
                .into_element()
        }
        ExecutionPhase::Streaming { .. } => {
            render_spinner_indicator("Streaming...", Color::Cyan)
        }
        ExecutionPhase::ExecutingTool { tool_name, .. } => {
            render_spinner_indicator(&format!("Running {}...", tool_name), Color::Blue)
        }
        ExecutionPhase::WaitingConfirmation { prompt } => {
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("? ").color(Color::Yellow).bold().into_element())
                .child(Text::new(prompt).into_element())
                .into_element()
        }
        ExecutionPhase::Error { message } => {
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("✗ ").color(Color::Red).bold().into_element())
                .child(Text::new(message).color(Color::Red).into_element())
                .into_element()
        }
    }
}

/// Render status bar
fn render_status_bar(permission_mode: PermissionMode, scroll_percent: Option<u8>) -> Element {
    let mode_indicator = match permission_mode {
        PermissionMode::Normal => ("▸▸", Color::BrightBlack),
        PermissionMode::Bypass => ("▸▸", Color::Yellow),
        PermissionMode::Plan => ("▸▸", Color::Blue),
    };

    let mut row = RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new(mode_indicator.0).color(mode_indicator.1).into_element())
        .child(
            Text::new(format!(" {}", permission_mode.display_text()))
                .dim()
                .into_element(),
        )
        .child(Text::new(" (shift+tab to cycle)").dim().into_element());

    // Add scroll indicator if scrollable
    if let Some(percent) = scroll_percent {
        row = row.child(Text::new(format!(" [{:3}%]", percent)).dim().into_element());
    }

    row.into_element()
}

/// Run the rnk-based app
pub fn run_rnk_app() -> io::Result<()> {
    // Create shared state
    let state: SharedState = Arc::new(RwLock::new(UiState::default()));
    let _ = GLOBAL_STATE.set(Arc::clone(&state));

    // Create command channel
    let (cmd_tx, cmd_rx) = mpsc::channel::<UiCommand>(16);
    let _ = GLOBAL_CMD_TX.set(cmd_tx);

    // Create input channel for executor
    let (input_channel, _input_handle) = InputChannel::new(16);

    // Clone state for executor thread
    let executor_state = Arc::clone(&state);

    // Spawn tokio runtime in background thread
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(executor_loop(executor_state, cmd_rx, input_channel));
    });

    // Run rnk app with fullscreen mode (like the demo)
    render(app).fullscreen().run()
}
