//! UI components for rnk app

use crossterm::terminal;
use rnk::prelude::*;
use sage_core::ui::bridge::state::{Message, MessageContent, Role, SessionState, ToolResult};

use super::formatting::{truncate_to_width, wrap_text_with_prefix};
use super::state::PermissionMode;
use super::theme::Theme;

// Alias rnk's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as RnkBox;

fn role_style(role: &Role, theme: &Theme) -> (&'static str, Color) {
    match role {
        Role::User => ("👤", theme.accent_user),
        Role::Assistant => ("🤖", theme.accent_assistant),
        Role::System => ("⚙", theme.accent_system),
    }
}

fn gutter_line(icon: &str, gutter_color: Color, text: String, text_color: Color) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("│ ").color(gutter_color).bold().into_element())
        .child(Text::new(format!("{icon} ")).color(gutter_color).bold().into_element())
        .child(Text::new(text).color(text_color).into_element())
        .into_element()
}

/// Format a message for printing via rnk::println
pub fn format_message(msg: &Message, theme: &Theme) -> Element {
    let term_width = terminal::size().map(|(w, _)| w as usize).unwrap_or(80);
    let (icon, gutter_color) = role_style(&msg.role, theme);

    match &msg.content {
        MessageContent::Text(text) => {
            let mut container = RnkBox::new().flex_direction(FlexDirection::Column);

            match msg.role {
                Role::User | Role::Assistant => {
                    for paragraph in text.split('\n') {
                        let wrapped =
                            wrap_text_with_prefix("", paragraph, term_width.saturating_sub(4));
                        for line in wrapped {
                            container = container.child(gutter_line(
                                icon,
                                gutter_color,
                                line,
                                theme.text_primary,
                            ));
                        }
                        container = container.child(Text::new("").into_element());
                    }
                }
                Role::System => {
                    let sys_text = truncate_to_width(text, term_width.saturating_sub(4));
                    container =
                        container.child(gutter_line(icon, gutter_color, sys_text, theme.text_muted));
                }
            }

            container.into_element()
        }
        MessageContent::Thinking(text) => {
            let preview: String = text.lines().take(2).collect::<Vec<_>>().join(" ");
            gutter_line(
                "💭",
                theme.text_muted,
                format!(
                    "{}…",
                    truncate_to_width(&preview, term_width.saturating_sub(8))
                ),
                theme.text_muted,
            )
        }
        MessageContent::ToolCall {
            tool_name,
            params,
            result,
        } => render_tool_call(tool_name, params, result.as_ref(), theme),
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

fn render_tool_call(
    tool_name: &str,
    params: &str,
    result: Option<&ToolResult>,
    theme: &Theme,
) -> Element {
    let term_width = terminal::size().map(|(w, _)| w as usize).unwrap_or(80);
    let icon = get_tool_icon(tool_name);

    let mut col = RnkBox::new().flex_direction(FlexDirection::Column);

    // Header
    col = col.child(
        Text::new(format!("╭─ {icon} {tool_name}"))
            .color(theme.tool)
            .bold()
            .into_element(),
    );

    // Params preview
    if !params.trim().is_empty() {
        let preview = truncate_to_width(params.trim(), term_width.saturating_sub(6));
        col = col.child(
            Text::new(format!("│  ⤷ {preview}"))
                .color(theme.tool_param)
                .into_element(),
        );
    }

    // Result
    if let Some(r) = result {
        if r.success {
            let out = r.output.as_deref().unwrap_or("").lines().next().unwrap_or("");
            let preview = truncate_to_width(out, term_width.saturating_sub(8));
            if !preview.is_empty() {
                col = col.child(
                    Text::new(format!("│  ✓ {preview}"))
                        .color(theme.ok)
                        .into_element(),
                );
            }
        } else {
            let err = r.error.as_deref().unwrap_or("Unknown error");
            let preview = truncate_to_width(err, term_width.saturating_sub(8));
            col = col.child(
                Text::new(format!("│  ✗ {preview}"))
                    .color(theme.err)
                    .into_element(),
            );
        }
    }

    // Footer
    col = col.child(Text::new("╰─").color(theme.border).into_element());

    col.into_element()
}

/// Format tool execution start for printing
pub fn format_tool_start(tool_name: &str, description: &str, theme: &Theme) -> Element {
    render_tool_call(tool_name, description, None, theme)
}

/// Render error message
pub fn render_error(message: &str, theme: &Theme) -> Element {
    let term_width = terminal::size().map(|(w, _)| w as usize).unwrap_or(80);

    let mut container = RnkBox::new().flex_direction(FlexDirection::Column);

    container = container.child(
        Text::new("✗ Error")
            .color(theme.err)
            .bold()
            .into_element(),
    );

    let lines = wrap_text_with_prefix("  ", message, term_width);
    for line in lines {
        container = container.child(Text::new(line).color(theme.err).into_element());
    }

    container.into_element()
}

/// Render header banner - compact single-line style
pub fn render_header(session: &SessionState, theme: &Theme) -> Element {
    let version = env!("CARGO_PKG_VERSION");
    let term_width = terminal::size().map(|(w, _)| w as usize).unwrap_or(80);

    let header_text = format!("sage v{} · {} · {}", version, session.model, session.provider);

    let max_cwd_len = term_width.saturating_sub(4);
    let cwd_display = if session.working_dir.len() > max_cwd_len {
        format!(
            "...{}",
            &session.working_dir[session
                .working_dir
                .len()
                .saturating_sub(max_cwd_len.saturating_sub(3))..]
        )
    } else {
        session.working_dir.clone()
    };

    RnkBox::new()
        .flex_direction(FlexDirection::Column)
        .child(
            Text::new(truncate_to_width(&header_text, term_width))
                .color(theme.accent_assistant)
                .bold()
                .into_element(),
        )
        .child(
            Text::new(truncate_to_width(&cwd_display, term_width))
                .color(theme.text_muted)
                .into_element(),
        )
        .into_element()
}

/// Render input line
pub fn render_input(input_text: &str, theme: &Theme, animation_frame: usize) -> Element {
    let hints = [
        "edit main.rs to add error handling",
        "explain this function",
        "write tests for auth module",
        "fix the bug in line 42",
        "refactor to use async/await",
    ];

    let hint_idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 5) as usize
        % hints.len();

    let is_empty = input_text.is_empty();
    let display_text = if is_empty { hints[hint_idx] } else { input_text };

    let caret_frames = ["▏", "▎", "▍", "▋"];
    let caret = caret_frames[animation_frame % caret_frames.len()];

    let text_color = if is_empty {
        theme.text_muted
    } else {
        theme.text_primary
    };

    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(
            Text::new("❯ ")
                .color(theme.accent_user)
                .bold()
                .into_element(),
        )
        .child(Text::new(caret).color(theme.accent_user).into_element())
        .child(Text::new(" ").into_element())
        .child(Text::new(display_text).color(text_color).into_element())
        .into_element()
}

/// Render spinner line
pub fn render_spinner(status_text: &str) -> Element {
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

/// Render thinking indicator above separator (Claude Code style)
pub fn render_thinking_indicator(status_text: &str, animation_frame: usize, theme: &Theme) -> Element {
    let dots = match animation_frame % 4 {
        0 => "·",
        1 => "··",
        2 => "···",
        _ => "····",
    };

    let display_text = if status_text.is_empty() {
        format!("Thinking{dots}")
    } else {
        format!("{status_text} {dots}")
    };

    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("⏺ ").color(theme.warn).bold().into_element())
        .child(Text::new(display_text).color(theme.warn).into_element())
        .child(
            Text::new("  Esc to interrupt")
                .color(theme.text_muted)
                .dim()
                .into_element(),
        )
        .into_element()
}

/// Render status bar
pub fn render_status_bar(permission_mode: PermissionMode, theme: &Theme) -> Element {
    let (mode_color, mode_label, mode_icon) = match permission_mode {
        PermissionMode::Normal => (theme.status_normal, "permissions required", "🔒"),
        PermissionMode::Bypass => (theme.status_bypass, "bypass permissions", "⚠"),
        PermissionMode::Plan => (theme.status_plan, "plan mode", "🧭"),
    };

    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new(" ").into_element())
        .child(
            Text::new(format!("{mode_icon} {mode_label}"))
                .color(mode_color)
                .bold()
                .into_element(),
        )
        .child(
            Text::new("  ·  ⇧Tab cycle  ·  / commands")
                .color(theme.text_muted)
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
pub fn render_command_suggestions(
    input: &str,
    selected_index: usize,
    theme: &Theme,
) -> Option<(Element, usize)> {
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

    let match_count = matches.len();
    let selected = selected_index.min(match_count.saturating_sub(1));

    let mut container = RnkBox::new().flex_direction(FlexDirection::Column);

    container = container.child(
        Text::new("⌘ commands")
            .color(theme.text_muted)
            .bold()
            .into_element(),
    );

    for (i, (name, desc)) in matches.iter().enumerate() {
        let is_selected = i == selected;
        let prefix = if is_selected { "▸" } else { " " };
        let cmd_color = if is_selected {
            theme.text_primary
        } else {
            theme.accent_assistant
        };
        let desc_color = theme.text_muted;

        let row = RnkBox::new()
            .flex_direction(FlexDirection::Row)
            .child(
                Text::new(format!("{prefix} /{name}"))
                    .color(cmd_color)
                    .bold()
                    .into_element(),
            )
            .child(Text::new(format!("  {desc}")).color(desc_color).into_element());

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
