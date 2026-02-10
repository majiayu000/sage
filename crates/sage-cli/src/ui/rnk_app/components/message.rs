//! Message display component

use rnk::prelude::*;
use sage_core::ui::bridge::state::{Message, UiMessageContent, Role};

use crate::ui::rnk_app::formatting::{truncate_to_width, wrap_text_with_prefix};
use crate::ui::rnk_app::theme::Theme;

use super::tool_call::render_tool_call;

// Alias rnk's Box to avoid conflict with std::boxed::Box
use rnk::prelude::Box as RnkBox;

fn role_color(role: &Role, theme: &Theme) -> Color {
    match role {
        Role::User => theme.accent_user,
        Role::Assistant => theme.accent_assistant,
        Role::System => theme.accent_system,
    }
}

fn content_line(color: Color, text: String, text_color: Color, is_first: bool) -> Element {
    RnkBox::new()
        .flex_direction(FlexDirection::Row)
        .child(
            Text::new(if is_first { "● " } else { "  " })
                .color(color)
                .into_element(),
        )
        .child(Text::new(text).color(text_color).into_element())
        .into_element()
}

/// Format a message for printing via rnk::println
pub fn format_message(msg: &Message, theme: &Theme) -> Element {
    let term_width = crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80);
    let color = role_color(&msg.role, theme);

    match &msg.content {
        UiMessageContent::Text(text) => {
            let mut container = RnkBox::new().flex_direction(FlexDirection::Column);

            match msg.role {
                Role::User | Role::Assistant => {
                    let mut is_first = true;
                    for paragraph in text.split('\n') {
                        let wrapped =
                            wrap_text_with_prefix("", paragraph, term_width.saturating_sub(4));
                        for line in wrapped {
                            container = container.child(content_line(
                                color,
                                line,
                                theme.text_primary,
                                is_first,
                            ));
                            is_first = false;
                        }
                    }
                }
                Role::System => {
                    let sys_text = truncate_to_width(text, term_width.saturating_sub(4));
                    container =
                        container.child(content_line(color, sys_text, theme.text_muted, true));
                }
            }

            container.into_element()
        }
        UiMessageContent::Thinking(text) => {
            let preview: String = text.lines().take(2).collect::<Vec<_>>().join(" ");
            RnkBox::new()
                .flex_direction(FlexDirection::Row)
                .child(Text::new("💭 ").color(theme.text_subtle).into_element())
                .child(
                    Text::new(format!(
                        "{}…",
                        truncate_to_width(&preview, term_width.saturating_sub(6))
                    ))
                    .color(theme.text_subtle)
                    .into_element(),
                )
                .into_element()
        }
        UiMessageContent::ToolCall {
            tool_name,
            params,
            result,
        } => render_tool_call(tool_name, params, result.as_ref(), theme),
    }
}
