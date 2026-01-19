//! rnk App Mode - Claude Code-style UI with terminal native scrolling
//!
//! This module implements a UI similar to Claude Code:
//! - Messages are printed directly to terminal (persists in scrollback)
//! - Input uses raw mode for proper CJK handling
//! - No fullscreen/alternate buffer - uses native terminal scrolling
//! - Header is printed once at startup, then scrolls away
//!
//! Key architecture:
//! - Direct println for messages (no render loop)
//! - Raw mode only during input
//! - Spinner in separate thread during LLM calls

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

/// Print rnk element to stdout (with newline)
fn print_element(element: &Element) {
    let output = rnk::render_to_string_auto(element);
    println!("{}", output);
}

/// Print rnk element to stdout (without newline, for inline prompts)
fn print_element_inline(element: &Element) {
    let output = rnk::render_to_string_auto(element);
    print!("{}", output);
    let _ = io::stdout().flush();
}

/// Render header banner
fn render_header(session: &SessionState) -> Element {
    let version = env!("CARGO_PKG_VERSION");
    let term_width = crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80);

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

/// Render user message
fn render_user_message(text: &str) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(
            Text::new("user: ")
                .color(Color::Blue)
                .bold()
                .into_element(),
        )
        .child(Text::new(text).color(Color::Blue).into_element())
        .into_element()
}

/// Render assistant message
fn render_assistant_message(text: &str) -> Element {
    let term_width = crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80);

    let mut container = RnkBox::new().flex_direction(FlexDirection::Column);
    let lines = wrap_text_with_prefix("assistant: ", text, term_width);

    for (i, line) in lines.iter().enumerate() {
        let text_elem = if i == 0 {
            Text::new(line.as_str()).color(Color::Green).bold()
        } else {
            Text::new(line.as_str()).color(Color::Green)
        };
        container = container.child(text_elem.into_element());
    }

    container.into_element()
}

/// Render tool call
fn render_tool_call(tool_name: &str, params: &str) -> Element {
    let term_width = crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80);

    let mut container = RnkBox::new().flex_direction(FlexDirection::Column);

    // Tool header
    let header = format!("● {}", tool_name);
    container = container.child(
        Text::new(truncate_to_width(&header, term_width))
            .color(Color::Magenta)
            .bold()
            .into_element(),
    );

    // Params
    if !params.trim().is_empty() {
        let param_lines = wrap_text_with_prefix("  args: ", params, term_width);
        for line in param_lines {
            container = container.child(Text::new(line).color(Color::Magenta).into_element());
        }
    }

    container.into_element()
}

/// Render tool result
fn render_tool_result(output: Option<&str>, error: Option<&str>, success: bool) -> Element {
    let term_width = crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80);

    let (label, color, content) = if success {
        ("  ⎿ ", Color::Ansi256(245), output.unwrap_or(""))
    } else {
        ("  ✗ ", Color::Red, error.unwrap_or("Unknown error"))
    };

    if content.is_empty() {
        return Text::new("").into_element();
    }

    let mut container = RnkBox::new().flex_direction(FlexDirection::Column);
    let result_lines = wrap_text_with_prefix(label, content, term_width);
    for line in result_lines {
        container = container.child(Text::new(line).color(color).into_element());
    }

    container.into_element()
}

/// Render prompt
fn render_prompt() -> Element {
    Text::new("❯ ").color(Color::Green).bold().into_element()
}

/// Print a message from state
fn print_message(msg: &Message) {
    match &msg.content {
        MessageContent::Text(text) => match msg.role {
            Role::User => print_element(&render_user_message(text)),
            Role::Assistant => print_element(&render_assistant_message(text)),
            Role::System => {
                println!("\x1b[36msystem: {}\x1b[0m", text);
            }
        },
        MessageContent::Thinking(text) => {
            let preview: String = text.lines().take(3).collect::<Vec<_>>().join(" ");
            println!("\x1b[90mthinking: {}...\x1b[0m", truncate_to_width(&preview, 60));
        }
        MessageContent::ToolCall {
            tool_name,
            params,
            result,
        } => {
            print_element(&render_tool_call(tool_name, params));
            if let Some(r) = result {
                print_element(&render_tool_result(
                    r.output.as_deref(),
                    r.error.as_deref(),
                    r.success,
                ));
            }
        }
    }
}

/// Read a line of input with proper CJK character handling
fn read_line_with_cjk() -> io::Result<String> {
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
                        println!();
                        return Err(io::Error::new(io::ErrorKind::Interrupted, "Ctrl+C"));
                    }
                    KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
                        terminal::disable_raw_mode()?;
                        println!();
                        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Ctrl+D"));
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
                        // Clear input
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
    Ok(input)
}

/// Spinner for loading animation with ESC cancellation support
struct Spinner {
    running: Arc<AtomicBool>,
    cancel_rx: watch::Receiver<bool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Spinner {
    fn new(message: &str) -> Self {
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

    fn get_cancel_receiver(&self) -> watch::Receiver<bool> {
        self.cancel_rx.clone()
    }

    fn stop(mut self) -> bool {
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

/// Wrap text with a prefix
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

/// Wrap a single line
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

/// Run the chat loop (async version - uses existing tokio runtime)
pub async fn run_rnk_app() -> io::Result<()> {
    // Initialize event adapter for state tracking
    let adapter = EventAdapter::with_default_state();
    set_global_adapter(adapter.clone());

    // Create input channel
    let (input_channel, _input_handle) = InputChannel::new(16);

    // Print header
    println!();
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
    print_element(&render_header(&session));
    println!();

    // Create executor
    let mut executor = match create_executor().await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("\x1b[31mFailed to create executor: {}\x1b[0m", e);
            return Err(io::Error::new(io::ErrorKind::Other, e.to_string()));
        }
    };
    executor.set_input_channel(input_channel);

    // Track printed messages
    let mut printed_count = 0;

    loop {
        // Print any new messages from adapter
        let state = adapter.get_state();
        let messages = state.display_messages();
        for msg in messages.iter().skip(printed_count) {
            print_message(msg);
        }
        printed_count = messages.len();

        // Print prompt
        print_element_inline(&render_prompt());

        // Read input using spawn_blocking to avoid blocking the async runtime
        let input = match tokio::task::spawn_blocking(read_line_with_cjk).await {
            Ok(Ok(s)) => s,
            Ok(Err(e)) if e.kind() == io::ErrorKind::Interrupted => {
                println!("\x1b[33mGoodbye!\x1b[0m");
                break;
            }
            Ok(Err(e)) if e.kind() == io::ErrorKind::UnexpectedEof => {
                println!("\x1b[33mGoodbye!\x1b[0m");
                break;
            }
            Ok(Err(e)) => return Err(e),
            Err(e) => {
                eprintln!("\x1b[31mInput error: {}\x1b[0m", e);
                break;
            }
        };

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        // Handle special commands
        match input.to_lowercase().as_str() {
            "quit" | "exit" | "/quit" | "/exit" => {
                println!("\x1b[33mGoodbye!\x1b[0m");
                break;
            }
            "clear" | "/clear" => {
                print!("\x1b[2J\x1b[H");
                print_element(&render_header(&session));
                println!();
                printed_count = 0;
                continue;
            }
            _ => {}
        }

        // Print user message (move cursor up to replace the input line)
        print!("\x1b[1A\x1b[2K");
        print_element(&render_user_message(input));

        // Emit user input event
        emit_event(AgentEvent::UserInputReceived {
            input: input.to_string(),
        });

        // Start spinner in background
        let spinner = Spinner::new("Thinking...");
        let cancel_rx = spinner.get_cancel_receiver();

        // Execute task
        let working_dir = std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let task_meta = TaskMetadata::new(input, &working_dir);

        // Run with cancellation support
        let result = tokio::select! {
            result = executor.execute(task_meta) => Some(result),
            _ = async {
                let mut rx = cancel_rx;
                loop {
                    rx.changed().await.ok();
                    if *rx.borrow() {
                        break;
                    }
                }
            } => None, // Cancelled
        };

        let was_cancelled = spinner.stop();

        if was_cancelled {
            println!("\x1b[33m● Cancelled\x1b[0m");
            continue;
        }

        // Handle result
        match result {
            Some(Ok(_)) => {
                // Print any new messages
                let state = adapter.get_state();
                let messages = state.display_messages();
                for msg in messages.iter().skip(printed_count) {
                    print_message(msg);
                }
                printed_count = messages.len();
            }
            Some(Err(e)) => {
                println!("\x1b[31m● Error: {}\x1b[0m", e);
            }
            None => {
                // Cancelled, already handled above
            }
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
