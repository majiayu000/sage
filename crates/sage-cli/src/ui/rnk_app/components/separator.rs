//! Separator line component

use rnk::prelude::*;

use crate::ui::rnk_app::theme::Theme;

/// Render a horizontal separator line
pub fn render_separator(width: usize, theme: &Theme) -> Element {
    let line = "─".repeat(width);
    Text::new(line)
        .color(theme.border_subtle)
        .dim()
        .into_element()
}
