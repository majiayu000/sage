//! Status Bar Component
//!
//! Displays session info, current status, and step count.

use crate::ui::bridge::state::{ExecutionPhase, SessionState};
use crate::ui::theme::{Colors, Icons};
use rnk::prelude::*;

/// Status bar component
pub struct StatusBar {
    session: SessionState,
    phase: ExecutionPhase,
}

impl StatusBar {
    /// Create a new status bar
    pub fn new(session: SessionState, phase: ExecutionPhase) -> Self {
        Self { session, phase }
    }

    /// Render the status bar
    pub fn render(self) -> Element {
        Box::new()
            .flex_direction(FlexDirection::Row)
            .justify_content(JustifyContent::SpaceBetween)
            .padding(1.0)
            .border_style(BorderStyle::Single)
            .border_color(Colors::BORDER)
            .child(self.render_left())
            .child(self.render_right())
            .into_element()
    }

    /// Render left side: session info
    fn render_left(&self) -> Element {
        let mut row = Box::new().flex_direction(FlexDirection::Row);

        // Brand icon
        row = row.child(
            Text::new(format!("{} ", Icons::sage()))
                .color(Colors::BRAND)
                .into_element(),
        );

        // Model and provider
        row = row.child(
            Text::new(format!("{} · {}", self.session.model, self.session.provider))
                .color(Colors::TEXT_DIM)
                .into_element(),
        );

        // Git branch if available
        if let Some(branch) = &self.session.git_branch {
            row = row.child(
                Text::new(format!(" · {} {}", Icons::git_branch(), branch))
                    .color(Colors::GIT)
                    .into_element(),
            );
        }

        row.into_element()
    }

    /// Render right side: step and status
    fn render_right(&self) -> Element {
        let mut row = Box::new().flex_direction(FlexDirection::Row);

        // Step counter
        let step_text = if let Some(max) = self.session.max_steps {
            format!("Step {}/{} · ", self.session.step, max)
        } else if self.session.step > 0 {
            format!("Step {} · ", self.session.step)
        } else {
            String::new()
        };

        if !step_text.is_empty() {
            row = row.child(Text::new(step_text).color(Colors::TEXT_DIM).into_element());
        }

        // Phase indicator
        row = row.child(self.render_phase_indicator());

        row.into_element()
    }

    /// Render the phase indicator
    fn render_phase_indicator(&self) -> Element {
        let (icon, color, text) = match &self.phase {
            ExecutionPhase::Idle => (Icons::success(), Colors::SUCCESS, "Ready".to_string()),
            ExecutionPhase::Thinking => {
                (Icons::cogitate(), Colors::THINKING, "Thinking".to_string())
            }
            ExecutionPhase::Streaming { .. } => {
                (Icons::message(), Colors::ASSISTANT, "Streaming".to_string())
            }
            ExecutionPhase::ExecutingTool { tool_name, .. } => {
                return Text::new(format!("{} {}", Icons::running(), tool_name))
                    .color(Colors::TOOL)
                    .into_element();
            }
            ExecutionPhase::WaitingConfirmation { .. } => {
                (Icons::warning(), Colors::WARNING, "Waiting".to_string())
            }
            ExecutionPhase::Error { .. } => (Icons::error(), Colors::ERROR, "Error".to_string()),
        };

        Text::new(format!("{} {}", icon, text))
            .color(color)
            .into_element()
    }
}
