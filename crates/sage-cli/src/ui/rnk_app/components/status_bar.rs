//! Status bar component

use rnk::prelude::*;

use crate::ui::rnk_app::state::PermissionMode;
use crate::ui::rnk_app::theme::Theme;

// Alias rnk's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as RnkBox;

/// Render status bar with mode and hints
pub fn render_status_bar(
    permission_mode: PermissionMode,
    model: Option<&str>,
    theme: &Theme,
) -> Element {
    let (mode_color, mode_label, mode_icon) = match permission_mode {
        PermissionMode::Normal => (theme.status_normal, "permissions required", "⏵"),
        PermissionMode::Bypass => (theme.status_bypass, "bypass mode", "⏵⏵"),
        PermissionMode::Plan => (theme.status_plan, "plan mode", "📋"),
    };

    let mut row = RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(Text::new("  ").into_element())
        .child(
            Text::new(format!("{} {}", mode_icon, mode_label))
                .color(mode_color)
                .bold()
                .into_element(),
        )
        .child(Text::new("  │  ").color(theme.border_subtle).into_element())
        .child(
            Text::new("shift+Tab to cycle")
                .color(theme.text_subtle)
                .into_element(),
        );

    // Show model name if available
    if let Some(m) = model {
        if m != "unknown" {
            row = row
                .child(Text::new("  │  ").color(theme.border_subtle).into_element())
                .child(Text::new(m).color(theme.text_subtle).into_element());
        }
    }

    row.into_element()
}
