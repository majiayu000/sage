//! Comprehensive UI Layout Tests for sage-cli
//!
//! These tests ensure correct rendering of the rnk-based UI.
//! Based on best practices from Ratatui, Ink, and Bubbletea testing patterns.

use rnk::layout::LayoutEngine;
use rnk::prelude::*;
use rnk::prelude::Box as RnkBox;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

// ============================================================================
// Test Utilities
// ============================================================================

/// Strip ANSI escape codes from a string
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

/// Get the starting column (number of leading spaces) of each line
fn get_line_starts(output: &str) -> Vec<usize> {
    output
        .lines()
        .map(|line| {
            let stripped = strip_ansi(line);
            stripped.len() - stripped.trim_start().len()
        })
        .collect()
}

/// Get display width of a string (accounting for wide characters)
fn display_width(s: &str) -> usize {
    s.width()
}

fn truncate_to_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if text.width() <= max_width {
        return text.to_string();
    }

    let mut trimmed = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width + 3 > max_width {
            break;
        }
        trimmed.push(ch);
        width += ch_width;
    }
    trimmed.push_str("...");
    trimmed
}

/// Layout assertion helper for fluent testing
struct LayoutAssertion {
    output: String,
    width: u16,
    height: u16,
}

impl LayoutAssertion {
    fn new(element: &Element, width: u16, height: u16) -> Self {
        let output = rnk::render_to_string(element, width);
        Self { output, width, height }
    }

    fn assert_contains(&self, text: &str) -> &Self {
        assert!(
            self.output.contains(text),
            "Output does not contain '{}'\nActual output:\n{}",
            text, self.output
        );
        self
    }

    fn assert_all_lines_left_aligned(&self) -> &Self {
        let starts = get_line_starts(&self.output);
        for (i, &start) in starts.iter().enumerate() {
            let line = self.output.lines().nth(i).unwrap_or("");
            let stripped = strip_ansi(line);
            if !stripped.trim().is_empty() {
                let allowed_indent = if stripped.starts_with("  ▘▘") || stripped.starts_with("  /model") {
                    2
                } else {
                    0
                };
                assert_eq!(
                    start, allowed_indent,
                    "Line {} should start at column {}, but starts at {}.\nLine content: '{}'",
                    i, allowed_indent, start, stripped
                );
            }
        }
        self
    }

    fn assert_no_line_exceeds_width(&self) -> &Self {
        for (i, line) in self.output.lines().enumerate() {
            let stripped = strip_ansi(line);
            let line_width = display_width(&stripped);
            assert!(
                line_width <= self.width as usize,
                "Line {} width {} exceeds terminal width {}\nLine content: '{}'",
                i, line_width, self.width, stripped
            );
        }
        self
    }

    fn debug_print(&self) -> &Self {
        println!("=== Layout Debug ({}x{}) ===", self.width, self.height);
        for (i, line) in self.output.lines().enumerate() {
            let stripped = strip_ansi(line);
            let leading = stripped.len() - stripped.trim_start().len();
            println!("{:3}: col={:3} |{}|", i, leading, stripped);
        }
        println!("=== End ===");
        self
    }
}

/// Debug helper to print Taffy layout tree
fn print_layout_tree(element: &Element, engine: &LayoutEngine, indent: usize) {
    let layout = engine.get_layout(element.id);
    let prefix = "  ".repeat(indent);

    let name = if let Some(text) = &element.text_content {
        let t: String = text.chars().take(20).collect();
        format!("Text(\"{}\")", t)
    } else {
        format!("Box({:?})", element.style.flex_direction)
    };

    if let Some(l) = layout {
        let marker = if l.x > 0.1 { " <-- X != 0" } else { "" };
        println!("{}{}: x={:.1}, y={:.1}, w={:.1}, h={:.1}{}",
            prefix, name, l.x, l.y, l.width, l.height, marker);
    } else {
        println!("{}{}: NO LAYOUT", prefix, name);
    }

    for child in &element.children {
        print_layout_tree(child, engine, indent + 1);
    }
}

// ============================================================================
// Sage UI Component Builders (matching rnk_app.rs exactly)
// ============================================================================

fn build_header(width: u16) -> Element {
    let version = env!("CARGO_PKG_VERSION");
    let title = truncate_to_width(
        &format!("▐▛███▜▌   Sage Code v{}", version),
        width as usize,
    );
    let model_line = truncate_to_width("▝▜█████▛▘  unknown · unknown", width as usize);
    let cwd_line = truncate_to_width("  ▘▘ ▝▝    .", width as usize);
    let hint_line = truncate_to_width("  /model to try another model", width as usize);

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new(title)
                .color(Color::Black)
                .bold()
                .into_element(),
        )
        .child(Text::new(model_line).color(Color::Black).into_element())
        .child(Text::new(cwd_line).color(Color::Black).into_element())
        .child(Newline::new().into_element())
        .child(
            Text::new(hint_line)
                .color(Color::Black)
                .into_element(),
        )
        .child(Newline::new().into_element())
        .into_element()
}

fn build_input_prompt(input_text: &str) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("❯ ").color(Color::Black).bold().into_element())
        .child(
            Text::new(if input_text.is_empty() {
                "Try \"edit base.rs to...\""
            } else {
                input_text
            })
            .color(Color::Black)
            .into_element(),
        )
        .into_element()
}

fn build_status_bar(mode: &str) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("⏵⏵").color(Color::Black).into_element())
        .child(Text::new(format!(" {}", mode)).color(Color::Black).into_element())
        .child(Text::new(" (shift+tab to cycle)").color(Color::Black).into_element())
        .into_element()
}

fn build_bottom_area(term_width: u16) -> Element {
    let separator = "─".repeat(term_width as usize);

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new(&separator).color(Color::Black).into_element())
        .child(build_input_prompt(""))
        .child(build_status_bar("permissions required"))
        .into_element()
}

fn build_full_ui(term_width: u16, term_height: u16) -> Element {
    let header = build_header(term_width);
    let content = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new("").into_element())
        .into_element();
    let bottom = build_bottom_area(term_width);

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .width(term_width as i32)
        .height(term_height as i32)
        .child(header)
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Column)
                .flex_grow(1.0)
                .child(content)
                .into_element(),
        )
        .child(bottom)
        .into_element()
}

// ============================================================================
// Layout Tests
// ============================================================================

#[test]
fn test_header_left_aligned() {
    let element = build_header(80);

    LayoutAssertion::new(&element, 80, 24)
        .debug_print()
        .assert_contains("Sage Code v")
        .assert_contains("/model to try")
        .assert_all_lines_left_aligned();
}

#[test]
fn test_input_prompt_left_aligned() {
    let element = build_input_prompt("");

    LayoutAssertion::new(&element, 80, 24)
        .debug_print()
        .assert_contains("❯")
        .assert_contains("Try \"edit base.rs to")
        .assert_all_lines_left_aligned();
}

#[test]
fn test_status_bar_left_aligned() {
    let element = build_status_bar("permissions required");

    LayoutAssertion::new(&element, 80, 24)
        .debug_print()
        .assert_contains("⏵⏵")
        .assert_contains("permissions required")
        .assert_all_lines_left_aligned();
}

#[test]
fn test_bottom_area_left_aligned() {
    let element = build_bottom_area(80);

    LayoutAssertion::new(&element, 80, 24)
        .debug_print()
        .assert_contains("─")
        .assert_contains("❯")
        .assert_all_lines_left_aligned();
}


#[test]
fn test_full_ui_left_aligned() {
    let element = build_full_ui(80, 24);

    LayoutAssertion::new(&element, 80, 24)
        .debug_print()
        .assert_contains("Sage Code v")
        .assert_contains("/model to try")
        .assert_contains("❯")
        .assert_contains("permissions required")
        .assert_all_lines_left_aligned()
        .assert_no_line_exceeds_width();
}

#[test]
fn test_full_ui_various_widths() {
    for width in [40, 60, 80, 100, 120, 160] {
        println!("\n=== Testing width {} ===", width);
        let element = build_full_ui(width, 24);

        LayoutAssertion::new(&element, width, 24)
            .assert_all_lines_left_aligned()
            .assert_no_line_exceeds_width();
    }
}

#[test]
fn test_taffy_layout_positions() {
    let term_width = 80u16;
    let term_height = 24u16;
    let element = build_full_ui(term_width, term_height);

    let mut engine = LayoutEngine::new();
    engine.compute(&element, term_width, term_height);

    println!("=== Taffy Layout Tree ===");
    print_layout_tree(&element, &engine, 0);

    // Check that root element starts at (0, 0)
    let root_layout = engine.get_layout(element.id).expect("Root should have layout");
    assert_eq!(root_layout.x, 0.0, "Root x should be 0");
    assert_eq!(root_layout.y, 0.0, "Root y should be 0");
    assert_eq!(root_layout.width as u16, term_width, "Root width should match terminal width");
}

#[test]
fn test_separator_full_width() {
    let term_width = 80u16;
    let separator = "─".repeat(term_width as usize);
    let element = Text::new(&separator).into_element();

    let output = rnk::render_to_string(&element, term_width);
    let stripped = strip_ansi(&output);
    let first_line = stripped.lines().next().unwrap_or("");

    // Separator character is 1 display width each
    let actual_width = display_width(first_line);
    assert_eq!(
        actual_width, term_width as usize,
        "Separator should be exactly {} chars wide, got {}",
        term_width, actual_width
    );
}

// ============================================================================
// Regression Tests
// ============================================================================

#[test]
fn test_no_content_offset_with_flex_grow() {
    // This test catches the bug where flex_grow caused content to be offset
    let term_width = 100u16;
    let term_height = 30u16;

    let element = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .width(term_width as i32)
        .height(term_height as i32)
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Column)
                .flex_grow(1.0)
                .child(Text::new("Content in flex-grow area").into_element())
                .into_element(),
        )
        .child(Text::new("Bottom content").into_element())
        .into_element();

    let output = rnk::render_to_string(&element, term_width);
    let starts = get_line_starts(&output);

    for (i, &start) in starts.iter().enumerate() {
        let line = output.lines().nth(i).unwrap_or("");
        let stripped = strip_ansi(line);
        if !stripped.trim().is_empty() {
            assert_eq!(
                start, 0,
                "Line {} with flex_grow should start at column 0, starts at {}",
                i, start
            );
        }
    }
}

#[test]
fn test_row_layout_children_positions() {
    // Row layout children should have increasing x positions
    let element = RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("Left").into_element())
        .child(Text::new("Right").into_element())
        .into_element();

    let output = rnk::render_to_string(&element, 80);
    let stripped = strip_ansi(&output);

    // Should be on same line
    assert_eq!(output.lines().count(), 1, "Row should produce single line");
    assert!(stripped.contains("Left"), "Should contain Left");
    assert!(stripped.contains("Right"), "Should contain Right");

    // Left should come before Right
    let left_pos = stripped.find("Left").unwrap();
    let right_pos = stripped.find("Right").unwrap();
    assert!(left_pos < right_pos, "Left should be before Right");
}
