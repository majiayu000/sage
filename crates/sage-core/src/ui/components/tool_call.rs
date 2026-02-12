//! Tool Execution View Component
//!
//! Displays the current tool execution state.

use crate::ui::bridge::state::{ToolExecution, ToolStatus};
use crate::ui::components::Spinner;
use crate::ui::Icons;
use crate::ui::theme::Colors;
use rnk::prelude::*;

/// Tool execution view component
pub struct ToolExecutionView {
    execution: ToolExecution,
}

impl ToolExecutionView {
    /// Create a new tool execution view
    pub fn new(execution: ToolExecution) -> Self {
        Self { execution }
    }

    /// Render the view
    pub fn render(self) -> Element {
        match &self.execution.status {
            ToolStatus::Running => {
                let elapsed = self.execution.started_at.elapsed().as_secs_f32();

                Box::new()
                    .flex_direction(FlexDirection::Row)
                    .padding_left(2.0)
                    .child(
                        Spinner::new()
                            .color(Colors::TOOL)
                            .started_at(self.execution.started_at)
                            .into_element(),
                    )
                    .child(
                        Text::new(format!(
                            " Running · {} ({:.1}s)",
                            self.execution.description, elapsed
                        ))
                        .color(Colors::TOOL)
                        .into_element(),
                    )
                    .into_element()
            }

            ToolStatus::Completed { duration } => Box::new()
                .flex_direction(FlexDirection::Row)
                .padding_left(2.0)
                .child(
                    Text::new(format!("{} ", Icons::success()))
                        .color(Colors::SUCCESS)
                        .into_element(),
                )
                .child(
                    Text::new(format!(
                        "{} ({:.2}s)",
                        self.execution.tool_name,
                        duration.as_secs_f32()
                    ))
                    .color(Colors::SUCCESS)
                    .dim()
                    .into_element(),
                )
                .into_element(),

            ToolStatus::Failed { error } => Box::new()
                .flex_direction(FlexDirection::Row)
                .padding_left(2.0)
                .child(
                    Text::new(format!("{} ", Icons::error()))
                        .color(Colors::ERROR)
                        .into_element(),
                )
                .child(
                    Text::new(format!("{}: {}", self.execution.tool_name, error))
                        .color(Colors::ERROR)
                        .into_element(),
                )
                .into_element(),
        }
    }
}
