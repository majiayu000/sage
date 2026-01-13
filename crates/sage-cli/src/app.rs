//! Sage CLI Main Application
//!
//! Claude Code style UI using rnk components for rendering.
//! Uses command-loop pattern (like glm_chat.rs) for async Agent integration.

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
use sage_tools::get_default_tools;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use unicode_width::UnicodeWidthChar;

// Alias rnk's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as RnkBox;

// ============================================================================
// UI Components (rnk Elements)
// ============================================================================

/// Render banner at the top
fn render_banner() -> Element {
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
        .into_element()
}

/// Render user message
fn render_user_message(text: &str) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("> ").color(Color::Yellow).bold().into_element())
        .child(Text::new(text).color(Color::BrightWhite).into_element())
        .into_element()
}

/// Render input prompt
fn render_prompt() -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("> ").color(Color::Yellow).bold().into_element())
        .into_element()
}

/// Render assistant response
fn render_assistant_response(text: &str) -> Element {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= 1 {
        RnkBox::new()
            .flex_direction(FlexDirection::Row)
            .child(Text::new("● ").color(Color::BrightWhite).into_element())
            .child(Text::new(text).color(Color::White).into_element())
            .into_element()
    } else {
        let mut container = RnkBox::new().flex_direction(FlexDirection::Column);
        for (i, line) in lines.iter().enumerate() {
            let prefix = if i == 0 { "● " } else { "  " };
            container = container.child(
                RnkBox::new()
                    .flex_direction(FlexDirection::Row)
                    .child(Text::new(prefix).color(Color::BrightWhite).into_element())
                    .child(Text::new(*line).color(Color::White).into_element())
                    .into_element(),
            );
        }
        container.into_element()
    }
}

/// Render thinking block (collapsed)
fn render_thinking(text: &str) -> Element {
    let lines: Vec<&str> = text.lines().take(5).collect();
    let has_more = text.lines().count() > 5;

    let mut container = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new("∴ Thinking…")
                .color(Color::Magenta)
                .dim()
                .italic()
                .into_element(),
        );

    for line in lines {
        container = container.child(
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("  ").into_element())
                .child(
                    Text::new(line)
                        .color(Color::Magenta)
                        .dim()
                        .italic()
                        .into_element(),
                )
                .into_element(),
        );
    }

    if has_more {
        container = container.child(
            Text::new("  ...")
                .color(Color::Ansi256(245))
                .dim()
                .into_element(),
        );
    }

    container.into_element()
}

/// Render tool call
fn render_tool_call(name: &str, args: Option<&str>) -> Element {
    let mut row = RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("● ").color(Color::Magenta).into_element())
        .child(
            Text::new(name)
                .color(Color::Magenta)
                .bold()
                .into_element(),
        );

    if let Some(args) = args {
        // Truncate long args
        let display_args = if args.len() > 50 {
            format!("{}...", &args[..47])
        } else {
            args.to_string()
        };
        row = row.child(
            Text::new(format!("(\"{}\")", display_args))
                .color(Color::Magenta)
                .dim()
                .into_element(),
        );
    }

    row.into_element()
}

/// Render tool result
fn render_tool_result(result: &str, success: bool) -> Element {
    let color = if success {
        Color::Ansi256(245)
    } else {
        Color::Red
    };
    // Truncate long output
    let display = if result.len() > 100 {
        format!("{}...", &result[..97])
    } else {
        result.to_string()
    };

    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("  ⎿ ").color(color).into_element())
        .child(Text::new(display).color(color).into_element())
        .into_element()
}

/// Render error message
fn render_error(message: &str) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("● Error: ").color(Color::Red).into_element())
        .child(Text::new(message).color(Color::Red).into_element())
        .into_element()
}

/// Render goodbye message
fn render_goodbye() -> Element {
    Text::new("Goodbye!").dim().into_element()
}

/// Render cancelled message
fn render_cancelled() -> Element {
    Text::new("⦻ Cancelled").dim().into_element()
}

// ============================================================================
// Output Helpers
// ============================================================================

/// Print rnk element to stdout (with newline)
fn print_element(element: &Element) {
    let output = rnk::render_to_string_auto(element);
    println!("{}", output);
}

/// Print rnk element to stdout (without newline, for inline prompts)
fn print_element_inline(element: &Element) {
    let output = rnk::render_to_string_auto(element);
    print!("{}", output);
}

// ============================================================================
// Spinner Animation
// ============================================================================

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
                    "\x1b[2K\r\x1b[35m{} {} \x1b[2m(ESC to cancel)\x1b[0m",
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

// ============================================================================
// Input Handling
// ============================================================================

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
                        std::process::exit(0);
                    }
                    KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
                        terminal::disable_raw_mode()?;
                        std::process::exit(0);
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

// ============================================================================
// Agent Integration
// ============================================================================

/// Create and configure UnifiedExecutor
async fn create_executor() -> SageResult<UnifiedExecutor> {
    let config = load_config()?;
    let working_dir = std::env::current_dir().unwrap_or_default();
    let mode = ExecutionMode::interactive();
    let options = ExecutionOptions::default()
        .with_mode(mode)
        .with_working_directory(&working_dir);

    let mut executor = UnifiedExecutor::with_options(config, options)?;

    // Use Streaming output mode - real-time display with animated thinking indicator
    executor.set_output_mode(OutputMode::Streaming);

    // Register default tools
    executor.register_tools(get_default_tools());

    // Initialize sub-agent support
    let _ = executor.init_subagent_support();

    Ok(executor)
}

// ============================================================================
// Main Application
// ============================================================================

/// Run the Sage CLI application
pub async fn run_app() -> io::Result<()> {
    // Print banner
    println!();
    print_element(&render_banner());
    println!();

    // Create executor
    let mut executor = create_executor().await.map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("Init error: {}", e))
    })?;

    // Set up input channel for interactive mode
    let (input_channel, _input_handle) = InputChannel::new(16);
    executor.set_input_channel(input_channel);

    // Main loop
    loop {
        // Print prompt (inline so user types on same line)
        print_element_inline(&render_prompt());
        io::stdout().flush()?;

        // Read user input
        let input = read_line_with_cjk()?;
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
                print_element(&render_banner());
                println!();
                continue;
            }
            "" => continue,
            _ => {}
        }

        // Clear the line and reprint with formatted user message
        print!("\x1b[1A\x1b[2K");
        print_element(&render_user_message(input));

        // Create task
        let working_dir = std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let task = TaskMetadata::new(input, &working_dir);

        // Execute task directly - StreamingOutput handles all display
        // No spinner here - it would conflict with StreamingOutput
        match executor.execute(task).await {
            Ok(_outcome) => {
                // Output is already complete via StreamingOutput
            }
            Err(e) => {
                println!();
                print_element(&render_error(&e.to_string()));
            }
        }

        println!();
    }

    Ok(())
}

/// Demo mode for testing UI components
pub fn run_demo() -> io::Result<()> {
    // Print banner
    println!();
    print_element(&render_banner());
    println!();

    // Demo user message
    print_element(&render_user_message("Help me refactor the UI code"));
    println!();

    // Demo thinking spinner
    let spinner = Spinner::new("Thinking...");
    std::thread::sleep(Duration::from_secs(2));
    spinner.stop();

    // Demo thinking block
    print_element(&render_thinking(
        "Analyzing the code structure...\nLooking for patterns...\nIdentifying opportunities...",
    ));
    println!();

    // Demo assistant response
    print_element(&render_assistant_response(
        "I'll help you refactor the UI code. Let me analyze the structure.",
    ));
    println!();

    // Demo tool call
    print_element(&render_tool_call("Read", Some("src/app.rs")));
    print_element(&render_tool_result("Read 150 lines", true));
    println!();

    // Demo error
    print_element(&render_error("Something went wrong"));
    println!();

    // Demo final prompt
    print_element_inline(&render_prompt());
    io::stdout().flush()?;

    // Wait a bit then show goodbye
    std::thread::sleep(Duration::from_secs(1));
    println!();
    print_element(&render_goodbye());

    Ok(())
}
