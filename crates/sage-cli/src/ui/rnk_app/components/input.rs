//! Input line component

use rnk::prelude::*;

use crate::ui::rnk_app::theme::Theme;

// Alias rnk's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as RnkBox;

/// Placeholder hints for empty input
const HINTS: &[&str] = &[
    "edit main.rs to add error handling",
    "explain this function",
    "write tests for auth module",
    "fix the bug in line 42",
    "refactor to use async/await",
];

/// Render input line with cursor animation
pub fn render_input(input_text: &str, theme: &Theme, animation_frame: usize) -> Element {
    let hint_idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 5) as usize
        % HINTS.len();

    let is_empty = input_text.is_empty();
    let display_text = if is_empty {
        HINTS[hint_idx]
    } else {
        input_text
    };

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
