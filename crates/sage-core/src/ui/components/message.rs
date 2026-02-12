//! Message Component - Display chat messages
//!
//! Renders user/assistant messages with appropriate styling.

use crate::ui::bridge::state::{Message, Role, UiMessageContent, UiToolResult};
use crate::ui::theme::{Colors, Icons};
use rnk::prelude::*;

/// Render a list of messages
#[allow(non_snake_case)]
pub fn MessageList(messages: Vec<Message>) -> Element {
    Box::new()
        .flex_direction(FlexDirection::Column)
        .children(
            messages
                .into_iter()
                .map(|msg| MessageView::new(msg).render()),
        )
        .into_element()
}

/// Single message view component
pub struct MessageView {
    message: Message,
}

impl MessageView {
    /// Create a new message view
    pub fn new(message: Message) -> Self {
        Self { message }
    }

    /// Render the message
    pub fn render(self) -> Element {
        match &self.message.content {
            UiMessageContent::Text(text) => Self::render_text_message(&self.message.role, text),
            UiMessageContent::ToolCall {
                tool_name,
                params,
                result,
            } => Self::render_tool_call(tool_name, params, result.as_ref()),
            UiMessageContent::Thinking(text) => Self::render_thinking(text),
        }
    }

    /// Render a text message
    fn render_text_message(role: &Role, text: &str) -> Element {
        let (icon, color) = match role {
            Role::User => (Icons::prompt(), Colors::USER),
            Role::Assistant => (Icons::message(), Colors::ASSISTANT),
            Role::System => (Icons::info(), Colors::SYSTEM),
        };

        // Split text into lines and indent continuation lines
        let lines: Vec<&str> = text.lines().collect();
        let mut children = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                // First line with icon
                children.push(
                    Box::new()
                        .flex_direction(FlexDirection::Row)
                        .child(
                            Text::new(format!("{} ", icon))
                                .color(color)
                                .bold()
                                .into_element(),
                        )
                        .child(Text::new(*line).color(Colors::TEXT).into_element())
                        .into_element(),
                );
            } else {
                // Continuation lines with 2-space indent
                children.push(
                    Box::new()
                        .padding_left(2.0)
                        .child(Text::new(*line).color(Colors::TEXT).into_element())
                        .into_element(),
                );
            }
        }

        Box::new()
            .flex_direction(FlexDirection::Column)
            .padding_top(1.0)
            .children(children)
            .into_element()
    }

    /// Render a tool call
    fn render_tool_call(tool_name: &str, params: &str, result: Option<&UiToolResult>) -> Element {
        let mut container = Box::new()
            .flex_direction(FlexDirection::Column)
            .padding_top(1.0);

        // Tool call header
        container = container.child(
            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(
                    Text::new(format!("{} ", Icons::message()))
                        .color(Colors::TOOL)
                        .into_element(),
                )
                .child(
                    Text::new(tool_name)
                        .color(Colors::TOOL)
                        .bold()
                        .into_element(),
                )
                .child(
                    Text::new(format!(" ({})", Self::truncate(params, 50)))
                        .color(Colors::TOOL)
                        .dim()
                        .into_element(),
                )
                .into_element(),
        );

        // Tool result
        if let Some(res) = result {
            let (icon, color, text) = if res.success {
                (
                    Icons::result(),
                    Colors::SUCCESS,
                    res.output.as_deref().unwrap_or(""),
                )
            } else {
                (
                    Icons::result(),
                    Colors::ERROR,
                    res.error.as_deref().unwrap_or("Unknown error"),
                )
            };

            container = container.child(
                Box::new()
                    .flex_direction(FlexDirection::Row)
                    .padding_left(2.0)
                    .child(
                        Text::new(format!("{} ", icon))
                            .color(color)
                            .dim()
                            .into_element(),
                    )
                    .child(
                        Text::new(Self::truncate(text, 200))
                            .color(color)
                            .dim()
                            .into_element(),
                    )
                    .into_element(),
            );
        }

        container.into_element()
    }

    /// Render thinking content
    fn render_thinking(text: &str) -> Element {
        Box::new()
            .flex_direction(FlexDirection::Column)
            .padding_top(1.0)
            .child(
                Text::new(format!("{} Thinking...", Icons::cogitate()))
                    .color(Colors::THINKING)
                    .into_element(),
            )
            .child(
                Box::new()
                    .padding_left(2.0)
                    .child(
                        Text::new(Self::truncate(text, 300))
                            .color(Colors::THINKING)
                            .dim()
                            .into_element(),
                    )
                    .into_element(),
            )
            .into_element()
    }

    /// Truncate text to max length with ellipsis
    fn truncate(text: &str, max_len: usize) -> String {
        if text.chars().count() <= max_len {
            text.to_string()
        } else {
            let truncated: String = text.chars().take(max_len).collect();
            format!("{}...", truncated)
        }
    }
}
