//! Debug version of fixed_bottom_demo with extensive logging
//!
//! Run with: cargo run --example fixed_bottom_debug 2> /tmp/debug.log
//! Then check /tmp/debug.log for render analysis

use rnk::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static RENDER_COUNT: AtomicUsize = AtomicUsize::new(0);

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
    id: usize,
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
    let msg_counter = use_signal(|| 0usize);

    // Get terminal size
    let (term_width, term_height) = crossterm::terminal::size().unwrap_or((80, 24));
    let viewport_height = term_height.saturating_sub(5) as usize; // Reserve 5 lines for bottom area

    // Update scroll viewport
    let msg_count = messages.get().len();
    scroll.set_content_size(term_width as usize, msg_count * 2);
    scroll.set_viewport_size(term_width as usize, viewport_height);

    // Increment render counter and log
    let frame = RENDER_COUNT.fetch_add(1, Ordering::SeqCst);
    eprintln!("\n=== RENDER FRAME {} ===", frame);
    eprintln!("  Terminal: {}x{}", term_width, term_height);
    eprintln!("  Viewport height: {}", viewport_height);
    eprintln!("  Message count: {}", msg_count);
    eprintln!("  Scroll offset Y: {}", scroll.offset_y());
    eprintln!("  Can scroll up: {}, down: {}", scroll.can_scroll_up(), scroll.can_scroll_down());

    // Handle keyboard input
    use_input({
        let input = input.clone();
        let messages = messages.clone();
        let mode = mode.clone();
        let msg_counter = msg_counter.clone();
        let app_ctx = app_ctx.clone();
        let scroll = scroll.clone();

        move |ch, key| {
            // Ctrl+C to quit
            if key.ctrl && ch == "c" {
                eprintln!("[INPUT] Ctrl+C - exiting");
                app_ctx.exit();
                return;
            }

            // Shift+Tab to toggle permission mode
            if key.tab && key.shift {
                eprintln!("[INPUT] Shift+Tab - cycling mode");
                mode.update(|m| {
                    *m = m.next();
                });
                return;
            }

            // Arrow keys for scrolling
            if key.up_arrow {
                eprintln!("[INPUT] Up arrow - scroll_up(1)");
                scroll.scroll_up(1);
                return;
            }
            if key.down_arrow {
                eprintln!("[INPUT] Down arrow - scroll_down(1)");
                scroll.scroll_down(1);
                return;
            }
            if key.page_up {
                eprintln!("[INPUT] Page Up");
                scroll.page_up();
                return;
            }
            if key.page_down {
                eprintln!("[INPUT] Page Down");
                scroll.page_down();
                return;
            }

            // Enter to submit
            if key.return_key {
                let current_input = input.get();
                if !current_input.is_empty() {
                    let counter = msg_counter.get();
                    eprintln!("[INPUT] Enter - submitting message #{}", counter);
                    messages.update(|msgs| {
                        msgs.push(ChatMessage {
                            id: counter * 2,
                            role: "user".to_string(),
                            content: current_input.clone(),
                        });
                        msgs.push(ChatMessage {
                            id: counter * 2 + 1,
                            role: "assistant".to_string(),
                            content: format!("Echo #{}: {}", counter, current_input),
                        });
                    });
                    msg_counter.set(counter + 1);
                    input.set(String::new());
                    scroll.scroll_to_bottom();
                    eprintln!("[INPUT] After submit - scroll_to_bottom called");
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

    eprintln!("  Building UI tree...");

    // Build the fixed-bottom layout using flexbox
    let content_area = if current_messages.is_empty() {
        eprintln!("    Content: Empty state");
        Text::new("Type something and press Enter... (Ctrl+C to quit)")
            .dim()
            .into_element()
    } else {
        let visible_count = viewport_height;
        let msgs: Vec<_> = current_messages.iter().rev().take(visible_count).collect();
        eprintln!("    Content: {} messages (showing last {})", current_messages.len(), msgs.len());

        let mut msg_box = Box::new().flex_direction(FlexDirection::Column);
        for msg in msgs.into_iter().rev() {
            eprintln!("      - Message #{}: {} '{}'", msg.id, msg.role, &msg.content[..msg.content.len().min(30)]);
            msg_box = msg_box.child(render_message(msg));
        }
        msg_box.into_element()
    };

    eprintln!("    Bottom area: separator + input + separator + status");

    // IMPORTANT: Using explicit pixel height instead of Dimension::Percent(100.0)
    // because inline mode tracking relies on consistent line counts between frames.
    eprintln!("    Root layout: {}x{} (explicit)", term_width, term_height);

    let root = Box::new()
        .flex_direction(FlexDirection::Column)
        .width(term_width as i32)
        .height(term_height as i32) // Explicit height, not Percent
        // Content area
        .child(
            Box::new()
                .flex_grow(1.0)
                .flex_direction(FlexDirection::Column)
                .overflow_y(Overflow::Hidden)
                .child(content_area)
                .into_element(),
        )
        // Separator 1
        .child(render_separator(term_width, 1))
        // Input line
        .child(render_input_line(&current_input))
        // Separator 2
        .child(render_separator(term_width, 2))
        // Status bar
        .child(render_status_bar(current_mode, &scroll))
        .into_element();

    eprintln!("  UI tree built");
    eprintln!("=== END FRAME {} ===\n", frame);

    root
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

/// Render separator line with ID for debugging
fn render_separator(width: u16, id: u8) -> Element {
    // Add unique marker to identify which separator is which
    let marker = format!("[S{}]", id);
    let line_width = (width as usize).saturating_sub(marker.len());
    let line = format!("{}{}", "─".repeat(line_width), marker);
    Text::new(line).dim().into_element()
}

/// Render input line with prompt
fn render_input_line(input: &str) -> Element {
    Box::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("❯ ").color(Color::Yellow).bold().into_element())
        .child(Text::new(input).into_element())
        .child(Text::new("█").color(Color::Yellow).into_element())
        .into_element()
}

/// Render status bar
fn render_status_bar(mode: PermissionMode, scroll: &ScrollHandle) -> Element {
    let mode_text = mode.display_text();

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
        .child(Text::new(" (shift+tab)").dim().into_element())
        .child(Text::new(scroll_indicator).dim().into_element())
        .into_element()
}

fn main() -> std::io::Result<()> {
    eprintln!("=== FIXED BOTTOM DEBUG START ===");
    eprintln!("Terminal size: {:?}", crossterm::terminal::size());
    eprintln!("TERM_PROGRAM: {:?}", std::env::var("TERM_PROGRAM").ok());
    eprintln!("");
    eprintln!("Instructions:");
    eprintln!("  1. Type text and press Enter to add messages");
    eprintln!("  2. Use Up/Down arrows to scroll");
    eprintln!("  3. Check /tmp/debug.log for render analysis");
    eprintln!("  4. Ctrl+C to quit");
    eprintln!("");

    // Run in inline mode
    render(app).run()
}
