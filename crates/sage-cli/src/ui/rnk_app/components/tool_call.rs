//! Tool call display component

use rnk::prelude::*;
use sage_core::ui::bridge::state::UiToolResult;

use crate::ui::rnk_app::formatting::truncate_to_width;
use crate::ui::rnk_app::theme::Theme;

// Alias rnk's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as RnkBox;

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

/// Render a tool call with optional result
pub fn render_tool_call(
    tool_name: &str,
    params: &str,
    result: Option<&UiToolResult>,
    theme: &Theme,
) -> Element {
    let term_width = crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80);
    let icon = get_tool_icon(tool_name);

    let mut col = RnkBox::new().flex_direction(FlexDirection::Column);

    // Top border - extends to terminal width
    let border_line = "─".repeat(term_width.saturating_sub(2));
    col = col.child(
        Text::new(format!("╭{}", border_line))
            .color(theme.border_subtle)
            .into_element(),
    );

    // Header with tool name
    col = col.child(
        RnkBox::new()
            .flex_direction(FlexDirection::Row)
            .child(Text::new("│ ").color(theme.border_subtle).into_element())
            .child(
                Text::new(format!("{} ", icon))
                    .color(theme.tool)
                    .into_element(),
            )
            .child(Text::new(tool_name).color(theme.tool).bold().into_element())
            .into_element(),
    );

    // Params preview with tree-style prefix
    if !params.trim().is_empty() {
        let preview = truncate_to_width(params.trim(), term_width.saturating_sub(8));
        col = col.child(
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("│ ").color(theme.border_subtle).into_element())
                .child(Text::new("╰─ ").color(theme.border_subtle).into_element())
                .child(Text::new(preview).color(theme.tool_param).into_element())
                .into_element(),
        );
    }

    // Result
    if let Some(r) = result {
        if r.success {
            let out = r
                .output
                .as_deref()
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("");
            let preview = truncate_to_width(out, term_width.saturating_sub(8));
            if !preview.is_empty() {
                col = col.child(
                    RnkBox::new()
                        .flex_direction(FlexDirection::Row)
                        .child(Text::new("│ ").color(theme.border_subtle).into_element())
                        .child(Text::new("✓ ").color(theme.ok).into_element())
                        .child(Text::new(preview).color(theme.text_muted).into_element())
                        .into_element(),
                );
            }
        } else {
            let err = r.error.as_deref().unwrap_or("Unknown error");
            let preview = truncate_to_width(err, term_width.saturating_sub(8));
            col = col.child(
                RnkBox::new()
                    .flex_direction(FlexDirection::Row)
                    .child(Text::new("│ ").color(theme.border_subtle).into_element())
                    .child(Text::new("✗ ").color(theme.err).into_element())
                    .child(Text::new(preview).color(theme.err).into_element())
                    .into_element(),
            );
        }
    }

    // Bottom border
    col = col.child(
        Text::new(format!("╰{}", border_line))
            .color(theme.border_subtle)
            .into_element(),
    );

    col.into_element()
}

/// Format tool execution start for printing
pub fn format_tool_start(tool_name: &str, description: &str, theme: &Theme) -> Element {
    render_tool_call(tool_name, description, None, theme)
}
