//! Command suggestions component

use rnk::prelude::*;

use crate::ui::rnk_app::theme::Theme;

// Alias rnk's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as RnkBox;

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
            .child(
                Text::new(format!("  {desc}"))
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
