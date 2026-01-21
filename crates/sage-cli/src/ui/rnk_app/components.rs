//! UI components for rnk app

use crossterm::terminal;
use rnk::prelude::*;
use sage_core::ui::bridge::state::{Message, MessageContent, Role, SessionState};

use super::formatting::{truncate_to_width, wrap_text_with_prefix};
use super::state::PermissionMode;

// Alias rnk's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as RnkBox;

/// Format a message for printing via rnk::println
pub fn format_message(msg: &Message) -> Element {
    let term_width = terminal::size().map(|(w, _)| w as usize).unwrap_or(80);

    match &msg.content {
        MessageContent::Text(text) => {
            let (prefix, color) = match msg.role {
                Role::User => ("user: ", Color::Blue),
                Role::Assistant => ("assistant: ", Color::Green),
                Role::System => ("system: ", Color::Cyan),
            };

            let mut container = RnkBox::new().flex_direction(FlexDirection::Column);
            let lines = wrap_text_with_prefix(prefix, text, term_width);

            for (i, line) in lines.iter().enumerate() {
                let text_elem = if i == 0 {
                    Text::new(line.as_str()).color(color).bold()
                } else {
                    Text::new(line.as_str()).color(color)
                };
                container = container.child(text_elem.into_element());
            }

            container.into_element()
        }
        MessageContent::Thinking(text) => {
            let preview: String = text.lines().take(3).collect::<Vec<_>>().join(" ");
            Text::new(format!(
                "thinking: {}...",
                truncate_to_width(&preview, term_width.saturating_sub(12))
            ))
            .color(Color::BrightBlack)
            .italic()
            .into_element()
        }
        MessageContent::ToolCall {
            tool_name,
            params,
            result,
        } => {
            let mut container = RnkBox::new().flex_direction(FlexDirection::Column);

            // Tool header
            container = container.child(
                Text::new(format!("● {}", tool_name))
                    .color(Color::Magenta)
                    .bold()
                    .into_element(),
            );

            // Params
            if !params.trim().is_empty() {
                let param_lines = wrap_text_with_prefix("  args: ", params, term_width);
                for line in param_lines {
                    container =
                        container.child(Text::new(line).color(Color::Magenta).into_element());
                }
            }

            // Result
            if let Some(r) = result {
                let (label, color, content) = if r.success {
                    ("  ⎿ ", Color::Ansi256(245), r.output.as_deref().unwrap_or(""))
                } else {
                    (
                        "  ✗ ",
                        Color::Red,
                        r.error.as_deref().unwrap_or("Unknown error"),
                    )
                };
                if !content.is_empty() {
                    let result_lines = wrap_text_with_prefix(label, content, term_width);
                    for line in result_lines {
                        container = container.child(Text::new(line).color(color).into_element());
                    }
                }
            }

            container.into_element()
        }
    }
}

/// Render header banner
pub fn render_header(session: &SessionState) -> Element {
    let version = env!("CARGO_PKG_VERSION");
    let term_width = terminal::size().map(|(w, _)| w as usize).unwrap_or(80);

    let title = format!("▐▛███▜▌   Sage Code v{}", version);
    let model_info = format!("{} · {}", session.model, session.provider);
    let model_line = format!("▝▜█████▛▘  {}", model_info);
    let cwd_line = format!("  ▘▘ ▝▝    {}", session.working_dir);
    let hint_line = "  /model to try another model";

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new(truncate_to_width(&title, term_width))
                .color(Color::Cyan)
                .bold()
                .into_element(),
        )
        .child(
            Text::new(truncate_to_width(&model_line, term_width))
                .color(Color::Blue)
                .into_element(),
        )
        .child(
            Text::new(truncate_to_width(&cwd_line, term_width))
                .color(Color::BrightBlack)
                .into_element(),
        )
        .child(Newline::new().into_element())
        .child(
            Text::new(truncate_to_width(hint_line, term_width))
                .color(Color::BrightBlack)
                .into_element(),
        )
        .into_element()
}

/// Render input line
pub fn render_input(input_text: &str) -> Element {
    let display_text = if input_text.is_empty() {
        "Try \"edit base.rs to...\""
    } else {
        input_text
    };
    let text_color = if input_text.is_empty() {
        Color::BrightBlack
    } else {
        Color::White
    };

    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("❯ ").color(Color::Green).bold().into_element())
        .child(Text::new(display_text).color(text_color).into_element())
        .into_element()
}

/// Render spinner line
pub fn render_spinner(status_text: &str) -> Element {
    // Use time-based frame selection for animation
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner = spinner_frames[(now_ms / 80 % spinner_frames.len() as u128) as usize];

    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new(spinner).color(Color::Yellow).into_element())
        .child(
            Text::new(format!(" {} (ESC to cancel)", status_text))
                .color(Color::Yellow)
                .into_element(),
        )
        .into_element()
}

/// Render status bar
pub fn render_status_bar(permission_mode: PermissionMode) -> Element {
    let mode_color = permission_mode.color();
    let mode_text = permission_mode.display_text();

    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("⏵⏵ ").color(mode_color).into_element())
        .child(Text::new(mode_text).color(mode_color).into_element())
        .child(
            Text::new(" (shift+tab to cycle)")
                .color(Color::BrightBlack)
                .into_element(),
        )
        .into_element()
}
