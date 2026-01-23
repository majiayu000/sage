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
            match msg.role {
                Role::User => {
                    // User message: simple "> " prefix
                    let mut container = RnkBox::new().flex_direction(FlexDirection::Column);
                    let lines = wrap_text_with_prefix("> ", text, term_width);
                    for line in lines {
                        container = container.child(
                            Text::new(line).color(Color::White).into_element()
                        );
                    }
                    container.into_element()
                }
                Role::Assistant => {
                    // Assistant message: no prefix, just content
                    let mut container = RnkBox::new().flex_direction(FlexDirection::Column);
                    for line in text.lines() {
                        let wrapped = wrap_text_with_prefix("", line, term_width);
                        for w in wrapped {
                            container = container.child(
                                Text::new(w).color(Color::White).into_element()
                            );
                        }
                    }
                    container.into_element()
                }
                Role::System => {
                    // System message: dim italic
                    Text::new(truncate_to_width(text, term_width))
                        .color(Color::BrightBlack)
                        .italic()
                        .into_element()
                }
            }
        }
        MessageContent::Thinking(text) => {
            let preview: String = text.lines().take(3).collect::<Vec<_>>().join(" ");
            Text::new(format!(
                "💭 {}...",
                truncate_to_width(&preview, term_width.saturating_sub(5))
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

            // Tool header with icon
            let icon = get_tool_icon(tool_name);
            container = container.child(
                Text::new(format!("{} {}", icon, tool_name))
                    .color(Color::Magenta)
                    .bold()
                    .into_element(),
            );

            // Params (truncated)
            if !params.trim().is_empty() {
                let param_preview = truncate_to_width(params.trim(), term_width.saturating_sub(4));
                container = container.child(
                    Text::new(format!("  ⎿ {}", param_preview))
                        .color(Color::BrightBlack)
                        .into_element()
                );
            }

            // Result
            if let Some(r) = result {
                if r.success {
                    if let Some(ref output) = r.output {
                        let preview = truncate_to_width(output.lines().next().unwrap_or(""), term_width.saturating_sub(4));
                        if !preview.is_empty() {
                            container = container.child(
                                Text::new(format!("  ✓ {}", preview))
                                    .color(Color::Green)
                                    .into_element()
                            );
                        }
                    }
                } else {
                    let error_msg = r.error.as_deref().unwrap_or("Unknown error");
                    let preview = truncate_to_width(error_msg, term_width.saturating_sub(4));
                    container = container.child(
                        Text::new(format!("  ✗ {}", preview))
                            .color(Color::Red)
                            .into_element()
                    );
                }
            }

            container.into_element()
        }
    }
}

/// Get icon for tool name
fn get_tool_icon(tool_name: &str) -> &'static str {
    match tool_name {
        "bash" | "Bash" => "⚡",
        "read" | "Read" => "📄",
        "write" | "Write" => "✏️",
        "edit" | "Edit" => "📝",
        "glob" | "Glob" => "🔍",
        "grep" | "Grep" => "🔎",
        "task" | "Task" => "🤖",
        "web_fetch" | "WebFetch" => "🌐",
        _ => "●",
    }
}

/// Render error message
pub fn render_error(message: &str) -> Element {
    let term_width = terminal::size().map(|(w, _)| w as usize).unwrap_or(80);

    let mut container = RnkBox::new().flex_direction(FlexDirection::Column);

    // Error header
    container = container.child(
        Text::new("✗ Error")
            .color(Color::Red)
            .bold()
            .into_element()
    );

    // Error message (wrapped)
    let lines = wrap_text_with_prefix("  ", message, term_width);
    for line in lines {
        container = container.child(
            Text::new(line)
                .color(Color::Red)
                .into_element()
        );
    }

    container.into_element()
}

/// Render header banner
pub fn render_header(session: &SessionState) -> Element {
    let version = env!("CARGO_PKG_VERSION");
    let term_width = terminal::size().map(|(w, _)| w as usize).unwrap_or(80);

    // Simpler, cleaner header
    let title = format!("╭─ Sage Code v{}", version);
    let model_info = format!("│  {} · {}", session.model, session.provider);
    let cwd = truncate_to_width(&session.working_dir, term_width.saturating_sub(5));
    let cwd_line = format!("│  {}", cwd);
    let bottom = "╰─────────────────────────────────";

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new(truncate_to_width(&title, term_width))
                .color(Color::Cyan)
                .bold()
                .into_element(),
        )
        .child(
            Text::new(truncate_to_width(&model_info, term_width))
                .color(Color::Blue)
                .into_element(),
        )
        .child(
            Text::new(truncate_to_width(&cwd_line, term_width))
                .color(Color::BrightBlack)
                .into_element(),
        )
        .child(
            Text::new(truncate_to_width(bottom, term_width))
                .color(Color::BrightBlack)
                .dim()
                .into_element(),
        )
        .into_element()
}

/// Render input line
pub fn render_input(input_text: &str) -> Element {
    let hints = [
        "edit main.rs to add error handling",
        "explain this function",
        "write tests for auth module",
        "fix the bug in line 42",
        "refactor to use async/await",
    ];

    // Rotate hints based on time
    let hint_idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() / 5) as usize % hints.len();

    let display_text = if input_text.is_empty() {
        hints[hint_idx]
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
            Text::new(format!(" {}", status_text))
                .color(Color::Yellow)
                .into_element(),
        )
        .child(
            Text::new(" (ESC to cancel)")
                .color(Color::BrightBlack)
                .dim()
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
            Text::new(" · shift+tab to cycle")
                .color(Color::BrightBlack)
                .dim()
                .into_element(),
        )
        .into_element()
}

/// Built-in commands for suggestions
const BUILTIN_COMMANDS: &[(&str, &str)] = &[
    ("help", "Show help information"),
    ("clear", "Clear conversation"),
    ("compact", "Compact conversation context"),
    ("commands", "List slash commands"),
    ("config", "Manage configuration"),
    ("context", "Show context usage"),
    ("cost", "Show session cost and usage"),
    ("init", "Initialize Sage in project"),
    ("login", "Configure API credentials"),
    ("output", "Switch output mode (streaming|batch|silent)"),
    ("plan", "View execution plan"),
    ("resume", "Resume a conversation"),
    ("status", "Show agent status"),
    ("tasks", "List background tasks"),
    ("title", "Set session title"),
    ("undo", "Undo file changes"),
];

/// Render command suggestions when input starts with /
/// Returns (Element, match_count) so caller can clamp selection index
pub fn render_command_suggestions(input: &str, selected_index: usize) -> Option<(Element, usize)> {
    if !input.starts_with('/') {
        return None;
    }

    let query = &input[1..]; // Remove leading /

    // Filter commands that match the query
    let matches: Vec<_> = BUILTIN_COMMANDS
        .iter()
        .filter(|(name, _)| name.starts_with(query))
        .take(6) // Show max 6 suggestions
        .collect();

    if matches.is_empty() {
        return None;
    }

    let match_count = matches.len();
    let selected = selected_index.min(match_count.saturating_sub(1));

    let mut container = RnkBox::new().flex_direction(FlexDirection::Column);

    // Command list - each command on its own line
    for (i, (name, desc)) in matches.iter().enumerate() {
        let is_selected = i == selected;
        let prefix = if is_selected { "▸ " } else { "  " };
        let cmd_color = if is_selected { Color::White } else { Color::Cyan };
        let desc_color = if is_selected { Color::White } else { Color::BrightBlack };

        let row = RnkBox::new()
            .flex_direction(FlexDirection::Row)
            .child(
                Text::new(format!("{}/{}", prefix, name))
                    .color(cmd_color)
                    .bold()
                    .into_element(),
            )
            .child(
                Text::new(format!(" - {}", desc))
                    .color(desc_color)
                    .into_element(),
            );

        container = container.child(row.into_element());
    }

    Some((container.into_element(), match_count))
}

/// Get the selected command name based on input and index
pub fn get_selected_command(input: &str, selected_index: usize) -> Option<String> {
    if !input.starts_with('/') {
        return None;
    }

    let query = &input[1..];
    let matches: Vec<_> = BUILTIN_COMMANDS
        .iter()
        .filter(|(name, _)| name.starts_with(query))
        .take(6)
        .collect();

    if matches.is_empty() {
        return None;
    }

    let selected = selected_index.min(matches.len().saturating_sub(1));
    Some(format!("/{}", matches[selected].0))
}

/// Count matching commands for suggestion index clamping
/// Used by input handler to clamp index before render
pub fn count_matching_commands(input: &str) -> usize {
    if !input.starts_with('/') {
        return 0;
    }

    let query = &input[1..];
    BUILTIN_COMMANDS
        .iter()
        .filter(|(name, _)| name.starts_with(query))
        .take(6)
        .count()
}
