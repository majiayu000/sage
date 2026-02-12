//! Input Box Component
//!
//! User input area with prompt and cursor.

use crate::ui::Icons;
use crate::ui::bridge::state::InputState;
use crate::ui::theme::Colors;
use rnk::prelude::*;

/// Input box component
pub struct InputBox {
    state: InputState,
}

impl InputBox {
    /// Create a new input box
    pub fn new(state: InputState) -> Self {
        Self { state }
    }

    /// Render the input box
    pub fn render(self) -> Element {
        if !self.state.enabled {
            return Box::new().into_element();
        }

        Box::new()
            .flex_direction(FlexDirection::Row)
            .padding(1.0)
            .border_style(BorderStyle::Single)
            .border_color(Colors::INPUT_BORDER)
            .child(
                Text::new(format!("{} ", Icons::prompt()))
                    .color(Colors::USER)
                    .bold()
                    .into_element(),
            )
            .child(
                Text::new(&self.state.text)
                    .color(Colors::TEXT)
                    .into_element(),
            )
            .child(
                Text::new("█") // Cursor
                    .color(Colors::CURSOR)
                    .into_element(),
            )
            .into_element()
    }
}
