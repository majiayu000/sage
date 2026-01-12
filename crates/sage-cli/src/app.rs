//! Sage CLI Main Application (streaming output mode with rnk 0.4.0)
//!
//! Uses rnk::println() with Message components for clean output

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal,
};
use rnk::prelude::*;
use rnk::components::Message;
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

// UI components are now provided by rnk's Message component

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

/// Simple event printer for Agent events using rnk components
struct EventPrinter;

impl EventPrinter {
    fn print_thinking_start() {
        // Print thinking message using rnk
        rnk::println(Message::tool("Thinking...").into_element());
    }

    fn print_thinking_stop() {
        // Clear the thinking line
        print!("\x1b[1A\x1b[2K");
        io::stdout().flush().ok();
    }

    fn print_tool_call(name: &str) {
        rnk::println(Message::tool(name).into_element());
    }

    fn print_error(message: &str) {
        rnk::println(Message::error(message).into_element());
    }

    fn print_assistant_response(text: &str) {
        rnk::println(Message::assistant(text).into_element());
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
                                            exec.final_result
                                        }
                                        ExecutionOutcome::NeedsUserInput { last_response, .. } => {
                                            Some(last_response)
                                        }
                                        ExecutionOutcome::Failed { error, .. } => {
                                            Some(format!("Error: {}", error.message))
                                        }
                                        ExecutionOutcome::MaxStepsReached { .. } => {
                                            Some("Max steps reached".to_string())
                                        }
                                        ExecutionOutcome::Interrupted { .. } => {
                                            Some("Interrupted".to_string())
                                        }
                                        ExecutionOutcome::UserCancelled { .. } => {
                                            Some("Cancelled".to_string())
                                        }
                                    };

                                    if let Some(response_text) = response {
                                        EventPrinter::print_assistant_response(&response_text);
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
        rnk::println(Message::user(input).into_element());

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

    rnk::println(Message::user("Help me refactor the UI code").into_element());
    println!();

    rnk::println(Message::tool("Thinking...").into_element());
    std::thread::sleep(Duration::from_secs(1));
    print!("\x1b[1A\x1b[2K");

    EventPrinter::print_assistant_response(
        "I'll help you refactor the UI code. Let me analyze the structure first.",
    );
    println!();

    rnk::println(Message::tool("read_file").into_element());
    println!();

    print!("\x1b[33m\x1b[1m> \x1b[0m");
    io::stdout().flush()?;

    Ok(())
}
