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
use rnk::hooks::set_mouse_enabled;
use sage_core::agent::{ExecutionMode, ExecutionOptions, UnifiedExecutor};
use sage_core::config::load_config;
use sage_core::error::SageResult;
use sage_core::input::InputChannel;
use sage_core::output::OutputMode;
use sage_core::types::TaskMetadata;
use sage_core::ui::bridge::state::{AppState, ExecutionPhase, Role, SessionState};
use sage_core::ui::bridge::{emit_event, set_global_adapter, AgentEvent, EventAdapter};
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
    /// Mouse capture enabled for scroll support
    pub mouse_enabled: bool,
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
            mouse_enabled: true,
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
    executor.set_output_mode(OutputMode::Rnk);
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
                emit_event(AgentEvent::UserInputReceived {
                    input: task.clone(),
                });
                emit_event(AgentEvent::ThinkingStarted);

                // Execute task
                let working_dir = std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let task_meta = TaskMetadata::new(&task, &working_dir);

                match executor.execute(task_meta).await {
                    Ok(_outcome) => {}
                    Err(e) => {
                        emit_event(AgentEvent::error("execution", e.to_string()));
                        let mut s = state.write();
                        s.error = Some(e.to_string());
                    }
                }
                rnk::request_render();
            }
            UiCommand::Cancel => {
                emit_event(AgentEvent::ThinkingStopped);
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
    let header_height = 6u16;
    let bottom_height = 3u16;
    let viewport_height = term_height
        .saturating_sub(header_height + bottom_height) as usize;


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

            // Ctrl+Y to toggle mouse capture (enable selection)
            if key.ctrl && ch == "y" {
                let mut s = state.write();
                s.mouse_enabled = !s.mouse_enabled;
                drop(s);
                rnk::request_render();
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
                rnk::request_render();
                return;
            }
            if key.down_arrow {
                scroll.scroll_down(1);
                rnk::request_render();
                return;
            }
            if key.page_up {
                scroll.page_up();
                rnk::request_render();
                return;
            }
            if key.page_down {
                scroll.page_down();
                rnk::request_render();
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
                        // If send fails, restore the input text so user doesn't lose their message
                        match cmd_tx.try_send(UiCommand::Submit(text.clone())) {
                            Ok(()) => {
                                log("try_send succeeded");
                            }
                            Err(mpsc::error::TrySendError::Full(_)) => {
                                log("try_send failed: channel full, restoring input");
                                // Restore input text since send failed
                                let mut s = state.write();
                                s.input_text = text;
                                s.error = Some("System busy, please try again".to_string());
                            }
                            Err(mpsc::error::TrySendError::Closed(_)) => {
                                log("try_send failed: channel closed");
                                let mut s = state.write();
                                s.input_text = text;
                                s.error = Some("Connection lost".to_string());
                            }
                        }
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

            // ESC to cancel - use try_send to avoid blocking UI
            if key.escape {
                let s = state.read();
                if !matches!(s.app_state.phase, ExecutionPhase::Idle) {
                    drop(s);
                    // Use try_send instead of blocking_send to prevent UI freeze
                    if let Err(e) = cmd_tx.try_send(UiCommand::Cancel) {
                        log(&format!("ESC cancel try_send failed: {:?}", e));
                    }
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

    // Re-read state for rendering
    let ui_state = state.read();
    let all_messages = ui_state.app_state.display_messages();
    let status_line = status_render_line(&ui_state.app_state);
    let all_lines = build_render_lines(&all_messages, status_line, term_width as usize);
    let total_lines = all_lines.len();
    scroll.set_content_size(term_width as usize, total_lines.max(1));
    scroll.set_viewport_size(term_width as usize, viewport_height);

    let scroll_offset = scroll.offset_y();
    let max_scroll = total_lines.saturating_sub(viewport_height);
    let scroll_percent = if max_scroll > 0 {
        Some(((scroll_offset.min(max_scroll) as f32 / max_scroll as f32) * 100.0) as u8)
    } else {
        None
    };

    // Build content area
    let content = if total_lines == 0 {
        RnkBox::new()
            .flex_direction(FlexDirection::Column)
            .width(term_width as i32)
            .into_element()
    } else {
        let visible_start = scroll_offset.min(total_lines.saturating_sub(viewport_height));
        let visible_end = (visible_start + viewport_height).min(total_lines);
        render_visible_lines(&all_lines[visible_start..visible_end], term_width)
    };

    // Build bottom area
    let separator = "─".repeat(term_width as usize);
    let header = render_header(&ui_state.app_state.session, term_width);

    if !ui_state.mouse_enabled {
        set_mouse_enabled(false);
    } else {
        // Handle mouse scroll
        use_mouse({
            let scroll = scroll.clone();
            move |mouse| {
                match mouse.action {
                    MouseAction::ScrollUp => {
                        scroll.scroll_up(3);
                        rnk::request_render();
                    }
                    MouseAction::ScrollDown => {
                        scroll.scroll_down(3);
                        rnk::request_render();
                    }
                    _ => {}
                }
            }
        });
    }
    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .align_items(AlignItems::FlexStart)  // Force left alignment
        .width(term_width as i32)
        .height(term_height as i32)
        // Header
        .child(header)
        // Content area with flex_grow
        .child(
            RnkBox::new()
                .flex_grow(1.0)
                .width(term_width as i32)
                .align_items(AlignItems::FlexStart)  // Force left alignment
                .flex_direction(FlexDirection::Column)
                .overflow_y(Overflow::Hidden)
                .child(content)
                .into_element(),
        )
        .child(Text::new(separator).color(Color::Black).into_element())
        .child(render_input_or_status(&ui_state.input_text, &ui_state.app_state.phase))
        .child(render_status_bar(
            ui_state.permission_mode,
            scroll_percent,
            ui_state.mouse_enabled,
        ))
        .into_element()
}

/// Represents a wrapped line with information about its origin
struct WrappedLine {
    text: String,
    /// True if this line is a continuation of the previous line (soft wrap)
    /// False if this is a new paragraph (hard line break from source)
    is_continuation: bool,
}

/// Wrap text into lines that fit within max_width
/// Returns WrappedLine with is_continuation flag to distinguish:
/// - Hard line breaks (explicit \n in source) -> is_continuation = false
/// - Soft wraps (word wrapping within a paragraph) -> is_continuation = true
fn wrap_text_lines(text: &str, max_width: usize) -> Vec<WrappedLine> {
    if max_width == 0 {
        return vec![];
    }

    let mut result = Vec::new();

    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            result.push(WrappedLine {
                text: String::new(),
                is_continuation: false,
            });
            continue;
        }

        let mut current_line = String::new();
        let mut current_width = 0;
        let mut first_line = true;

        for ch in paragraph.chars() {
            if ch == '\t' {
                for _ in 0..2 {
                    if current_width + 1 > max_width && current_width > 0 {
                        result.push(WrappedLine {
                            text: current_line,
                            is_continuation: !first_line,
                        });
                        current_line = String::new();
                        current_width = 0;
                        first_line = false;
                    }
                    current_line.push(' ');
                    current_width += 1;
                }
                continue;
            }

            let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if ch_width == 0 {
                continue;
            }

            if current_width + ch_width > max_width && current_width > 0 {
                result.push(WrappedLine {
                    text: current_line,
                    is_continuation: !first_line,
                });
                current_line = String::new();
                current_width = 0;
                first_line = false;
            }

            current_line.push(ch);
            current_width += ch_width;
        }

        if !current_line.is_empty() {
            result.push(WrappedLine {
                text: current_line,
                is_continuation: !first_line,
            });
        }
    }

    if result.is_empty() {
        result.push(WrappedLine {
            text: String::new(),
            is_continuation: false,
        });
    }

    result
}

struct RenderLine {
    text: String,
    color: Color,
    bold: bool,
}

fn build_render_lines(
    messages: &[sage_core::ui::bridge::state::Message],
    status_line: Option<RenderLine>,
    max_width: usize,
) -> Vec<RenderLine> {
    let mut lines = Vec::new();
    for msg in messages {
        append_message_lines(&mut lines, msg, max_width);
        lines.push(RenderLine {
            text: String::new(),
            color: Color::Black,
            bold: false,
        });
    }
    if let Some(line) = status_line {
        while lines.last().map(|l| l.text.is_empty()).unwrap_or(false) {
            lines.pop();
        }
        lines.push(line);
    }
    while lines.last().map(|l| l.text.is_empty()).unwrap_or(false) {
        lines.pop();
    }
    lines
}

fn append_message_lines(
    lines: &mut Vec<RenderLine>,
    msg: &sage_core::ui::bridge::state::Message,
    max_width: usize,
) {
    use sage_core::ui::bridge::state::MessageContent;

    match &msg.content {
        MessageContent::Text(text) => {
            let (prefix, color) = match msg.role {
                Role::User => ("user: ", Color::Black),
                Role::Assistant => ("assistant: ", Color::Black),
                Role::System => ("system: ", Color::Black),
            };
            append_wrapped_text(lines, prefix, color, true, text, max_width);
        }
        MessageContent::Thinking(text) => {
            append_wrapped_text(lines, "thinking: ", Color::Black, false, text, max_width);
        }
        MessageContent::ToolCall {
            tool_name,
            params,
            result,
        } => {
            let header = format!("tool: {}", tool_name);
            lines.push(RenderLine {
                text: truncate_to_width(&header, max_width),
                color: Color::Magenta,
                bold: true,
            });

            if !params.trim().is_empty() {
                append_wrapped_text(lines, "  args: ", Color::Magenta, false, params, max_width);
            }

            if let Some(r) = result {
                let (label, color, content) = if r.success {
                    ("  result: ", Color::Magenta, r.output.as_deref().unwrap_or(""))
                } else {
                    ("  error: ", Color::Magenta, r.error.as_deref().unwrap_or("Unknown error"))
                };
                if !content.is_empty() {
                    append_wrapped_text(lines, label, color, false, content, max_width);
                }
            }
        }
    }
}

fn append_wrapped_text(
    lines: &mut Vec<RenderLine>,
    prefix: &str,
    color: Color,
    bold: bool,
    text: &str,
    max_width: usize,
) {
    let prefix_width = unicode_width::UnicodeWidthStr::width(prefix);
    let text_width = max_width.saturating_sub(prefix_width);
    let wrapped = wrap_text_lines(text, text_width);
    let mut is_first_line = true;

    for line in wrapped {
        if is_first_line {
            is_first_line = false;
            let combined = format!("{}{}", prefix, line.text);
            lines.push(RenderLine {
                text: truncate_to_width(&combined, max_width),
                color,
                bold,
            });
            continue;
        }

        if line.is_continuation {
            let indent = " ".repeat(prefix_width);
            let combined = format!("{}{}", indent, line.text);
            lines.push(RenderLine {
                text: truncate_to_width(&combined, max_width),
                color,
                bold: false,
            });
        } else {
            lines.push(RenderLine {
                text: truncate_to_width(line.text.as_str(), max_width),
                color,
                bold: false,
            });
        }
    }
}

fn render_visible_lines(lines: &[RenderLine], width: u16) -> Element {
    let mut content_box = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .align_items(AlignItems::FlexStart)
        .width(width as i32);

    for line in lines {
        let mut text = Text::new(line.text.as_str()).color(line.color);
        if line.bold {
            text = text.bold();
        }
        content_box = content_box.child(text.into_element());
    }

    content_box.into_element()
}

/// Render header banner
fn render_header(session: &SessionState, width: u16) -> Element {
    let version = env!("CARGO_PKG_VERSION");
    let title = truncate_to_width(
        &format!("▐▛███▜▌   Sage Code v{}", version),
        width as usize,
    );
    let model_info = format!("{} · {}", session.model, session.provider);
    let model_line = truncate_to_width(
        &format!("▝▜█████▛▘  {}", model_info),
        width as usize,
    );
    let cwd_line = truncate_to_width(
        &format!("  ▘▘ ▝▝    {}", session.working_dir),
        width as usize,
    );
    let hint_line = truncate_to_width("  /model to try another model", width as usize);

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .width(width as i32)
        .child(
            Text::new(title)
                .color(Color::Black)
                .bold()
                .into_element(),
        )
        .child(
            Text::new(model_line)
                .color(Color::Black)
                .into_element(),
        )
        .child(Text::new(cwd_line).color(Color::Black).into_element())
        .child(Newline::new().into_element())
        .child(
            Text::new(hint_line)
                .color(Color::Black)
                .into_element(),
        )
        .child(Newline::new().into_element())
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
        ExecutionPhase::Idle | ExecutionPhase::Thinking | ExecutionPhase::Streaming { .. } | ExecutionPhase::ExecutingTool { .. } => {
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("❯ ").color(Color::Black).bold().into_element())
                .child(
                    Text::new(if input_text.is_empty() && matches!(phase, ExecutionPhase::Idle) {
                        "Try \"edit base.rs to...\""
                    } else {
                        input_text
                    })
                    .color(Color::Black)
                    .into_element(),
                )
                .into_element()
        }
        ExecutionPhase::WaitingConfirmation { prompt } => {
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("? ").color(Color::Black).bold().into_element())
                .child(Text::new(prompt).color(Color::Black).into_element())
                .into_element()
        }
        ExecutionPhase::Error { message } => {
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("✗ ").color(Color::Black).bold().into_element())
                .child(Text::new(message).color(Color::Black).into_element())
                .into_element()
        }
    }
}

/// Render status bar
fn render_status_bar(
    permission_mode: PermissionMode,
    scroll_percent: Option<u8>,
    mouse_enabled: bool,
) -> Element {
    let mode_indicator = match permission_mode {
        PermissionMode::Normal => ("⏵⏵", Color::Black),
        PermissionMode::Bypass => ("⏵⏵", Color::Black),
        PermissionMode::Plan => ("⏵⏵", Color::Black),
    };

    let mut row = RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new(mode_indicator.0).color(mode_indicator.1).into_element())
        .child(
            Text::new(format!(" {}", permission_mode.display_text()))
                .color(Color::Black)
                .into_element(),
        )
        .child(Text::new(" (shift+tab to cycle)").color(Color::Black).into_element());
    row = row.child(
        Text::new(format!(" | mouse {}", if mouse_enabled { "on" } else { "off" }))
            .color(Color::Black)
            .into_element(),
    );

    // Add scroll indicator if scrollable
    if let Some(percent) = scroll_percent {
        row = row.child(
            Text::new(format!(" [{:3}%]", percent))
                .color(Color::Black)
                .into_element(),
        );
    }

    row.into_element()
}

fn status_render_line(app_state: &AppState) -> Option<RenderLine> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner = spinner_frames[(now_ms / 80 % spinner_frames.len() as u128) as usize];
    let dot_count = ((now_ms / 400) % 4) as usize;
    let dots = ".".repeat(dot_count);

    match app_state.phase {
        ExecutionPhase::Idle => None,
        ExecutionPhase::Thinking
        | ExecutionPhase::Streaming { .. }
        | ExecutionPhase::ExecutingTool { .. } => Some(RenderLine {
            text: format!("{} {}{}", spinner, app_state.status_text(), dots),
            color: Color::Yellow,
            bold: false,
        }),
        ExecutionPhase::WaitingConfirmation { .. } | ExecutionPhase::Error { .. } => Some(RenderLine {
            text: app_state.status_text(),
            color: Color::Yellow,
            bold: false,
        }),
    }
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

#[cfg(test)]
mod tests {
    use super::{build_render_lines, status_render_line, wrap_text_lines};
    use chrono::Utc;
    use sage_core::ui::bridge::state::{
        AppState, Message, MessageContent, MessageMetadata, Role, ToolResult,
    };
    use unicode_width::UnicodeWidthStr;

    #[test]
    fn wrap_text_lines_cjk_respects_width() {
        let text = "你好世界这是一个很长的中文句子用于测试换行是否正确对齐";
        let max_width = 12;
        let lines = wrap_text_lines(text, max_width);

        assert!(!lines.is_empty(), "Expected wrapped lines");
        for line in lines {
            let width = UnicodeWidthStr::width(line.text.as_str());
            assert!(
                width <= max_width,
                "Line width {} exceeds max width {}: '{}'",
                width,
                max_width,
                line.text
            );
        }
    }

    #[test]
    fn wrap_text_lines_preserves_paragraph_breaks() {
        let text = "第一段\n\n第二段";
        let lines = wrap_text_lines(text, 10);
        let rendered: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert!(
            rendered.contains(&""),
            "Expected empty line to preserve paragraph break"
        );
        assert!(rendered.contains(&"第一段"));
        assert!(rendered.contains(&"第二段"));
    }

    #[test]
    fn tool_call_lines_respect_width() {
        let msg = Message {
            role: Role::Assistant,
            content: MessageContent::ToolCall {
                tool_name: "Read".to_string(),
                params: "这是一个很长很长的参数用于测试工具调用换行是否正常".to_string(),
                result: Some(ToolResult {
                    success: true,
                    output: Some("输出内容也很长很长需要换行显示以避免错位".to_string()),
                    error: None,
                    duration: std::time::Duration::from_millis(5),
                }),
            },
            timestamp: Utc::now(),
            metadata: MessageMetadata::default(),
        };

        let lines = build_render_lines(&[msg], None, 20);
        for line in lines {
            let width = UnicodeWidthStr::width(line.text.as_str());
            assert!(
                width <= 20,
                "Tool line width {} exceeds max width: '{}'",
                width,
                line.text
            );
        }
    }

    #[test]
    fn status_line_renders_for_streaming() {
        let mut state = AppState::default();
        state.start_streaming();
        let line = status_render_line(&state).expect("Streaming should produce status line");
        let lines = build_render_lines(&[], Some(line), 40);
        assert!(
            lines.iter().any(|l| l.text.contains("Streaming")),
            "Expected streaming status line in output"
        );
    }
}

/// Run the rnk-based app
pub fn run_rnk_app() -> io::Result<()> {
    let adapter = EventAdapter::with_default_state();
    set_global_adapter(adapter.clone());

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

    let state_for_updates = Arc::clone(&state);
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async move {
            let mut rx = adapter.subscribe();
            loop {
                if rx.changed().await.is_err() {
                    break;
                }
                let snapshot = rx.borrow().clone();
                {
                    let mut s = state_for_updates.write();
                    s.app_state = snapshot;
                }
                rnk::request_render();
            }
        });
    });

    let animation_state = Arc::clone(&state);
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(120));
        let s = animation_state.read();
        if s.should_quit {
            break;
        }
        if !matches!(s.app_state.phase, ExecutionPhase::Idle) {
            rnk::request_render();
        }
    });

    if let Ok(working_dir) = std::env::current_dir() {
        emit_event(AgentEvent::WorkingDirectoryChanged {
            path: working_dir.to_string_lossy().to_string(),
        });
    }

    // Run rnk app with fullscreen mode (like the demo)
    render(app).fullscreen().run()
}
