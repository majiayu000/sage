//! Model selector component - similar to command suggestions

use rnk::prelude::*;

use crate::ui::rnk_app::theme::Theme;

// Alias rnk's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as RnkBox;

/// Maximum number of models to display at once
const MAX_VISIBLE: usize = 6;

/// Render model selector when in model selection mode
pub fn render_model_selector(
    models: &[String],
    selected_index: usize,
    theme: &Theme,
) -> Element {
    let match_count = models.len();
    let selected = selected_index.min(match_count.saturating_sub(1));

    // Calculate visible window (scroll to keep selected item visible)
    let (start, end) = if match_count <= MAX_VISIBLE {
        (0, match_count)
    } else {
        let half = MAX_VISIBLE / 2;
        let start = selected.saturating_sub(half);
        let end = (start + MAX_VISIBLE).min(match_count);
        let start = if end == match_count {
            match_count.saturating_sub(MAX_VISIBLE)
        } else {
            start
        };
        (start, end)
    };

    let mut container = RnkBox::new().flex_direction(FlexDirection::Column);

    // Header with scroll indicator
    let header = if match_count > MAX_VISIBLE {
        format!("⌘ select model ({}/{}) - ↑↓ navigate, Enter select, Esc cancel", selected + 1, match_count)
    } else {
        "⌘ select model - ↑↓ navigate, Enter select, Esc cancel".to_string()
    };
    container = container.child(
        Text::new(header)
            .color(theme.text_muted)
            .bold()
            .into_element(),
    );

    // Show scroll up indicator
    if start > 0 {
        container = container.child(
            Text::new("  ↑ more above")
                .color(theme.text_subtle)
                .dim()
                .into_element(),
        );
    }

    for (i, model) in models.iter().enumerate().skip(start).take(end - start) {
        let is_selected = i == selected;
        let prefix = if is_selected { "▸" } else { " " };
        let model_color = if is_selected {
            theme.accent_user
        } else {
            theme.accent_assistant
        };

        let row = RnkBox::new()
            .flex_direction(FlexDirection::Row)
            .child(
                Text::new(format!("{} {}", prefix, model))
                    .color(model_color)
                    .bold()
                    .into_element(),
            );

        container = container.child(row.into_element());
    }

    // Show scroll down indicator
    if end < match_count {
        container = container.child(
            Text::new("  ↓ more below")
                .color(theme.text_subtle)
                .dim()
                .into_element(),
        );
    }

    container.into_element()
}
