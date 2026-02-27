//! Separator line component

use rnk::prelude::*;

use crate::ui::rnk_app::theme::TerminalTheme;

/// Render a horizontal separator line
pub fn render_separator(width: usize, theme: &TerminalTheme) -> Element {
    let line = "─".repeat(width);
    Text::new(line)
        .color(theme.border_subtle)
        .dim()
        .into_element()
}
