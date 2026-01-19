//! rnk App Mode - Claude Code-style UI with terminal native scrolling
//!
//! This module implements a UI similar to Claude Code:
//! - Messages are printed using println + rnk::render_to_string_auto
//! - Terminal native scrolling for message history
//! - Header is printed once at startup, then scrolls away
//! - Simple input with status bar before prompt

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal,
};
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
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use unicode_width::UnicodeWidthChar;

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
    pub fn display_text(self) -> &'static str {
        match self {
            PermissionMode::Normal => "permissions required",
            PermissionMode::Bypass => "bypass permissions",
            PermissionMode::Plan => "plan mode",
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            PermissionMode::Normal => "\x1b[33m", // Yellow
            PermissionMode::Bypass => "\x1b[31m", // Red
            PermissionMode::Plan => "\x1b[36m",   // Cyan
        }
    }
}

/// Print rnk element to stdout
fn print_element(element: &Element) {
    let output = rnk::render_to_string_auto(element);
    println!("{}", output);
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

/// Format a message for printing
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

/// Render user message
fn render_user_message(text: &str) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("user: ").color(Color::Blue).bold().into_element())
        .child(Text::new(text).color(Color::Blue).into_element())
        .into_element()
}

/// Render prompt with status bar
fn print_status_and_prompt(permission_mode: PermissionMode) {
    let mode_color = permission_mode.color();
    let mode_text = permission_mode.display_text();
    // Status bar line
    print!(
        "{}⏵⏵ {} (shift+tab to cycle)\x1b[0m\n",
        mode_color, mode_text
    );
    // Prompt
    print!("\x1b[32;1m❯\x1b[0m ");
    io::stdout().flush().unwrap();
}

/// Render goodbye message
fn render_goodbye() -> Element {
    Text::new("Goodbye!").dim().into_element()
}

/// Read a line of input with proper CJK character handling and shift+tab detection
fn read_line_with_cjk(permission_mode: &mut PermissionMode) -> io::Result<Option<String>> {
    let mut input = String::new();
    let mut stdout = io::stdout();

    terminal::enable_raw_mode()?;

    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
            {
                match code {
                    KeyCode::Enter => {
                        print!("\r\n");
                        stdout.flush()?;
                        break;
                    }
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        terminal::disable_raw_mode()?;
                        return Ok(None); // Signal to quit
                    }
                    KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
                        terminal::disable_raw_mode()?;
                        return Ok(None); // Signal to quit
                    }
                    KeyCode::BackTab => {
                        // Shift+Tab - cycle permission mode
                        *permission_mode = match *permission_mode {
                            PermissionMode::Normal => PermissionMode::Bypass,
                            PermissionMode::Bypass => PermissionMode::Plan,
                            PermissionMode::Plan => PermissionMode::Normal,
                        };
                        // Redraw status and prompt
                        // Move up 2 lines, clear to end of screen, reprint
                        print!("\x1b[2A\x1b[J");
                        stdout.flush()?;
                        terminal::disable_raw_mode()?;
                        print_status_and_prompt(*permission_mode);
                        terminal::enable_raw_mode()?;
                        // Reprint current input
                        print!("{}", input);
                        stdout.flush()?;
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                        print!("{}", c);
                        stdout.flush()?;
                    }
                    KeyCode::Backspace => {
                        if let Some(ch) = input.pop() {
                            let char_width = ch.width().unwrap_or(1);
                            for _ in 0..char_width {
                                print!("\x08 \x08");
                            }
                            stdout.flush()?;
                        }
                    }
                    KeyCode::Esc => {
                        // Clear input on Escape
                        let total_width: usize =
                            input.chars().map(|c| c.width().unwrap_or(1)).sum();
                        for _ in 0..total_width {
                            print!("\x08 \x08");
                        }
                        stdout.flush()?;
                        input.clear();
                    }
                    _ => {}
                }
            }
        }
    }

    terminal::disable_raw_mode()?;
    Ok(Some(input))
}

/// Spinner for loading animation with ESC cancellation support
pub struct Spinner {
    running: Arc<AtomicBool>,
    cancel_rx: watch::Receiver<bool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Spinner {
    pub fn new(message: &str) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let message = message.to_string();

        let handle = std::thread::spawn(move || {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut i = 0;

            let _ = terminal::enable_raw_mode();

            while running_clone.load(Ordering::Relaxed) {
                // Check for ESC key
                if event::poll(Duration::from_millis(80)).unwrap_or(false) {
                    if let Ok(Event::Key(KeyEvent {
                        code: KeyCode::Esc, ..
                    })) = event::read()
                    {
                        let _ = cancel_tx.send(true);
                        running_clone.store(false, Ordering::Relaxed);
                        break;
                    }
                }

                print!(
                    "\x1b[2K\r\x1b[33m{} {} \x1b[2m(ESC to cancel)\x1b[0m",
                    frames[i], message
                );
                io::stdout().flush().unwrap();
                i = (i + 1) % frames.len();
            }

            let _ = terminal::disable_raw_mode();
            print!("\x1b[2K\r");
            io::stdout().flush().unwrap();
        });

        Self {
            running,
            cancel_rx,
            handle: Some(handle),
        }
    }

    pub fn get_cancel_receiver(&self) -> watch::Receiver<bool> {
        self.cancel_rx.clone()
    }

    pub fn stop(mut self) -> bool {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        *self.cancel_rx.borrow()
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
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

/// Run the rnk-based app (async version)
/// Uses simple println + render_to_string pattern like glm_chat
pub async fn run_rnk_app() -> io::Result<()> {
    // Initialize event adapter
    let adapter = EventAdapter::with_default_state();
    set_global_adapter(adapter.clone());

    // Get initial session info
    let session = SessionState {
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
    };

    // Print header
    println!();
    print_element(&render_header(&session));
    println!();

    // Create executor
    let mut executor = create_executor().await.map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("Init error: {}", e))
    })?;

    // Set up input channel
    let (input_channel, _input_handle) = InputChannel::new(16);
    executor.set_input_channel(input_channel);

    // Permission mode
    let mut permission_mode = PermissionMode::Normal;

    // Track printed messages
    let mut printed_count = 0;

    // Main loop
    loop {
        // Print status bar and prompt
        print_status_and_prompt(permission_mode);

        // Read user input
        let input = match read_line_with_cjk(&mut permission_mode)? {
            Some(input) => input,
            None => {
                // Ctrl+C or Ctrl+D
                println!();
                print_element(&render_goodbye());
                println!();
                break;
            }
        };

        let input = input.trim();

        // Handle special commands
        match input.to_lowercase().as_str() {
            "quit" | "exit" | "/exit" | "/quit" => {
                println!();
                print_element(&render_goodbye());
                println!();
                break;
            }
            "clear" => {
                print!("\x1b[2J\x1b[H");
                print_element(&render_header(&session));
                println!();
                continue;
            }
            "" => continue,
            _ => {}
        }

        // Clear the status and prompt lines, then reprint with user message
        print!("\x1b[2A\x1b[J");
        print_element(&render_user_message(input));
        println!();

        // Create task
        let working_dir = std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let task = TaskMetadata::new(input, &working_dir);

        // Emit events
        emit_event(AgentEvent::UserInputReceived {
            input: input.to_string(),
        });
        emit_event(AgentEvent::ThinkingStarted);

        // Show spinner while executing
        let spinner = Spinner::new("Thinking...");
        let cancel_rx = spinner.get_cancel_receiver();

        // Execute task with cancellation support
        let result = tokio::select! {
            result = executor.execute(task) => result,
            _ = async {
                let mut rx = cancel_rx;
                loop {
                    rx.changed().await.ok();
                    if *rx.borrow() {
                        break;
                    }
                }
            } => {
                Err(sage_core::error::SageError::Cancelled)
            }
        };

        let was_cancelled = spinner.stop();

        // Print any new messages from adapter
        let app_state = adapter.get_state();
        let messages = app_state.display_messages();
        let new_count = messages.len();

        if new_count > printed_count {
            for msg in messages.iter().skip(printed_count) {
                print_element(&format_message(msg));
                println!();
            }
            printed_count = new_count;
        }

        // Handle result
        if was_cancelled {
            println!("\x1b[33m⦻ Cancelled\x1b[0m");
        } else if let Err(e) = result {
            println!("\x1b[31mError: {}\x1b[0m", e);
        }

        println!();
    }

    Ok(())
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
