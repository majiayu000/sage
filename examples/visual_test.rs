//! Visual diagnostic that runs in fullscreen mode
//! Run: cargo run --example visual_test

use crossterm::{
    terminal::{self, ClearType},
    cursor, execute,
    event::{self, Event, KeyCode},
};
use rnk::prelude::*;
use rnk::prelude::Box as RnkBox;
use rnk::layout::LayoutEngine;
use std::io::{self, Write};
use std::time::Duration;

fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn build_test_ui(term_width: u16, term_height: u16) -> Element {
    let welcome = RnkBox::new()
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
        .into_element();

    let separator = "─".repeat(term_width as usize);
    let bottom = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new(&separator).dim().into_element())
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("❯ ").color(Color::Yellow).bold().into_element())
                .child(Text::new("Type your message...").into_element())
                .into_element(),
        )
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("▸▸").into_element())
                .child(Text::new(" permissions required").dim().into_element())
                .child(Text::new(" (shift+tab to cycle)").dim().into_element())
                .into_element(),
        )
        .into_element();

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .width(term_width as i32)
        .height(term_height as i32)
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Column)
                .flex_grow(1.0)
                .child(welcome)
                .into_element(),
        )
        .child(bottom)
        .into_element()
}

fn main() -> io::Result<()> {
    let mut stdout = io::stdout();

    // Get size BEFORE alternate screen
    let (term_width, term_height) = terminal::size().unwrap_or((80, 24));

    // Build UI
    let root = build_test_ui(term_width, term_height);

    // Compute Taffy layout
    let mut engine = LayoutEngine::new();
    engine.compute(&root, term_width, term_height);

    // Render to string
    let output = rnk::render_to_string(&root, term_width);

    // === Pre-test diagnostics (before alternate screen) ===
    println!("=== Pre-Test Diagnostics ===");
    println!("Terminal: {}x{}", term_width, term_height);
    println!("TERM_PROGRAM: {:?}", std::env::var("TERM_PROGRAM").ok());
    println!();

    // Check Taffy layouts
    println!("=== Taffy Layout Summary ===");
    fn check_layout(element: &Element, engine: &LayoutEngine, path: &str) {
        if let Some(layout) = engine.get_layout(element.id) {
            if layout.x > 0.1 && element.children.is_empty() {
                // Only flag leaf nodes with x > 0 that aren't in Row layout
                println!("  {} x={:.1} (potential issue)", path, layout.x);
            }
        }
        for (i, child) in element.children.iter().enumerate() {
            check_layout(child, engine, &format!("{}/{}", path, i));
        }
    }
    check_layout(&root, &engine, "root");
    println!();

    // Check output alignment
    println!("=== Output Line Analysis ===");
    let mut issues_found = false;
    for (i, line) in output.lines().enumerate() {
        let stripped = strip_ansi(line);
        let leading = stripped.len() - stripped.trim_start().len();
        if leading > 0 && !stripped.trim().is_empty() {
            println!("  Line {}: {} leading spaces - '{}'",
                i, leading, stripped.chars().take(40).collect::<String>());
            issues_found = true;
        }
    }
    if !issues_found {
        println!("  All lines start at column 0 ✓");
    }
    println!();

    println!("Press ENTER to view fullscreen test (q/ESC to exit)...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    // === Fullscreen test ===
    execute!(stdout, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;

    print!("{}", output);
    stdout.flush()?;

    // Wait for exit
    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    break;
                }
            }
        }
    }

    // Cleanup
    terminal::disable_raw_mode()?;
    execute!(stdout, terminal::LeaveAlternateScreen)?;

    println!("\n=== Test Complete ===");
    println!("If the UI appeared misaligned:");
    println!("1. Please share the 'Pre-Test Diagnostics' output above");
    println!("2. Screenshot the fullscreen test");
    println!("3. Note your terminal application (e.g., WarpTerminal, iTerm2, etc.)");

    Ok(())
}
