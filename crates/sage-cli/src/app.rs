//! Sage CLI Main Application (streaming output mode with rnk 0.4.0)
//!
//! Uses rnk::println() to print Element directly

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal,
};
use rnk::prelude::*;
use sage_core::agent::{ExecutionMode, ExecutionOptions, ExecutionOutcome, UnifiedExecutor};
use sage_core::config::load_config;
use sage_core::error::SageResult;
use sage_core::input::InputChannel;
use sage_core::output::OutputMode;
use sage_core::types::TaskMetadata;
use sage_core::ui::theme::Icons;
use sage_tools::get_default_tools;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use unicode_width::UnicodeWidthChar;

// Alias rnk's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as RnkBox;

/// User action from keyboard input
#[derive(Debug, Clone)]
pub enum UserAction {
    Submit(String),
    Exit,
    Cancel,
}

// ===== UI Components =====

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

fn render_user_message(text: &str) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("> ").color(Color::Yellow).bold().into_element())
        .child(Text::new(text).color(Color::BrightWhite).into_element())
        .into_element()
}

fn render_thinking() -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("● ").color(Color::Magenta).into_element())
        .child(
            Text::new("Thinking...")
                .color(Color::Magenta)
                .into_element(),
        )
        .into_element()
}

fn render_tool_call(name: &str) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("● ").color(Color::Magenta).into_element())
        .child(Text::new(name).color(Color::Magenta).bold().into_element())
        .into_element()
}

fn render_error(message: &str) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("● ").color(Color::Red).into_element())
        .child(Text::new(message).color(Color::Red).into_element())
        .into_element()
}

// ===== Input Handling =====

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

// ===== Agent Integration =====

/// Simple event printer for Agent events
struct EventPrinter;

impl EventPrinter {
    fn print_thinking_start() {
        // Use simple println instead of rnk::println to avoid layout issues
        println!("\x1b[35m● Thinking...\x1b[0m");
    }

    fn print_thinking_stop() {
        // Clear the thinking line
        print!("\x1b[1A\x1b[2K");
        io::stdout().flush().ok();
    }

    fn print_tool_call(name: &str) {
        println!("\x1b[35m● {}\x1b[0m", name);
    }

    fn print_error(message: &str) {
        println!("\x1b[31m● {}\x1b[0m", message);
    }

    fn print_assistant_response(text: &str) {
        println!("\x1b[97m● {}\x1b[0m", text);
    }
}

/// Create and configure UnifiedExecutor
async fn create_executor() -> SageResult<UnifiedExecutor> {
    let config = load_config()?;

    let working_dir = std::env::current_dir().unwrap_or_default();

    let mode = ExecutionMode::interactive();
    let options = ExecutionOptions::default()
        .with_mode(mode)
        .with_working_directory(&working_dir);

    let mut executor = UnifiedExecutor::with_options(config, options)?;

    // Use Rnk output mode
    executor.set_output_mode(OutputMode::Rnk);

    // Register default tools
    executor.register_tools(get_default_tools());

    // Initialize sub-agent support
    let _ = executor.init_subagent_support();

    Ok(executor)
}

/// Run the Sage CLI application with streaming output
pub fn run_app() -> std::io::Result<()> {
    Icons::init_from_env();

    // Print banner using rnk::println
    println!();
    rnk::println(render_banner());
    println!();

    // Create action channel
    let (action_tx, mut action_rx) = mpsc::unbounded_channel::<UserAction>();

    // Spawn Agent executor in background thread
    let executor_handle = Arc::new(Mutex::new(None::<UnifiedExecutor>));
    let executor_handle_clone = Arc::clone(&executor_handle);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Create executor
            let executor_result = create_executor().await;
            let mut executor = match executor_result {
                Ok(e) => e,
                Err(e) => {
                    EventPrinter::print_error(&format!("Init error: {}", e));
                    return;
                }
            };

            // Set up input channel
            let (input_channel, _input_handle) = InputChannel::new(16);
            executor.set_input_channel(input_channel);

            *executor_handle_clone.lock().unwrap() = Some(executor);

            // Process user actions
            while let Some(action) = action_rx.recv().await {
                match action {
                    UserAction::Submit(text) => {
                        if text == "/exit" || text == "/quit" {
                            std::process::exit(0);
                        }

                        EventPrinter::print_thinking_start();

                        let working_dir = std::env::current_dir()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let task = TaskMetadata::new(&text, &working_dir);

                        let mut exec_guard = executor_handle_clone.lock().unwrap();
                        if let Some(ref mut executor) = *exec_guard {
                            match executor.execute(task).await {
                                Ok(outcome) => {
                                    EventPrinter::print_thinking_stop();

                                    let response = match outcome {
                                        ExecutionOutcome::Success(exec) => {
                                            // Debug: print outcome type
                                            eprintln!("[DEBUG] Success outcome, final_result: {:?}", exec.final_result.as_ref().map(|s| &s[..s.len().min(50)]));
                                            exec.final_result
                                        }
                                        ExecutionOutcome::NeedsUserInput { last_response, .. } => {
                                            eprintln!("[DEBUG] NeedsUserInput outcome");
                                            Some(last_response)
                                        }
                                        ExecutionOutcome::Failed { error, .. } => {
                                            eprintln!("[DEBUG] Failed outcome: {}", error.message);
                                            Some(format!("Error: {}", error.message))
                                        }
                                        ExecutionOutcome::MaxStepsReached { .. } => {
                                            eprintln!("[DEBUG] MaxStepsReached outcome");
                                            Some("Max steps reached".to_string())
                                        }
                                        ExecutionOutcome::Interrupted { .. } => {
                                            eprintln!("[DEBUG] Interrupted outcome");
                                            Some("Interrupted".to_string())
                                        }
                                        ExecutionOutcome::UserCancelled { .. } => {
                                            eprintln!("[DEBUG] UserCancelled outcome");
                                            Some("Cancelled".to_string())
                                        }
                                    };

                                    if let Some(response_text) = response {
                                        eprintln!("[DEBUG] Printing response: {}", &response_text[..response_text.len().min(100)]);
                                        EventPrinter::print_assistant_response(&response_text);
                                    } else {
                                        eprintln!("[DEBUG] No response to print (response is None)");
                                    }
                                }
                                Err(e) => {
                                    EventPrinter::print_thinking_stop();
                                    EventPrinter::print_error(&format!("Execution error: {}", e));
                                }
                            }
                        }

                        println!();
                    }
                    UserAction::Exit => {
                        std::process::exit(0);
                    }
                    UserAction::Cancel => {
                        EventPrinter::print_thinking_stop();
                        println!();
                    }
                }
            }
        });
    });

    // Main input loop
    loop {
        // Print prompt (simple text, no need for Element)
        print!("\x1b[33m\x1b[1m> \x1b[0m");
        io::stdout().flush()?;

        let input = read_line_with_cjk()?;
        let input = input.trim();

        match input.to_lowercase().as_str() {
            "quit" | "exit" => {
                println!();
                break;
            }
            "" => continue,
            _ => {}
        }

        // Clear the line and reprint with formatting
        print!("\x1b[1A\x1b[2K");
        rnk::println(render_user_message(input));

        let _ = action_tx.send(UserAction::Submit(input.to_string()));
    }

    Ok(())
}

/// Demo mode for testing UI
pub fn run_demo() -> std::io::Result<()> {
    Icons::init_from_env();

    println!();
    rnk::println(render_banner());
    println!();

    rnk::println(render_user_message("Help me refactor the UI code"));
    println!();

    rnk::println(render_thinking());
    std::thread::sleep(Duration::from_secs(1));
    print!("\x1b[1A\x1b[2K");

    EventPrinter::print_assistant_response(
        "I'll help you refactor the UI code. Let me analyze the structure first.",
    );
    println!();

    rnk::println(render_tool_call("read_file"));
    println!();

    print!("\x1b[33m\x1b[1m> \x1b[0m");
    io::stdout().flush()?;

    Ok(())
}
