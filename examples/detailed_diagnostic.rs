//! Detailed diagnostic with byte-level analysis
//! Run: cargo run --example detailed_diagnostic

use crossterm::terminal;
use rnk::prelude::*;
use rnk::prelude::Box as RnkBox;

fn main() {
    let (term_width, term_height) = terminal::size().unwrap_or((80, 24));

    println!("=== Detailed Diagnostic ===");
    println!("Terminal: {}x{}", term_width, term_height);
    println!("TERM_PROGRAM: {:?}", std::env::var("TERM_PROGRAM").ok());
    println!();

    // Test 1: Simple println behavior
    println!("=== Test 1: Plain println ===");
    println!("Line 1 - should be at column 0");
    println!("Line 2 - should be at column 0");
    println!("Line 3 - should be at column 0");
    println!();

    // Test 2: Print with explicit \r\n
    println!("=== Test 2: With explicit CR+LF ===");
    print!("Line A - with CR+LF\r\n");
    print!("Line B - with CR+LF\r\n");
    print!("Line C - with CR+LF\r\n");
    println!();

    // Test 3: Simple rnk element
    println!("=== Test 3: Simple rnk Text ===");
    let simple = Text::new("Simple text element").into_element();
    let output = rnk::render_to_string(&simple, term_width);
    println!("Raw output: {:?}", output);
    println!("Rendered:");
    print!("{}", output);
    println!();
    println!();

    // Test 4: Column layout
    println!("=== Test 4: Column layout ===");
    let column = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new("Column Line 1").into_element())
        .child(Text::new("Column Line 2").into_element())
        .child(Text::new("Column Line 3").into_element())
        .into_element();
    let output = rnk::render_to_string(&column, term_width);
    println!("Raw output bytes:");
    for (i, b) in output.bytes().enumerate() {
        if b == b'\r' {
            print!("[CR]");
        } else if b == b'\n' {
            print!("[LF]");
            if i < output.len() - 1 {
                println!(); // Actually print newline for readability
            }
        } else if b < 32 {
            print!("[0x{:02x}]", b);
        } else {
            print!("{}", b as char);
        }
    }
    println!();
    println!();
    println!("Rendered output:");
    print!("{}", output);
    println!();
    println!();

    // Test 5: Full UI
    println!("=== Test 5: Full sage-like UI ===");
    let welcome = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new("Sage Agent").color(Color::Cyan).bold().into_element())
        .child(Text::new("Subtitle").dim().into_element())
        .into_element();

    let separator = "─".repeat(term_width as usize);
    let bottom = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new(&separator).dim().into_element())
        .child(Text::new("❯ Input prompt").into_element())
        .into_element();

    let root = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .width(term_width as i32)
        .height(10)
        .child(welcome)
        .child(bottom)
        .into_element();

    let output = rnk::render_to_string(&root, term_width);

    println!("Output line count: {}", output.lines().count());
    println!("Output contains CR: {}", output.contains('\r'));
    println!("Output contains LF: {}", output.contains('\n'));
    println!();

    println!("Line-by-line analysis:");
    for (i, line) in output.lines().enumerate() {
        let has_ansi = line.contains('\x1b');
        let display_len = strip_ansi(line).len();
        println!("  Line {}: len={}, has_ansi={}, content='{}'",
            i, display_len, has_ansi,
            strip_ansi(line).chars().take(40).collect::<String>());
    }
    println!();

    println!("Rendered output:");
    println!("--- START ---");
    print!("{}", output);
    println!();
    println!("--- END ---");

    println!();
    println!("=== Diagnostic Complete ===");
}

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
