//! Demo: Fixed bottom input area like Claude Code
//!
//! Run with: cargo run --example fixed_bottom_demo

use rnk::prelude::*;

/// Permission mode
#[derive(Clone, Copy, PartialEq)]
enum PermissionMode {
    Normal,
    Bypass,
}

/// Message in chat history
#[derive(Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

fn app() -> Element {
    let app_ctx = use_app();

    // State: messages, input buffer, permission mode
    let messages = use_signal(|| Vec::<ChatMessage>::new());
    let input = use_signal(|| String::new());
    let mode = use_signal(|| PermissionMode::Normal);

    // Handle keyboard input
    use_input({
        let input = input.clone();
        let messages = messages.clone();
        let mode = mode.clone();
        let app_ctx = app_ctx.clone();

        move |ch, key| {
            // Ctrl+C to quit
            if key.ctrl && ch == "c" {
                app_ctx.exit();
                return;
            }

            // Shift+Tab to toggle permission mode
            if key.tab && key.shift {
                mode.update(|m| {
                    *m = if *m == PermissionMode::Normal {
                        PermissionMode::Bypass
                    } else {
                        PermissionMode::Normal
                    };
                });
                return;
            }

            // Enter to submit
            if key.return_key {
                let current_input = input.get();
                if !current_input.is_empty() {
                    messages.update(|msgs| {
                        msgs.push(ChatMessage {
                            role: "user".to_string(),
                            content: current_input.clone(),
                        });
                        // Simulate assistant response
                        msgs.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: format!("Echo: {}", current_input),
                        });
                    });
                    input.set(String::new());
                }
                return;
            }

            // Backspace
            if key.backspace {
                input.update(|s| {
                    s.pop();
                });
                return;
            }

            // Regular character input
            if !ch.is_empty() && !key.ctrl && !key.alt {
                input.update(|s| {
                    s.push_str(ch);
                });
            }
        }
    });

    // Get terminal height for layout
    let (_, term_height) = crossterm::terminal::size().unwrap_or((80, 24));
    let content_height = term_height.saturating_sub(4) as u16;

    let current_messages = messages.get();
    let current_input = input.get();
    let current_mode = mode.get();

    // Build the layout
    Box::new()
        .flex_direction(FlexDirection::Column)
        .child(render_content_area(&current_messages, content_height))
        .child(render_separator())
        .child(render_input_line(&current_input))
        .child(render_status_bar(current_mode))
        .into_element()
}

/// Render scrollable content area
fn render_content_area(messages: &[ChatMessage], max_lines: u16) -> Element {
    let mut container = Box::new()
        .flex_direction(FlexDirection::Column)
        .min_height(max_lines);

    // Show last N messages that fit
    let max = max_lines as usize / 2;
    let start = messages.len().saturating_sub(max);
    for msg in messages.iter().skip(start) {
        let (prefix, color) = if msg.role == "user" {
            ("❯ ", Color::Yellow)
        } else {
            ("● ", Color::BrightWhite)
        };

        container = container.child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new(prefix).color(color).bold().into_element())
                .child(Text::new(&msg.content).into_element())
                .into_element(),
        );
    }

    // Fill remaining space if needed
    if messages.is_empty() {
        container = container.child(
            Text::new("Type something and press Enter... (Ctrl+C to quit)")
                .dim()
                .into_element(),
        );
    }

    container.into_element()
}

/// Render separator line
fn render_separator() -> Element {
    let (term_width, _) = crossterm::terminal::size().unwrap_or((80, 24));
    let line = "─".repeat(term_width as usize);
    Text::new(line).dim().into_element()
}

/// Render input line with prompt
fn render_input_line(input: &str) -> Element {
    Box::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("❯ ").color(Color::Yellow).bold().into_element())
        .child(Text::new(input).into_element())
        .child(Text::new("█").color(Color::Yellow).into_element()) // Cursor
        .into_element()
}

/// Render status bar
fn render_status_bar(mode: PermissionMode) -> Element {
    let mode_text = match mode {
        PermissionMode::Normal => "permissions required",
        PermissionMode::Bypass => "bypass permissions on",
    };

    Box::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("▸▸ ").color(Color::Cyan).into_element())
        .child(Text::new(mode_text).dim().into_element())
        .child(Text::new(" (shift+tab to cycle)").dim().into_element())
        .into_element()
}

fn main() -> std::io::Result<()> {
    // Run in inline mode (preserves terminal history)
    render(app).run()
}
