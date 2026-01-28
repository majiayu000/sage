//! Thinking indicator component

use rnk::prelude::*;

use crate::ui::rnk_app::theme::Theme;

// Alias rnk's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as RnkBox;

/// Braille spinner frames for smooth animation
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Render thinking indicator with braille spinner
pub fn render_thinking_indicator(status_text: &str, animation_frame: usize, theme: &Theme) -> Element {
    let spinner = SPINNER_FRAMES[animation_frame % SPINNER_FRAMES.len()];

    let display_text = if status_text.is_empty() {
        "Thinking...".to_string()
    } else {
        format!("{}...", status_text)
    };

    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new(format!("{} ", spinner)).color(theme.accent_primary).bold().into_element())
        .child(Text::new(display_text).color(theme.text_muted).into_element())
        .into_element()
}
