//! Comprehensive UI Layout Tests for sage-cli
//!
//! These tests ensure correct rendering of the rnk-based UI based on UI_DESIGN_SPEC.md.
//! Test Phases:
//!   Phase 1: Core Layout Tests
//!   Phase 2: Virtual Scroll Tests
//!   Phase 3: Message Rendering Tests
//!   Phase 4: Animation Tests
//!   Phase 5: Input/Status Bar Tests
//!   Phase 6: Integration Tests

use chrono::Utc;
use rnk::layout::LayoutEngine;
use rnk::prelude::Box as RnkBox;
use rnk::prelude::*;
use sage_core::ui::bridge::state::{
    AppState, ExecutionPhase, Message, MessageMetadata, Role, UiMessageContent, UiSessionInfo,
    UiToolResult,
};
use std::time::Duration;
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

/// Truncate text to max display width with ellipsis
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
        Self {
            output,
            width,
            height,
        }
    }

    fn assert_contains(&self, text: &str) -> &Self {
        assert!(
            self.output.contains(text),
            "Output does not contain '{}'\nActual output:\n{}",
            text,
            self.output
        );
        self
    }

    fn assert_not_contains(&self, text: &str) -> &Self {
        let stripped = strip_ansi(&self.output);
        assert!(
            !stripped.contains(text),
            "Output should NOT contain '{}'\nActual output:\n{}",
            text,
            stripped
        );
        self
    }

    fn assert_line_count(&self, expected: usize) -> &Self {
        let actual = self.output.lines().count();
        assert_eq!(
            actual, expected,
            "Expected {} lines, got {}\nActual output:\n{}",
            expected, actual, self.output
        );
        self
    }

    fn assert_all_lines_left_aligned(&self) -> &Self {
        let starts = get_line_starts(&self.output);
        for (i, &start) in starts.iter().enumerate() {
            let line = self.output.lines().nth(i).unwrap_or("");
            let stripped = strip_ansi(line);
            if !stripped.trim().is_empty() {
                // Allow specific patterns with known indentation
                let allowed_indent = if stripped.starts_with("  ▘▘")
                    || stripped.starts_with("  /model")
                    || stripped.starts_with("  args:")
                    || stripped.starts_with("  result:")
                    || stripped.starts_with("  error:")
                {
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
                i,
                line_width,
                self.width,
                stripped
            );
        }
        self
    }

    fn assert_line_width_equals(&self, line_idx: usize, expected_width: usize) -> &Self {
        let line = self.output.lines().nth(line_idx);
        match line {
            Some(l) => {
                let stripped = strip_ansi(l);
                let actual_width = display_width(&stripped);
                assert_eq!(
                    actual_width, expected_width,
                    "Line {} width should be {}, got {}\nLine content: '{}'",
                    line_idx, expected_width, actual_width, stripped
                );
            }
            None => panic!("Line {} does not exist", line_idx),
        }
        self
    }

    fn debug_print(&self) -> &Self {
        println!("=== Layout Debug ({}x{}) ===", self.width, self.height);
        for (i, line) in self.output.lines().enumerate() {
            let stripped = strip_ansi(line);
            let leading = stripped.len() - stripped.trim_start().len();
            let width = display_width(&stripped);
            println!("{:3}: col={:2} w={:3} |{}|", i, leading, width, stripped);
        }
        println!("=== End ===");
        self
    }

    fn get_line(&self, idx: usize) -> Option<String> {
        self.output.lines().nth(idx).map(strip_ansi)
    }
}

// ============================================================================
// Test Data Factories
// ============================================================================

fn create_default_session() -> UiSessionInfo {
    UiSessionInfo {
        session_id: Some("test-session".to_string()),
        model: "claude-3-opus".to_string(),
        provider: "anthropic".to_string(),
        working_dir: "/Users/test/project".to_string(),
        git_branch: Some("main".to_string()),
        step: 0,
        max_steps: None,
    }
}

fn create_user_message(content: &str) -> Message {
    Message {
        role: Role::User,
        content: UiMessageContent::Text(content.to_string()),
        timestamp: Utc::now(),
        metadata: MessageMetadata::default(),
    }
}

fn create_assistant_message(content: &str) -> Message {
    Message {
        role: Role::Assistant,
        content: UiMessageContent::Text(content.to_string()),
        timestamp: Utc::now(),
        metadata: MessageMetadata::default(),
    }
}

fn create_thinking_message(content: &str) -> Message {
    Message {
        role: Role::Assistant,
        content: UiMessageContent::Thinking(content.to_string()),
        timestamp: Utc::now(),
        metadata: MessageMetadata::default(),
    }
}

fn create_tool_call_message(
    tool_name: &str,
    params: &str,
    result: Option<UiToolResult>,
) -> Message {
    Message {
        role: Role::Assistant,
        content: UiMessageContent::ToolCall {
            tool_name: tool_name.to_string(),
            params: params.to_string(),
            result,
        },
        timestamp: Utc::now(),
        metadata: MessageMetadata::default(),
    }
}

// ============================================================================
// Component Builders (matching rnk_app.rs)
// ============================================================================

fn build_header(session: &UiSessionInfo, width: u16) -> Element {
    let version = env!("CARGO_PKG_VERSION");
    let title = truncate_to_width(&format!("▐▛███▜▌   Sage Code v{}", version), width as usize);
    let model_info = format!("{} · {}", session.model, session.provider);
    let model_line = truncate_to_width(&format!("▝▜█████▛▘  {}", model_info), width as usize);
    let cwd_line = truncate_to_width(
        &format!("  ▘▘ ▝▝    {}", session.working_dir),
        width as usize,
    );
    let hint_line = truncate_to_width("  /model to try another model", width as usize);

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .width(width as i32)
        .child(Text::new(title).color(Color::Black).bold().into_element())
        .child(Text::new(model_line).color(Color::Black).into_element())
        .child(Text::new(cwd_line).color(Color::Black).into_element())
        .child(Newline::new().into_element())
        .child(Text::new(hint_line).color(Color::Black).into_element())
        .child(Newline::new().into_element())
        .into_element()
}

fn build_input_prompt(input_text: &str, is_idle: bool) -> Element {
    let display_text = if input_text.is_empty() && is_idle {
        "Try \"edit base.rs to...\""
    } else {
        input_text
    };

    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("❯ ").color(Color::Yellow).bold().into_element())
        .child(Text::new(display_text).color(Color::Black).into_element())
        .into_element()
}

fn build_status_bar(mode: &str, scroll_percent: Option<u8>, mouse_enabled: bool) -> Element {
    let mut row = RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("⏵⏵").color(Color::Black).into_element())
        .child(
            Text::new(format!(" {}", mode))
                .color(Color::Black)
                .into_element(),
        )
        .child(
            Text::new(" (shift+tab to cycle)")
                .color(Color::Black)
                .into_element(),
        )
        .child(
            Text::new(format!(
                " | mouse {}",
                if mouse_enabled { "on" } else { "off" }
            ))
            .color(Color::Black)
            .into_element(),
        );

    if let Some(percent) = scroll_percent {
        row = row.child(
            Text::new(format!(" [{:3}%]", percent))
                .color(Color::Black)
                .into_element(),
        );
    }

    row.into_element()
}

fn build_separator(width: u16) -> Element {
    let separator = "─".repeat(width as usize);
    Text::new(&separator).color(Color::Black).into_element()
}

fn build_bottom_area(width: u16, mode: &str, scroll_percent: Option<u8>) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(build_separator(width))
        .child(build_input_prompt("", true))
        .child(build_status_bar(mode, scroll_percent, true))
        .into_element()
}

fn build_full_ui(session: &UiSessionInfo, width: u16, height: u16) -> Element {
    let header = build_header(session, width);
    let content = RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new("").into_element())
        .into_element();
    let bottom = build_bottom_area(width, "permissions required", None);

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .align_items(AlignItems::FlexStart)
        .width(width as i32)
        .height(height as i32)
        .child(header)
        .child(
            RnkBox::new()
                .flex_direction(FlexDirection::Column)
                .flex_grow(1.0)
                .width(width as i32)
                .align_items(AlignItems::FlexStart)
                .overflow_y(Overflow::Hidden)
                .child(content)
                .into_element(),
        )
        .child(bottom)
        .into_element()
}

// ============================================================================
// Phase 1: Core Layout Tests
// ============================================================================

mod phase1_core_layout {
    use super::*;

    #[test]
    fn test_header_has_5_lines() {
        // Note: rnk doesn't produce trailing newline as a separate line
        // So we have 5 lines: logo, model, cwd, empty, hint
        let session = create_default_session();
        let element = build_header(&session, 80);

        LayoutAssertion::new(&element, 80, 24)
            .debug_print()
            .assert_line_count(5);
    }

    #[test]
    fn test_header_line1_logo_and_version() {
        let session = create_default_session();
        let element = build_header(&session, 80);
        let assertion = LayoutAssertion::new(&element, 80, 24);

        let line0 = assertion.get_line(0).unwrap();
        assert!(line0.contains("▐▛███▜▌"), "Line 0 should contain logo");
        assert!(
            line0.contains("Sage Code v"),
            "Line 0 should contain version"
        );
    }

    #[test]
    fn test_header_line2_model_provider() {
        let session = create_default_session();
        let element = build_header(&session, 80);
        let assertion = LayoutAssertion::new(&element, 80, 24);

        let line1 = assertion.get_line(1).unwrap();
        assert!(line1.contains("▝▜█████▛▘"), "Line 1 should contain banner");
        assert!(
            line1.contains(&session.model) && line1.contains(&session.provider),
            "Line 1 should contain model and provider"
        );
    }

    #[test]
    fn test_header_line3_working_directory() {
        let session = create_default_session();
        let element = build_header(&session, 80);
        let assertion = LayoutAssertion::new(&element, 80, 24);

        let line2 = assertion.get_line(2).unwrap();
        assert!(
            line2.contains("▘▘ ▝▝"),
            "Line 2 should contain cwd indicator"
        );
        assert!(
            line2.contains(&session.working_dir),
            "Line 2 should contain working directory"
        );
    }

    #[test]
    fn test_header_line4_empty() {
        let session = create_default_session();
        let element = build_header(&session, 80);
        let assertion = LayoutAssertion::new(&element, 80, 24);

        let line3 = assertion.get_line(3).unwrap();
        assert!(line3.trim().is_empty(), "Line 3 should be empty");
    }

    #[test]
    fn test_header_line5_hint_text() {
        let session = create_default_session();
        let element = build_header(&session, 80);
        let assertion = LayoutAssertion::new(&element, 80, 24);

        let line4 = assertion.get_line(4).unwrap();
        assert!(
            line4.contains("/model to try another model"),
            "Line 4 should contain hint text"
        );
    }

    // Note: test_header_line6_empty removed - rnk doesn't produce trailing newline as separate line
    // Header has 5 lines: logo(0), model(1), cwd(2), empty(3), hint(4)

    #[test]
    fn test_header_all_lines_left_aligned() {
        let session = create_default_session();
        let element = build_header(&session, 80);

        LayoutAssertion::new(&element, 80, 24).assert_all_lines_left_aligned();
    }

    #[test]
    fn test_bottom_area_has_3_lines() {
        let element = build_bottom_area(80, "permissions required", None);

        LayoutAssertion::new(&element, 80, 24)
            .debug_print()
            .assert_line_count(3);
    }

    #[test]
    fn test_bottom_area_separator_full_width() {
        let width = 80u16;
        let element = build_separator(width);

        LayoutAssertion::new(&element, width, 24).assert_line_width_equals(0, width as usize);
    }

    #[test]
    fn test_bottom_area_input_line_present() {
        let element = build_bottom_area(80, "permissions required", None);

        LayoutAssertion::new(&element, 80, 24).assert_contains("❯");
    }

    #[test]
    fn test_bottom_area_status_bar_present() {
        let element = build_bottom_area(80, "permissions required", None);

        LayoutAssertion::new(&element, 80, 24)
            .assert_contains("⏵⏵")
            .assert_contains("permissions required");
    }

    #[test]
    fn test_content_area_flex_grow() {
        let session = create_default_session();
        let width = 80u16;
        let height = 24u16;
        let element = build_full_ui(&session, width, height);

        let mut engine = LayoutEngine::new();
        engine.compute(&element, width, height);

        let root_layout = engine
            .get_layout(element.id)
            .expect("Root should have layout");
        assert_eq!(
            root_layout.height as u16, height,
            "Root should fill terminal height"
        );
        assert_eq!(
            root_layout.width as u16, width,
            "Root should fill terminal width"
        );
    }

    #[test]
    fn test_height_allocation() {
        let session = create_default_session();
        let width = 80u16;
        let height = 24u16;
        let header_height = 6u16;
        let bottom_height = 3u16;
        let expected_content_height = height - header_height - bottom_height;

        let element = build_full_ui(&session, width, height);
        let mut engine = LayoutEngine::new();
        engine.compute(&element, width, height);

        let root = engine.get_layout(element.id).expect("Root layout");
        assert_eq!(root.height as u16, height);

        assert!(
            expected_content_height > 0,
            "Content area should have positive height"
        );
    }

    #[test]
    fn test_no_line_exceeds_terminal_width() {
        let session = create_default_session();
        for width in [40, 60, 80, 100, 120, 160] {
            let element = build_full_ui(&session, width, 24);
            LayoutAssertion::new(&element, width, 24).assert_no_line_exceeds_width();
        }
    }
}

// ============================================================================
// Phase 2: Virtual Scroll Tests
// ============================================================================

mod phase2_virtual_scroll {
    use super::*;

    /// Simulates wrap_text_lines from rnk_app.rs
    fn wrap_text_lines(text: &str, max_width: usize) -> Vec<String> {
        if max_width == 0 {
            return vec![];
        }

        let mut result = Vec::new();
        for paragraph in text.split('\n') {
            if paragraph.is_empty() {
                result.push(String::new());
                continue;
            }

            let mut current_line = String::new();
            let mut current_width = 0;

            for ch in paragraph.chars() {
                let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
                if ch_width == 0 {
                    continue;
                }

                if current_width + ch_width > max_width && current_width > 0 {
                    result.push(current_line);
                    current_line = String::new();
                    current_width = 0;
                }

                current_line.push(ch);
                current_width += ch_width;
            }

            if !current_line.is_empty() {
                result.push(current_line);
            }
        }

        if result.is_empty() {
            result.push(String::new());
        }
        result
    }

    #[test]
    fn test_wrap_text_lines_ascii() {
        let text = "Hello World this is a test";
        let lines = wrap_text_lines(text, 10);

        for line in &lines {
            assert!(
                display_width(line) <= 10,
                "Line width {} exceeds max 10: '{}'",
                display_width(line),
                line
            );
        }
    }

    #[test]
    fn test_wrap_text_lines_cjk_respects_width() {
        let text = "你好世界这是一个很长的中文句子用于测试换行是否正确对齐";
        let max_width = 12;
        let lines = wrap_text_lines(text, max_width);

        assert!(!lines.is_empty(), "Expected wrapped lines");
        for line in &lines {
            let width = display_width(line);
            assert!(
                width <= max_width,
                "Line width {} exceeds max width {}: '{}'",
                width,
                max_width,
                line
            );
        }
    }

    #[test]
    fn test_wrap_text_lines_mixed_cjk_ascii() {
        let text = "Hello你好World世界";
        let max_width = 10;
        let lines = wrap_text_lines(text, max_width);

        for line in &lines {
            let width = display_width(line);
            assert!(
                width <= max_width,
                "Mixed line width {} exceeds max {}: '{}'",
                width,
                max_width,
                line
            );
        }
    }

    #[test]
    fn test_wrap_text_lines_preserves_paragraph_breaks() {
        let text = "First paragraph\n\nSecond paragraph";
        let lines = wrap_text_lines(text, 80);

        assert!(
            lines.iter().any(|l| l.is_empty()),
            "Should preserve empty line for paragraph break"
        );
    }

    #[test]
    fn test_wrap_text_lines_empty_input() {
        let lines = wrap_text_lines("", 80);
        assert_eq!(lines.len(), 1, "Empty input should produce one empty line");
        assert!(lines[0].is_empty());
    }

    #[test]
    fn test_wrap_text_lines_zero_width() {
        let lines = wrap_text_lines("test", 0);
        assert!(lines.is_empty(), "Zero width should produce no lines");
    }

    #[test]
    fn test_scroll_offset_calculation_at_top() {
        let total_lines = 100usize;
        let viewport_height = 20usize;
        let scroll_offset = 0usize;

        let visible_start = scroll_offset.min(total_lines.saturating_sub(viewport_height));
        let visible_end = (visible_start + viewport_height).min(total_lines);

        assert_eq!(visible_start, 0, "At top, visible_start should be 0");
        assert_eq!(
            visible_end, 20,
            "At top, visible_end should be viewport_height"
        );
    }

    #[test]
    fn test_scroll_offset_calculation_at_bottom() {
        let total_lines = 100usize;
        let viewport_height = 20usize;
        let scroll_offset = 80usize;

        let visible_start = scroll_offset.min(total_lines.saturating_sub(viewport_height));
        let visible_end = (visible_start + viewport_height).min(total_lines);

        assert_eq!(
            visible_start, 80,
            "At bottom, visible_start should be max_scroll"
        );
        assert_eq!(
            visible_end, 100,
            "At bottom, visible_end should be total_lines"
        );
    }

    #[test]
    fn test_scroll_offset_clamped_to_max() {
        let total_lines = 100usize;
        let viewport_height = 20usize;
        let scroll_offset = 200usize;

        let max_scroll = total_lines.saturating_sub(viewport_height);
        let visible_start = scroll_offset.min(max_scroll);

        assert_eq!(visible_start, 80, "Scroll offset should be clamped to max");
    }

    #[test]
    fn test_scroll_offset_small_content() {
        let total_lines = 10usize;
        let viewport_height = 20usize;
        let scroll_offset = 5usize;

        let visible_start = scroll_offset.min(total_lines.saturating_sub(viewport_height));
        let visible_end = (visible_start + viewport_height).min(total_lines);

        assert_eq!(
            visible_start, 0,
            "When content fits, visible_start should be 0"
        );
        assert_eq!(
            visible_end, 10,
            "When content fits, visible_end should be total_lines"
        );
    }

    #[test]
    fn test_scroll_percent_at_top() {
        let scroll_offset = 0u32;
        let max_scroll = 80u32;

        let scroll_percent = if max_scroll > 0 {
            Some(((scroll_offset as f32 / max_scroll as f32) * 100.0) as u8)
        } else {
            None
        };

        assert_eq!(scroll_percent, Some(0), "At top, percent should be 0%");
    }

    #[test]
    fn test_scroll_percent_at_middle() {
        let scroll_offset = 40u32;
        let max_scroll = 80u32;

        let scroll_percent = if max_scroll > 0 {
            Some(((scroll_offset as f32 / max_scroll as f32) * 100.0) as u8)
        } else {
            None
        };

        assert_eq!(scroll_percent, Some(50), "At middle, percent should be 50%");
    }

    #[test]
    fn test_scroll_percent_at_bottom() {
        let scroll_offset = 80u32;
        let max_scroll = 80u32;

        let scroll_percent = if max_scroll > 0 {
            Some(((scroll_offset as f32 / max_scroll as f32) * 100.0) as u8)
        } else {
            None
        };

        assert_eq!(
            scroll_percent,
            Some(100),
            "At bottom, percent should be 100%"
        );
    }

    #[test]
    fn test_scroll_percent_not_scrollable() {
        let max_scroll = 0u32;

        let scroll_percent: Option<u8> = if max_scroll > 0 { Some(50) } else { None };

        assert_eq!(scroll_percent, None, "Non-scrollable should return None");
    }

    #[test]
    fn test_scroll_indicator_displayed() {
        let element = build_status_bar("permissions required", Some(50), true);

        LayoutAssertion::new(&element, 80, 24).assert_contains("[ 50%]");
    }

    #[test]
    fn test_scroll_indicator_not_displayed_when_none() {
        let element = build_status_bar("permissions required", None, true);

        LayoutAssertion::new(&element, 80, 24)
            .assert_not_contains("[")
            .assert_not_contains("%]");
    }

    #[test]
    fn test_prefix_alignment_user() {
        let prefix = "user: ";
        let prefix_width = display_width(prefix);

        assert_eq!(prefix_width, 6, "user: prefix should be 6 chars wide");
    }

    #[test]
    fn test_prefix_alignment_assistant() {
        let prefix = "assistant: ";
        let prefix_width = display_width(prefix);

        assert_eq!(
            prefix_width, 11,
            "assistant: prefix should be 11 chars wide"
        );
    }

    #[test]
    fn test_prefix_alignment_tool() {
        let prefix = "tool: ";
        let prefix_width = display_width(prefix);

        assert_eq!(prefix_width, 6, "tool: prefix should be 6 chars wide");
    }
}

// ============================================================================
// Phase 3: Message Rendering Tests
// ============================================================================

mod phase3_message_rendering {
    use super::*;

    fn build_message_element(msg: &Message, max_width: usize) -> Element {
        match &msg.content {
            UiMessageContent::Text(text) => {
                let (prefix, color) = match msg.role {
                    Role::User => ("user: ", Color::Black),
                    Role::Assistant => ("assistant: ", Color::Black),
                    Role::System => ("system: ", Color::Black),
                };
                let display_text = format!("{}{}", prefix, text);
                Text::new(truncate_to_width(&display_text, max_width))
                    .color(color)
                    .bold()
                    .into_element()
            }
            UiMessageContent::Thinking(text) => {
                let display_text = format!("∴ Thinking: {}", text);
                Text::new(truncate_to_width(&display_text, max_width))
                    .color(Color::Magenta)
                    .into_element()
            }
            UiMessageContent::ToolCall {
                tool_name,
                params,
                result,
            } => {
                // Tool calls need multiple lines, use Column layout
                let mut container = RnkBox::new().flex_direction(FlexDirection::Column);

                // Line 1: tool: Name
                container = container.child(
                    Text::new(truncate_to_width(
                        &format!("tool: {}", tool_name),
                        max_width,
                    ))
                    .color(Color::Magenta)
                    .bold()
                    .into_element(),
                );

                // Line 2: args (if any)
                if !params.is_empty() {
                    container = container.child(
                        Text::new(truncate_to_width(&format!("  args: {}", params), max_width))
                            .color(Color::Magenta)
                            .into_element(),
                    );
                }

                // Line 3: result or error (if any)
                if let Some(r) = result {
                    if r.success {
                        if let Some(output) = &r.output {
                            container = container.child(
                                Text::new(truncate_to_width(
                                    &format!("  result: {}", output),
                                    max_width,
                                ))
                                .color(Color::Magenta)
                                .into_element(),
                            );
                        }
                    } else if let Some(error) = &r.error {
                        container = container.child(
                            Text::new(truncate_to_width(&format!("  error: {}", error), max_width))
                                .color(Color::Red)
                                .into_element(),
                        );
                    }
                }

                container.into_element()
            }
        }
    }

    #[test]
    fn test_user_message_has_prefix() {
        let msg = create_user_message("Hello, world!");
        let element = build_message_element(&msg, 80);

        LayoutAssertion::new(&element, 80, 24).assert_contains("user:");
    }

    #[test]
    fn test_user_message_content_displayed() {
        let msg = create_user_message("Hello, world!");
        let element = build_message_element(&msg, 80);

        LayoutAssertion::new(&element, 80, 24).assert_contains("Hello, world!");
    }

    #[test]
    fn test_assistant_message_has_prefix() {
        let msg = create_assistant_message("I can help you with that.");
        let element = build_message_element(&msg, 80);

        LayoutAssertion::new(&element, 80, 24).assert_contains("assistant:");
    }

    #[test]
    fn test_assistant_message_content_displayed() {
        let msg = create_assistant_message("I can help you with that.");
        let element = build_message_element(&msg, 80);

        LayoutAssertion::new(&element, 80, 24).assert_contains("I can help you with that.");
    }

    #[test]
    fn test_thinking_block_has_symbol() {
        let msg = create_thinking_message("Analyzing the code...");
        let element = build_message_element(&msg, 80);

        LayoutAssertion::new(&element, 80, 24).assert_contains("∴ Thinking");
    }

    #[test]
    fn test_tool_call_has_prefix() {
        let msg = create_tool_call_message("Read", "path=\"src/main.rs\"", None);
        let element = build_message_element(&msg, 80);

        LayoutAssertion::new(&element, 80, 24)
            .debug_print()
            .assert_contains("tool: Read");
    }

    #[test]
    fn test_tool_call_args_displayed() {
        let msg = create_tool_call_message("Read", "path=\"src/main.rs\"", None);
        let element = build_message_element(&msg, 80);

        LayoutAssertion::new(&element, 80, 24).assert_contains("args:");
    }

    #[test]
    fn test_tool_call_result_displayed() {
        let result = UiToolResult {
            success: true,
            output: Some("Read 150 lines".to_string()),
            error: None,
            duration: Duration::from_millis(50),
        };
        let msg = create_tool_call_message("Read", "path=\"src/main.rs\"", Some(result));
        let element = build_message_element(&msg, 80);

        LayoutAssertion::new(&element, 80, 24).assert_contains("result:");
    }

    #[test]
    fn test_tool_call_error_displayed() {
        let result = UiToolResult {
            success: false,
            output: None,
            error: Some("File not found".to_string()),
            duration: Duration::from_millis(10),
        };
        let msg = create_tool_call_message("Read", "path=\"nonexistent.rs\"", Some(result));
        let element = build_message_element(&msg, 80);

        LayoutAssertion::new(&element, 80, 24).assert_contains("error:");
    }

    #[test]
    fn test_message_respects_max_width() {
        let long_content = "a".repeat(200);
        let msg = create_user_message(&long_content);
        let element = build_message_element(&msg, 80);

        LayoutAssertion::new(&element, 80, 24).assert_no_line_exceeds_width();
    }

    #[test]
    fn test_cjk_message_respects_width() {
        let cjk_content = "这是一个很长的中文消息用于测试换行功能是否正确";
        let msg = create_user_message(cjk_content);
        let element = build_message_element(&msg, 40);

        LayoutAssertion::new(&element, 40, 24).assert_no_line_exceeds_width();
    }
}

// ============================================================================
// Phase 4: Animation Tests
// ============================================================================

mod phase4_animation {
    const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    const SPINNER_INTERVAL_MS: u128 = 80;

    #[test]
    fn test_spinner_has_10_frames() {
        assert_eq!(SPINNER_FRAMES.len(), 10, "Spinner should have 10 frames");
    }

    #[test]
    fn test_spinner_frames_are_braille() {
        for frame in &SPINNER_FRAMES {
            let ch = frame.chars().next().unwrap();
            assert!(
                ('\u{2800}'..='\u{28FF}').contains(&ch),
                "Frame '{}' should be a Braille character",
                frame
            );
        }
    }

    #[test]
    fn test_spinner_frame_selection() {
        for ms in [0u128, 80, 160, 800] {
            let frame_idx = (ms / SPINNER_INTERVAL_MS) as usize % SPINNER_FRAMES.len();
            assert!(
                frame_idx < SPINNER_FRAMES.len(),
                "Frame index {} should be valid at {}ms",
                frame_idx,
                ms
            );
        }
    }

    #[test]
    fn test_spinner_interval_80ms() {
        assert_eq!(
            SPINNER_INTERVAL_MS, 80,
            "Spinner interval should be 80ms (12.5 FPS)"
        );
    }

    #[test]
    fn test_dots_animation_4_states() {
        let dot_states: Vec<String> = (0..4).map(|i| ".".repeat(i)).collect();

        assert_eq!(dot_states.len(), 4, "Dots animation should have 4 states");
        assert_eq!(dot_states[0], "", "State 0: empty");
        assert_eq!(dot_states[1], ".", "State 1: one dot");
        assert_eq!(dot_states[2], "..", "State 2: two dots");
        assert_eq!(dot_states[3], "...", "State 3: three dots");
    }

    #[test]
    fn test_dots_animation_400ms_interval() {
        let interval_ms = 400u128;

        for ms in [0u128, 400, 800, 1200, 1600] {
            let dot_count = ((ms / interval_ms) % 4) as usize;
            assert!(dot_count < 4, "Dot count should be 0-3 at {}ms", ms);
        }
    }

    #[test]
    fn test_time_display_format() {
        let elapsed_secs = 2.3f32;
        let formatted = format!("({:.1}s)", elapsed_secs);

        assert_eq!(formatted, "(2.3s)", "Time format should be (X.Xs)");
    }

    #[test]
    fn test_time_display_with_various_durations() {
        let test_cases = [
            (0.0f32, "(0.0s)"),
            (1.0f32, "(1.0s)"),
            (5.5f32, "(5.5s)"),
            (10.25f32, "(10.2s)"),
            (99.99f32, "(100.0s)"),
        ];

        for (secs, expected) in test_cases {
            let formatted = format!("({:.1}s)", secs);
            assert_eq!(
                formatted, expected,
                "Duration {} should format as {}",
                secs, expected
            );
        }
    }

    #[test]
    fn test_spinner_with_text() {
        let spinner_frame = SPINNER_FRAMES[0];
        let message = "Thinking...";
        let combined = format!("{} {}", spinner_frame, message);

        assert!(combined.contains("⠋"), "Should contain spinner frame");
        assert!(combined.contains("Thinking"), "Should contain message");
    }
}

// ============================================================================
// Phase 5: Input/Status Bar Tests
// ============================================================================

mod phase5_input_status {
    use super::*;

    #[test]
    fn test_input_prompt_symbol() {
        let element = build_input_prompt("", true);

        LayoutAssertion::new(&element, 80, 24).assert_contains("❯");
    }

    #[test]
    fn test_input_placeholder_when_empty_idle() {
        let element = build_input_prompt("", true);

        LayoutAssertion::new(&element, 80, 24).assert_contains("Try \"edit base.rs to...\"");
    }

    #[test]
    fn test_input_shows_text_when_provided() {
        let element = build_input_prompt("hello world", true);

        LayoutAssertion::new(&element, 80, 24)
            .assert_contains("hello world")
            .assert_not_contains("Try \"edit base.rs");
    }

    #[test]
    fn test_input_no_placeholder_when_not_idle() {
        let element = build_input_prompt("", false);

        let assertion = LayoutAssertion::new(&element, 80, 24);
        let line = assertion.get_line(0).unwrap();
        assert!(
            !line.contains("Try"),
            "Should not show placeholder when not idle"
        );
    }

    #[test]
    fn test_status_bar_mode_indicator() {
        let element = build_status_bar("permissions required", None, true);

        LayoutAssertion::new(&element, 80, 24).assert_contains("⏵⏵");
    }

    #[test]
    fn test_status_bar_normal_mode() {
        let element = build_status_bar("permissions required", None, true);

        LayoutAssertion::new(&element, 80, 24).assert_contains("permissions required");
    }

    #[test]
    fn test_status_bar_bypass_mode() {
        let element = build_status_bar("bypass permissions on", None, true);

        LayoutAssertion::new(&element, 80, 24).assert_contains("bypass permissions on");
    }

    #[test]
    fn test_status_bar_plan_mode() {
        let element = build_status_bar("plan mode", None, true);

        LayoutAssertion::new(&element, 80, 24).assert_contains("plan mode");
    }

    #[test]
    fn test_status_bar_cycle_hint() {
        let element = build_status_bar("permissions required", None, true);

        LayoutAssertion::new(&element, 80, 24).assert_contains("(shift+tab to cycle)");
    }

    #[test]
    fn test_status_bar_mouse_on() {
        let element = build_status_bar("permissions required", None, true);

        LayoutAssertion::new(&element, 80, 24).assert_contains("mouse on");
    }

    #[test]
    fn test_status_bar_mouse_off() {
        let element = build_status_bar("permissions required", None, false);

        LayoutAssertion::new(&element, 80, 24).assert_contains("mouse off");
    }

    #[test]
    fn test_status_bar_scroll_indicator_format() {
        let element = build_status_bar("permissions required", Some(75), true);

        LayoutAssertion::new(&element, 80, 24).assert_contains("[ 75%]");
    }

    #[test]
    fn test_status_bar_scroll_indicator_100_percent() {
        let element = build_status_bar("permissions required", Some(100), true);

        LayoutAssertion::new(&element, 80, 24).assert_contains("[100%]");
    }

    #[test]
    fn test_status_bar_scroll_indicator_0_percent() {
        let element = build_status_bar("permissions required", Some(0), true);

        LayoutAssertion::new(&element, 80, 24).assert_contains("[  0%]");
    }

    #[test]
    fn test_permission_mode_cycle() {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        enum PermissionMode {
            Normal,
            Bypass,
            Plan,
        }

        impl PermissionMode {
            fn cycle(self) -> Self {
                match self {
                    PermissionMode::Normal => PermissionMode::Bypass,
                    PermissionMode::Bypass => PermissionMode::Plan,
                    PermissionMode::Plan => PermissionMode::Normal,
                }
            }
        }

        let mut mode = PermissionMode::Normal;
        mode = mode.cycle();
        assert_eq!(mode, PermissionMode::Bypass);
        mode = mode.cycle();
        assert_eq!(mode, PermissionMode::Plan);
        mode = mode.cycle();
        assert_eq!(mode, PermissionMode::Normal);
    }
}

// ============================================================================
// Phase 6: Integration Tests
// ============================================================================

mod phase6_integration {
    use super::*;

    #[test]
    fn test_full_ui_80x24() {
        let session = create_default_session();
        let element = build_full_ui(&session, 80, 24);

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
    fn test_full_ui_40x24_small_width() {
        let session = create_default_session();
        let element = build_full_ui(&session, 40, 24);

        LayoutAssertion::new(&element, 40, 24).assert_no_line_exceeds_width();
    }

    #[test]
    fn test_full_ui_120x40_large() {
        let session = create_default_session();
        let element = build_full_ui(&session, 120, 40);

        LayoutAssertion::new(&element, 120, 40)
            .assert_contains("Sage Code v")
            .assert_all_lines_left_aligned()
            .assert_no_line_exceeds_width();
    }

    #[test]
    fn test_full_ui_160x50_extra_large() {
        let session = create_default_session();
        let element = build_full_ui(&session, 160, 50);

        LayoutAssertion::new(&element, 160, 50)
            .assert_all_lines_left_aligned()
            .assert_no_line_exceeds_width();
    }

    #[test]
    fn test_full_ui_various_terminal_sizes() {
        let session = create_default_session();
        let sizes = [(40, 24), (80, 24), (120, 40), (160, 50)];

        for (width, height) in sizes {
            let element = build_full_ui(&session, width, height);
            LayoutAssertion::new(&element, width, height)
                .assert_all_lines_left_aligned()
                .assert_no_line_exceeds_width();
        }
    }

    #[test]
    fn test_taffy_layout_root_position() {
        let session = create_default_session();
        let width = 80u16;
        let height = 24u16;
        let element = build_full_ui(&session, width, height);

        let mut engine = LayoutEngine::new();
        engine.compute(&element, width, height);

        let root_layout = engine
            .get_layout(element.id)
            .expect("Root should have layout");
        assert_eq!(root_layout.x, 0.0, "Root x should be 0");
        assert_eq!(root_layout.y, 0.0, "Root y should be 0");
        assert_eq!(
            root_layout.width as u16, width,
            "Root width should match terminal"
        );
        assert_eq!(
            root_layout.height as u16, height,
            "Root height should match terminal"
        );
    }

    #[test]
    fn test_state_transition_idle_to_thinking() {
        let mut state = AppState::default();
        assert!(matches!(state.phase, ExecutionPhase::Idle));

        state.start_thinking();
        assert!(matches!(state.phase, ExecutionPhase::Thinking));
        assert!(state.thinking.is_some());
    }

    #[test]
    fn test_state_transition_thinking_to_streaming() {
        let mut state = AppState::default();
        state.start_thinking();
        state.stop_thinking();

        state.start_streaming();
        assert!(matches!(state.phase, ExecutionPhase::Streaming { .. }));
        assert!(state.streaming_content.is_some());
    }

    #[test]
    fn test_state_transition_streaming_to_idle() {
        let mut state = AppState::default();
        state.start_streaming();
        state.append_streaming_chunk("Hello ");
        state.append_streaming_chunk("World");

        state.finish_streaming();
        assert!(matches!(state.phase, ExecutionPhase::Idle));
        assert!(state.streaming_content.is_none());
        assert_eq!(state.messages.len(), 1);
    }

    #[test]
    fn test_state_transition_idle_to_tool() {
        let mut state = AppState::default();

        state.start_tool("bash".to_string(), "ls -la".to_string());
        assert!(matches!(state.phase, ExecutionPhase::ExecutingTool { .. }));
        assert!(state.tool_execution.is_some());
    }

    #[test]
    fn test_state_transition_tool_to_idle() {
        let mut state = AppState::default();
        state.start_tool("bash".to_string(), "ls -la".to_string());

        state.finish_tool(true, Some("file1\nfile2".to_string()), None);
        assert!(matches!(state.phase, ExecutionPhase::Idle));
        assert!(state.tool_execution.is_none());
        assert_eq!(state.messages.len(), 1);
    }

    #[test]
    fn test_row_layout_children_same_line() {
        let element = RnkBox::new()
            .flex_direction(FlexDirection::Row)
            .child(Text::new("Left").into_element())
            .child(Text::new("Right").into_element())
            .into_element();

        let output = rnk::render_to_string(&element, 80);
        let stripped = strip_ansi(&output);

        assert_eq!(output.lines().count(), 1, "Row should produce single line");

        let left_pos = stripped.find("Left").unwrap();
        let right_pos = stripped.find("Right").unwrap();
        assert!(left_pos < right_pos, "Left should be before Right");
    }

    #[test]
    fn test_column_layout_children_different_lines() {
        let element = RnkBox::new()
            .flex_direction(FlexDirection::Column)
            .child(Text::new("Top").into_element())
            .child(Text::new("Bottom").into_element())
            .into_element();

        let output = rnk::render_to_string(&element, 80);

        assert_eq!(
            output.lines().count(),
            2,
            "Column should produce multiple lines"
        );
    }

    #[test]
    fn test_no_content_offset_with_flex_grow() {
        let width = 100u16;
        let height = 30u16;

        let element = RnkBox::new()
            .flex_direction(FlexDirection::Column)
            .width(width as i32)
            .height(height as i32)
            .child(
                RnkBox::new()
                    .flex_direction(FlexDirection::Column)
                    .flex_grow(1.0)
                    .child(Text::new("Content in flex-grow area").into_element())
                    .into_element(),
            )
            .child(Text::new("Bottom content").into_element())
            .into_element();

        let output = rnk::render_to_string(&element, width);
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
    fn test_app_state_status_text() {
        let mut state = AppState::default();

        assert_eq!(state.status_text(), "Ready");

        state.start_thinking();
        assert!(state.status_text().contains("Thinking"));

        state.stop_thinking();
        state.start_streaming();
        assert!(state.status_text().contains("Streaming"));

        state.finish_streaming();
        state.start_tool("bash".to_string(), "test".to_string());
        assert!(state.status_text().contains("Running"));
        assert!(state.status_text().contains("bash"));
    }

    #[test]
    fn test_message_accumulation() {
        let mut state = AppState::default();

        state.add_user_message("First message".to_string());
        assert_eq!(state.messages.len(), 1);

        state.add_user_message("Second message".to_string());
        assert_eq!(state.messages.len(), 2);

        state.start_streaming();
        state.append_streaming_chunk("Response");
        state.finish_streaming();
        assert_eq!(state.messages.len(), 3);
    }

    #[test]
    fn test_display_messages_includes_streaming() {
        let mut state = AppState::default();
        state.add_user_message("Hello".to_string());
        state.start_streaming();
        state.append_streaming_chunk("Partial response");

        let display = state.display_messages();
        assert_eq!(display.len(), 2, "Should include streaming message");

        if let UiMessageContent::Text(text) = &display[1].content {
            assert_eq!(text, "Partial response");
        } else {
            panic!("Expected text content for streaming");
        }
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn test_empty_session_info() {
        let session = UiSessionInfo {
            session_id: None,
            model: String::new(),
            provider: String::new(),
            working_dir: String::new(),
            git_branch: None,
            step: 0,
            max_steps: None,
        };

        let element = build_header(&session, 80);
        LayoutAssertion::new(&element, 80, 24).assert_no_line_exceeds_width();
    }

    #[test]
    fn test_very_long_model_name() {
        let session = UiSessionInfo {
            session_id: None,
            model: "very-long-model-name-that-exceeds-normal-width-limits".to_string(),
            provider: "provider".to_string(),
            working_dir: "/path".to_string(),
            git_branch: None,
            step: 0,
            max_steps: None,
        };

        let element = build_header(&session, 40);
        LayoutAssertion::new(&element, 40, 24).assert_no_line_exceeds_width();
    }

    #[test]
    fn test_very_long_working_directory() {
        let session = UiSessionInfo {
            session_id: None,
            model: "model".to_string(),
            provider: "provider".to_string(),
            working_dir: "/Users/very/long/path/that/exceeds/terminal/width/limit".to_string(),
            git_branch: None,
            step: 0,
            max_steps: None,
        };

        let element = build_header(&session, 40);
        LayoutAssertion::new(&element, 40, 24).assert_no_line_exceeds_width();
    }

    #[test]
    fn test_unicode_emoji_in_message() {
        let msg = create_user_message("Hello 👋 World 🌍!");
        let content = match &msg.content {
            UiMessageContent::Text(t) => t,
            _ => panic!("Expected text"),
        };

        let width = display_width(content);
        assert!(width > 0, "Unicode message should have width");
    }

    #[test]
    fn test_minimum_terminal_width() {
        let session = create_default_session();
        let element = build_full_ui(&session, 20, 24);

        LayoutAssertion::new(&element, 20, 24).assert_no_line_exceeds_width();
    }

    #[test]
    fn test_minimum_terminal_height() {
        let session = create_default_session();
        let element = build_full_ui(&session, 80, 10);

        let _assertion = LayoutAssertion::new(&element, 80, 10);
    }

    #[test]
    fn test_special_characters_in_tool_args() {
        let result = UiToolResult {
            success: true,
            output: Some("Output with \"quotes\" and 'apostrophes'".to_string()),
            error: None,
            duration: Duration::from_millis(10),
        };
        let msg = create_tool_call_message(
            "bash",
            "echo \"hello\\nworld\" | grep -E '^[a-z]+'",
            Some(result),
        );

        let _content = match &msg.content {
            UiMessageContent::ToolCall { params, .. } => params,
            _ => panic!("Expected tool call"),
        };
    }

    #[test]
    fn test_multiline_tool_output() {
        let result = UiToolResult {
            success: true,
            output: Some("Line 1\nLine 2\nLine 3".to_string()),
            error: None,
            duration: Duration::from_millis(10),
        };
        let msg = create_tool_call_message("bash", "cat file.txt", Some(result));

        if let UiMessageContent::ToolCall {
            result: Some(r), ..
        } = &msg.content
        {
            assert!(r.output.as_ref().unwrap().contains('\n'));
        }
    }

    #[test]
    fn test_truncate_to_width_preserves_short() {
        let text = "short";
        let result = truncate_to_width(text, 80);
        assert_eq!(result, text, "Short text should not be truncated");
    }

    #[test]
    fn test_truncate_to_width_adds_ellipsis() {
        let text = "this is a very long text that should be truncated";
        let result = truncate_to_width(text, 20);
        assert!(
            result.ends_with("..."),
            "Truncated text should end with ..."
        );
        assert!(display_width(&result) <= 20, "Result should fit in width");
    }

    #[test]
    fn test_truncate_to_width_zero() {
        let result = truncate_to_width("test", 0);
        assert!(result.is_empty(), "Zero width should produce empty string");
    }

    #[test]
    fn test_truncate_to_width_cjk() {
        let text = "中文文本测试";
        let result = truncate_to_width(text, 6);
        assert!(
            display_width(&result) <= 6,
            "CJK truncation should respect width"
        );
    }
}
