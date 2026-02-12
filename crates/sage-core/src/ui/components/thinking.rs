//! Thinking Indicator Component
//!
//! Shows the current thinking state with elapsed time.

use crate::ui::Icons;
use crate::ui::bridge::state::ThinkingState;
use crate::ui::components::Spinner;
use crate::ui::theme::Colors;
use rnk::prelude::*;

/// Thinking indicator component
pub struct ThinkingIndicator {
    state: ThinkingState,
}

impl ThinkingIndicator {
    /// Create a new thinking indicator
    pub fn new(state: ThinkingState) -> Self {
        Self { state }
    }

    /// Render the indicator
    pub fn render(self) -> Element {
        if self.state.completed {
            // Completed state - show checkmark and duration
            let duration_text = if let Some(duration) = self.state.duration {
                format!(
                    "  {} Thought for {:.1}s",
                    Icons::success(),
                    duration.as_secs_f32()
                )
            } else {
                format!("  {} Completed", Icons::success())
            };

            Text::new(duration_text)
                .color(Colors::SUCCESS)
                .dim()
                .into_element()
        } else {
            // In progress - show spinner and elapsed time
            let elapsed = self.state.started_at.elapsed().as_secs_f32();

            Box::new()
                .flex_direction(FlexDirection::Row)
                .child(
                    Spinner::new()
                        .color(Colors::THINKING)
                        .started_at(self.state.started_at)
                        .into_element(),
                )
                .child(
                    Text::new(format!(" Thinking ({:.1}s)", elapsed))
                        .color(Colors::THINKING)
                        .dim()
                        .into_element(),
                )
                .into_element()
        }
    }
}
