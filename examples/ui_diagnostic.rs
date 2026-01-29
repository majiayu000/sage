//! Diagnostic tool for UI rendering issues
//! Run with: cargo run --example ui_diagnostic

use crossterm::{
    terminal::{self, ClearType},
    cursor, execute,
    event::{self, Event, KeyCode},
};
use rnk::prelude::*;
use rnk::prelude::Box as RnkBox;
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

fn main() -> io::Result<()> {
    let mut stdout = io::stdout();

    // Get terminal size BEFORE entering alternate screen
    let (term_width, term_height) = terminal::size().unwrap_or((80, 24));

    println!("=== UI Diagnostic Tool ===");
    println!("Terminal size detected: {}x{}", term_width, term_height);
    println!("\nPress ENTER to start fullscreen test...");

    // Wait for enter
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    // Enter alternate screen
    execute!(stdout, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    // Build UI exactly like rnk_app.rs
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

    let root = RnkBox::new()
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
        .into_element();

    // Clear and render
    execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;

    let output = rnk::render_to_string(&root, term_width);
    print!("{}", output);
    stdout.flush()?;

    // Wait for key
    loop {
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Char('d') => {
                    // Debug mode - print layout info to file
                    let debug_output = format!(
                        "Terminal: {}x{}\n\n=== Raw Output ===\n{}\n\n=== Line Analysis ===\n{}",
                        term_width, term_height,
                        output,
                        output.lines().enumerate().map(|(i, line)| {
                            let stripped = strip_ansi(line);
                            let leading = stripped.len() - stripped.trim_start().len();
                            format!("Line {:3}: col={:3} |{}|\n", i, leading, stripped)
                        }).collect::<String>()
                    );
                    std::fs::write("/tmp/ui_debug.txt", debug_output)?;
                }
                _ => {}
            }
        }
    }

    // Cleanup
    terminal::disable_raw_mode()?;
    execute!(stdout, terminal::LeaveAlternateScreen)?;

    println!("\n=== Diagnostic Complete ===");
    println!("If the UI appeared correctly (left-aligned), the issue may be:");
    println!("1. A different code path in sage");
    println!("2. Terminal-specific rendering");
    println!("3. State-dependent rendering");
    println!("\nPress 'd' during the test to dump debug info to /tmp/ui_debug.txt");

    Ok(())
}
