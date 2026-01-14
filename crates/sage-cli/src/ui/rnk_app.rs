//! rnk App Mode - Declarative UI with fixed-bottom layout
//!
//! This module implements the Claude Code-style UI using rnk for rendering.
//! Key architecture:
//! - rnk render_to_string_auto() for declarative rendering
//! - Custom event loop for input handling (crossterm)
//! - Tokio runtime in background thread for async operations
//! - Shared state via Arc<RwLock<UiState>>
//! - Cross-thread updates via request_render pattern

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, EnableMouseCapture, DisableMouseCapture},
    terminal::{self, ClearType},
    cursor, execute,
};
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
use std::io::{self, Write};
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
    /// Needs redraw
    pub needs_redraw: bool,
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
            needs_redraw: true,
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
    // Create executor
    let mut executor = match create_executor().await {
        Ok(e) => e,
        Err(e) => {
            let mut s = state.write();
            s.error = Some(format!("Failed to create executor: {}", e));
            s.needs_redraw = true;
            return;
        }
    };
    executor.set_input_channel(input_channel);

    // Process commands
    while let Some(cmd) = rx.recv().await {
        match cmd {
            UiCommand::Submit(task) => {
                // Update state to thinking
                {
                    let mut s = state.write();
                    s.app_state.start_thinking();
                    s.app_state.add_user_message(task.clone());
                    s.needs_redraw = true;
                }

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
                        s.needs_redraw = true;
                    }
                    Err(e) => {
                        let mut s = state.write();
                        s.app_state.phase = ExecutionPhase::Error {
                            message: e.to_string(),
                        };
                        s.error = Some(e.to_string());
                        s.needs_redraw = true;
                    }
                }
            }
            UiCommand::Cancel => {
                let mut s = state.write();
                s.app_state.phase = ExecutionPhase::Idle;
                s.needs_redraw = true;
            }
            UiCommand::Quit => {
                state.write().should_quit = true;
                break;
            }
        }
    }
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

/// Render the full UI
fn render_ui(state: &UiState, term_width: u16, term_height: u16) -> Element {
    let all_messages = state.app_state.display_messages();
    let total_messages = all_messages.len();

    // Calculate viewport (reserve 3 lines for bottom area)
    let viewport_height = term_height.saturating_sub(3) as usize;

    // Calculate scroll parameters
    let max_scroll = total_messages.saturating_sub(viewport_height);
    let scroll_offset = state.scroll_offset.min(max_scroll);

    // Calculate scroll percentage
    let scroll_percent = if max_scroll > 0 {
        Some(((scroll_offset as f32 / max_scroll as f32) * 100.0) as u8)
    } else {
        None
    };

    // Content area
    let content = if all_messages.is_empty() {
        render_welcome()
    } else {
        let mut content_box = RnkBox::new().flex_direction(FlexDirection::Column);

        // Show messages based on scroll offset
        let end = (scroll_offset + viewport_height).min(total_messages);
        for msg in all_messages.iter().skip(scroll_offset).take(end - scroll_offset) {
            content_box = content_box.child(render_message(msg));
        }
        content_box.into_element()
    };

    // Bottom area with dynamic separator width
    let separator = "─".repeat(term_width as usize);
    let bottom = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new(separator).dim().into_element())
        .child(render_input_or_status(&state.input_text, &state.app_state.phase))
        .child(render_status_bar(state.permission_mode, scroll_percent))
        .into_element();

    // Full layout with explicit dimensions
    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .width(term_width as i32)
        .height(term_height as i32)
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Column)
                .flex_grow(1.0)
                .child(content)
                .into_element(),
        )
        .child(bottom)
        .into_element()
}

/// Clear screen and move cursor to top
fn clear_screen() -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
    Ok(())
}

/// Run the rnk-based app
pub fn run_rnk_app() -> io::Result<()> {
    // Create shared state
    let state: SharedState = Arc::new(RwLock::new(UiState::default()));

    // Create command channel
    let (cmd_tx, cmd_rx) = mpsc::channel::<UiCommand>(16);

    // Create input channel for executor
    let (input_channel, _input_handle) = InputChannel::new(16);

    // Clone state for executor thread
    let executor_state = state.clone();

    // Spawn tokio runtime in background thread
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(executor_loop(executor_state, cmd_rx, input_channel));
    });

    // Enter alternate screen and raw mode with mouse capture
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen, EnableMouseCapture)?;
    terminal::enable_raw_mode()?;

    // Get terminal size
    let (term_width, term_height) = terminal::size().unwrap_or((80, 24));

    // Initial render
    clear_screen()?;
    {
        let s = state.read();
        let element = render_ui(&*s, term_width, term_height);
        let output = rnk::render_to_string(&element, term_width);
        print!("{}", output);
        stdout.flush()?;
    }

    // Main event loop
    loop {
        // Check if should quit
        if state.read().should_quit {
            break;
        }

        // Poll for events with timeout
        if event::poll(Duration::from_millis(50))? {
            let evt = event::read()?;
            let mut needs_render = false;

            match evt {
                Event::Key(KeyEvent { code, modifiers, .. }) => {
                    match code {
                        // Ctrl+C or Ctrl+D to quit
                        KeyCode::Char('c') | KeyCode::Char('d')
                            if modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            let _ = cmd_tx.blocking_send(UiCommand::Quit);
                            break;
                        }

                        // ESC to cancel
                        KeyCode::Esc => {
                            let s = state.read();
                            if !matches!(s.app_state.phase, ExecutionPhase::Idle) {
                                let _ = cmd_tx.blocking_send(UiCommand::Cancel);
                            }
                        }

                        // Shift+Tab to cycle permission mode
                        KeyCode::BackTab => {
                            let mut s = state.write();
                            s.permission_mode = s.permission_mode.cycle();
                            needs_render = true;
                        }

                        // Arrow keys for scrolling
                        KeyCode::Up => {
                            let mut s = state.write();
                            if s.scroll_offset > 0 {
                                s.scroll_offset -= 1;
                                needs_render = true;
                            }
                        }
                        KeyCode::Down => {
                            let mut s = state.write();
                            let total = s.app_state.display_messages().len();
                            let viewport = term_height.saturating_sub(3) as usize;
                            let max_scroll = total.saturating_sub(viewport);
                            if s.scroll_offset < max_scroll {
                                s.scroll_offset += 1;
                                needs_render = true;
                            }
                        }
                        KeyCode::PageUp => {
                            let mut s = state.write();
                            let viewport = term_height.saturating_sub(3) as usize;
                            s.scroll_offset = s.scroll_offset.saturating_sub(viewport);
                            needs_render = true;
                        }
                        KeyCode::PageDown => {
                            let mut s = state.write();
                            let total = s.app_state.display_messages().len();
                            let viewport = term_height.saturating_sub(3) as usize;
                            let max_scroll = total.saturating_sub(viewport);
                            s.scroll_offset = (s.scroll_offset + viewport).min(max_scroll);
                            needs_render = true;
                        }
                        KeyCode::Home => {
                            let mut s = state.write();
                            s.scroll_offset = 0;
                            needs_render = true;
                        }
                        KeyCode::End => {
                            let mut s = state.write();
                            let total = s.app_state.display_messages().len();
                            let viewport = term_height.saturating_sub(3) as usize;
                            s.scroll_offset = total.saturating_sub(viewport);
                            needs_render = true;
                        }

                        // Enter to submit
                        KeyCode::Enter => {
                            let mut s = state.write();
                            if matches!(s.app_state.phase, ExecutionPhase::Idle) {
                                let text = std::mem::take(&mut s.input_text);
                                if !text.is_empty() {
                                    // Auto-scroll to bottom when submitting
                                    let total = s.app_state.display_messages().len() + 2; // +2 for new msgs
                                    let viewport = term_height.saturating_sub(3) as usize;
                                    s.scroll_offset = total.saturating_sub(viewport);
                                    let _ = cmd_tx.blocking_send(UiCommand::Submit(text));
                                }
                            }
                        }

                        // Backspace
                        KeyCode::Backspace => {
                            let mut s = state.write();
                            if matches!(s.app_state.phase, ExecutionPhase::Idle) {
                                s.input_text.pop();
                                needs_render = true;
                            }
                        }

                        // Regular character input
                        KeyCode::Char(c) => {
                            let mut s = state.write();
                            if matches!(s.app_state.phase, ExecutionPhase::Idle) {
                                s.input_text.push(c);
                                needs_render = true;
                            }
                        }

                        _ => {}
                    }
                }

                // Handle mouse scroll events
                Event::Mouse(MouseEvent { kind, .. }) => {
                    match kind {
                        MouseEventKind::ScrollUp => {
                            let mut s = state.write();
                            if s.scroll_offset >= 3 {
                                s.scroll_offset -= 3;
                            } else {
                                s.scroll_offset = 0;
                            }
                            needs_render = true;
                        }
                        MouseEventKind::ScrollDown => {
                            let mut s = state.write();
                            let total = s.app_state.display_messages().len();
                            let viewport = term_height.saturating_sub(3) as usize;
                            let max_scroll = total.saturating_sub(viewport);
                            s.scroll_offset = (s.scroll_offset + 3).min(max_scroll);
                            needs_render = true;
                        }
                        _ => {}
                    }
                }

                _ => {}
            }

            if needs_render {
                state.write().needs_redraw = true;
            }
        }

        // Check if needs redraw
        let should_redraw = {
            let mut s = state.write();
            let redraw = s.needs_redraw;
            s.needs_redraw = false;
            redraw
        };

        // Also redraw during non-idle phases for spinner animation
        let is_animating = {
            let s = state.read();
            !matches!(s.app_state.phase, ExecutionPhase::Idle)
        };

        if should_redraw || is_animating {
            // Get current terminal size (may have changed)
            let (tw, th) = terminal::size().unwrap_or((term_width, term_height));
            clear_screen()?;
            let s = state.read();
            let element = render_ui(&*s, tw, th);
            let output = rnk::render_to_string(&element, tw);
            print!("{}", output);
            stdout.flush()?;
        }
    }

    // Cleanup
    terminal::disable_raw_mode()?;
    execute!(stdout, DisableMouseCapture, terminal::LeaveAlternateScreen)?;

    Ok(())
}
