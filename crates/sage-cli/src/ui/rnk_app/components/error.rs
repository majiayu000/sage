//! Error message component

use rnk::prelude::*;

use crate::ui::rnk_app::formatting::wrap_text_with_prefix;
use crate::ui::rnk_app::theme::Theme;

// Alias rnk's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as RnkBox;

/// Render error message
pub fn render_error(message: &str, theme: &Theme) -> Element {
    let term_width = crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80);

    let mut container = RnkBox::new().flex_direction(FlexDirection::Column);

    container = container.child(Text::new("✗ Error").color(theme.err).bold().into_element());

    let lines = wrap_text_with_prefix("  ", message, term_width);
    for line in lines {
        container = container.child(Text::new(line).color(theme.err).into_element());
    }

    container.into_element()
}
