//! Demo: Fixed bottom input area like Claude Code
//!
//! Uses inline mode with flexbox layout for fixed-bottom behavior.
//! The content area uses flex_grow: 1 to take remaining space,
//! pushing the bottom elements (input + status) to the bottom.
//!
//! Run with: cargo run --example fixed_bottom_demo

use rnk::core::Dimension;
use rnk::prelude::*;

/// Permission mode
#[derive(Clone, Copy, PartialEq)]
enum PermissionMode {
    Normal,
    Bypass,
    Plan,
}

impl PermissionMode {
    fn next(self) -> Self {
        match self {
            PermissionMode::Normal => PermissionMode::Bypass,
            PermissionMode::Bypass => PermissionMode::Plan,
            PermissionMode::Plan => PermissionMode::Normal,
        }
    }

    fn display_text(self) -> &'static str {
        match self {
            PermissionMode::Normal => "permissions required",
            PermissionMode::Bypass => "bypass permissions on",
            PermissionMode::Plan => "plan mode",
        }
    }
}

/// Message in chat history
#[derive(Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

fn app() -> Element {
    let app_ctx = use_app();
    let scroll = use_scroll();

    // State: messages, input buffer, permission mode
    let messages = use_signal(|| Vec::<ChatMessage>::new());
    let input = use_signal(|| String::new());
    let mode = use_signal(|| PermissionMode::Normal);

    // Get terminal size
    let (term_width, term_height) = crossterm::terminal::size().unwrap_or((80, 24));
    let viewport_height = term_height.saturating_sub(3) as usize; // Reserve 3 lines for bottom

    // Update scroll viewport
    let msg_count = messages.get().len();
    scroll.set_content_size(term_width as usize, msg_count * 2); // ~2 lines per message
    scroll.set_viewport_size(term_width as usize, viewport_height);

    // Handle keyboard input
    use_input({
        let input = input.clone();
        let messages = messages.clone();
        let mode = mode.clone();
        let app_ctx = app_ctx.clone();
        let scroll = scroll.clone();

        move |ch, key| {
            // Ctrl+C to quit
            if key.ctrl && ch == "c" {
                app_ctx.exit();
                return;
            }

            // Shift+Tab to toggle permission mode
            if key.tab && key.shift {
                mode.update(|m| {
                    *m = m.next();
                });
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
                    // Auto-scroll to bottom on new message
                    scroll.scroll_to_bottom();
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

    let current_messages = messages.get();
    let current_input = input.get();
    let current_mode = mode.get();

    // Build the fixed-bottom layout using flexbox
    // Root: height 100% with column direction
    // Content area: flex_grow 1 (takes remaining space)
    // Bottom elements: fixed height (naturally pushed to bottom)
    Box::new()
        .flex_direction(FlexDirection::Column)
        .height(Dimension::Percent(100.0)) // Full terminal height
        // Content area - takes remaining space with flex_grow
        .child(
            Box::new()
                .flex_grow(1.0) // Take all remaining space
                .flex_direction(FlexDirection::Column)
                .overflow_y(Overflow::Hidden) // Clip overflow
                .child(if current_messages.is_empty() {
                    Text::new("Type something and press Enter... (Ctrl+C to quit)")
                        .dim()
                        .into_element()
                } else {
                    // Show messages (most recent at bottom)
                    let visible_count = (term_height.saturating_sub(4)) as usize;
                    let mut msg_box = Box::new().flex_direction(FlexDirection::Column);
                    let msgs: Vec<_> = current_messages.iter().rev().take(visible_count).collect();
                    for msg in msgs.into_iter().rev() {
                        msg_box = msg_box.child(render_message(msg));
                    }
                    msg_box.into_element()
                })
                .into_element(),
        )
        // Separator line 1 (above input)
        .child(render_separator(term_width))
        // Input line
        .child(render_input_line(&current_input))
        // Separator line 2 (above status bar)
        .child(render_separator(term_width))
        // Status bar
        .child(render_status_bar(current_mode, &scroll))
        .into_element()
}

/// Render a chat message
fn render_message(msg: &ChatMessage) -> Element {
    let (prefix, color) = if msg.role == "user" {
        ("❯ ", Color::Yellow)
    } else {
        ("● ", Color::BrightWhite)
    };

    Box::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new(prefix).color(color).bold().into_element())
        .child(Text::new(&msg.content).into_element())
        .into_element()
}

/// Render separator line
fn render_separator(width: u16) -> Element {
    let line = "─".repeat(width as usize);
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
fn render_status_bar(mode: PermissionMode, scroll: &ScrollHandle) -> Element {
    let mode_text = mode.display_text();

    // Show scroll indicator if content is scrollable
    let scroll_indicator = if scroll.can_scroll_up() || scroll.can_scroll_down() {
        let percent = (scroll.scroll_percent_y() * 100.0) as u8;
        format!(" [{:3}%]", percent)
    } else {
        String::new()
    };

    Box::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("▸▸ ").color(Color::Cyan).into_element())
        .child(Text::new(mode_text).dim().into_element())
        .child(Text::new(" (shift+tab to cycle)").dim().into_element())
        .child(Text::new(scroll_indicator).dim().into_element())
        .into_element()
}

fn main() -> std::io::Result<()> {
    // Run in inline mode
    render(app).run()
}
