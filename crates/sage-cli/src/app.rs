//! Sage CLI Main Application (streaming output mode)
//!
//! Uses rnk components for rendering but outputs to stdout directly (not fullscreen TUI)

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal,
};
use rnk::core::Style;
use rnk::layout::LayoutEngine;
use rnk::prelude::*;
use rnk::renderer::Output;
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

// ===== Rendering Helpers =====

/// Wrap text to fit within max_width columns
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0usize;

    for ch in text.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);

        if ch == '\n' {
            result.push(current_line.clone());
            current_line = String::new();
            current_width = 0;
        } else if current_width + ch_width > max_width {
            if current_line.is_empty() {
                current_line.push(ch);
                current_width = ch_width;
            } else {
                result.push(current_line.clone());
                current_line = ch.to_string();
                current_width = ch_width;
            }
        } else {
            current_line.push(ch);
            current_width += ch_width;
        }
    }

    if !current_line.is_empty() {
        result.push(current_line);
    }

    if result.is_empty() {
        result.push(String::new());
    }

    result
}

/// Calculate the actual height needed for an element
fn calculate_element_height(element: &Element, max_width: u16) -> u16 {
    let mut height = 1u16;
    let available_width = if element.style.has_border() {
        max_width.saturating_sub(2)
    } else {
        max_width
    };
    let padding_h = (element.style.padding.left + element.style.padding.right) as u16;
    let available_width = available_width.saturating_sub(padding_h);

    if let Some(lines) = &element.spans {
        let mut total_lines = 0usize;
        for line in lines {
            let line_text: String = line.spans.iter().map(|s| s.content.as_str()).collect();
            let wrapped = wrap_text(&line_text, available_width as usize);
            total_lines += wrapped.len();
        }
        height = height.max(total_lines as u16);
    }

    if let Some(text) = &element.text_content {
        let wrapped = wrap_text(text, available_width as usize);
        height = height.max(wrapped.len() as u16);
    }

    let mut child_height_sum = 0u16;
    for child in &element.children {
        let child_height = calculate_element_height(child, max_width);
        child_height_sum += child_height;
    }

    if !element.children.is_empty() {
        height = height.max(child_height_sum);
    }

    height
}

/// Render a single text span at position
fn render_text_span(
    output: &mut Output,
    text: &str,
    x: u16,
    y: u16,
    max_width: u16,
    style: &Style,
) {
    let wrapped_lines = wrap_text(text, max_width as usize);
    for (i, line) in wrapped_lines.iter().enumerate() {
        output.write(x, y + i as u16, line, style);
    }
}

/// Recursively render element tree
fn render_element_recursive(
    element: &Element,
    engine: &LayoutEngine,
    output: &mut Output,
    offset_x: f32,
    offset_y: f32,
    container_width: u16,
) {
    if element.style.display == Display::None {
        return;
    }

    let layout = match engine.get_layout(element.id) {
        Some(l) => l,
        None => return,
    };

    let x = (offset_x + layout.x) as u16;
    let y = (offset_y + layout.y) as u16;
    let w = layout.width as u16;
    let h = layout.height as u16;

    // Background
    if element.style.background_color.is_some() {
        for row in 0..h {
            output.write(x, y + row, &" ".repeat(w as usize), &element.style);
        }
    }

    // Border
    if element.style.has_border() {
        let (tl, tr, bl, br, hz, vt) = element.style.border_style.chars();
        let mut style = element.style.clone();

        style.color = element.style.get_border_top_color();
        output.write(
            x,
            y,
            &format!("{}{}{}", tl, hz.repeat((w as usize).saturating_sub(2)), tr),
            &style,
        );

        style.color = element.style.get_border_bottom_color();
        output.write(
            x,
            y + h.saturating_sub(1),
            &format!("{}{}{}", bl, hz.repeat((w as usize).saturating_sub(2)), br),
            &style,
        );

        for row in 1..h.saturating_sub(1) {
            style.color = element.style.get_border_left_color();
            output.write(x, y + row, vt, &style);
            style.color = element.style.get_border_right_color();
            output.write(x + w.saturating_sub(1), y + row, vt, &style);
        }
    }

    let inner_x =
        x + if element.style.has_border() { 1 } else { 0 } + element.style.padding.left as u16;
    let inner_y =
        y + if element.style.has_border() { 1 } else { 0 } + element.style.padding.top as u16;
    let padding_h = (element.style.padding.left + element.style.padding.right) as u16;
    let inner_width = w.saturating_sub(if element.style.has_border() { 2 } else { 0 } + padding_h);

    if let Some(text) = &element.text_content {
        render_text_span(output, text, inner_x, inner_y, inner_width, &element.style);
    } else if let Some(lines) = &element.spans {
        let mut line_offset = 0u16;
        for line in lines {
            let line_text: String = line.spans.iter().map(|s| s.content.as_str()).collect();
            let wrapped = wrap_text(&line_text, inner_width as usize);

            for (wrapped_idx, wrapped_line) in wrapped.iter().enumerate() {
                let span_style = if !line.spans.is_empty() {
                    let span = &line.spans[0];
                    let mut style = element.style.clone();
                    if span.style.color.is_some() {
                        style.color = span.style.color;
                    }
                    if span.style.background_color.is_some() {
                        style.background_color = span.style.background_color;
                    }
                    if span.style.bold {
                        style.bold = true;
                    }
                    if span.style.italic {
                        style.italic = true;
                    }
                    if span.style.dim {
                        style.dim = true;
                    }
                    if span.style.underline {
                        style.underline = true;
                    }
                    style
                } else {
                    element.style.clone()
                };

                output.write(
                    inner_x,
                    inner_y + line_offset + wrapped_idx as u16,
                    wrapped_line,
                    &span_style,
                );
            }
            line_offset += wrapped.len() as u16;
        }
    }

    for child in element.children.iter() {
        render_element_recursive(child, engine, output, x as f32, y as f32, container_width);
    }
}

/// Main render-to-string function
fn render_to_string(element: &Element, width: u16) -> String {
    let mut engine = LayoutEngine::new();
    engine.compute(element, width, 100);

    let height = calculate_element_height(element, width);

    let mut output = Output::new(width, height);
    render_element_recursive(element, &engine, &mut output, 0.0, 0.0, width);
    output.render()
}

/// Print rnk element to stdout
fn print_element(element: &Element) {
    let (width, _) = crossterm::terminal::size().unwrap_or((80, 24));
    let output = render_to_string(element, width);
    println!("{}", output);
}

/// Print rnk element inline (no newline)
fn print_element_inline(element: &Element) {
    let (width, _) = crossterm::terminal::size().unwrap_or((80, 24));
    let output = render_to_string(element, width);
    // Trim trailing spaces from each line to avoid cursor drift
    let trimmed: String = output
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    print!("{}", trimmed);
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

fn render_prompt() -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("> ").color(Color::Yellow).bold().into_element())
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
        print_element(&render_thinking());
    }

    fn print_thinking_stop() {
        // Clear the thinking line
        print!("\x1b[1A\x1b[2K");
    }

    fn print_tool_call(name: &str) {
        print_element(&render_tool_call(name));
    }

    fn print_error(message: &str) {
        print_element(&render_error(message));
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

    // Use Rnk output mode (though we're not using the bridge anymore)
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

    // Print banner
    println!();
    print_element(&render_banner());
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
                                        ExecutionOutcome::Success(exec) => exec.final_result,
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
        print_element_inline(&render_prompt());
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
        print_element(&render_user_message(input));

        let _ = action_tx.send(UserAction::Submit(input.to_string()));
    }

    Ok(())
}

/// Demo mode for testing UI
pub fn run_demo() -> std::io::Result<()> {
    Icons::init_from_env();

    println!();
    print_element(&render_banner());
    println!();

    print_element(&render_user_message("Help me refactor the UI code"));
    println!();

    print_element(&render_thinking());
    std::thread::sleep(Duration::from_secs(1));
    print!("\x1b[1A\x1b[2K");

    EventPrinter::print_assistant_response(
        "I'll help you refactor the UI code. Let me analyze the structure first.",
    );
    println!();

    print_element(&render_tool_call("read_file"));
    println!();

    print_element_inline(&render_prompt());
    io::stdout().flush()?;

    Ok(())
}
