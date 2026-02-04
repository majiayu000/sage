//! UI components for rnk app
//!
//! Each component is in its own module for better organization and reusability.

mod command_suggestions;
mod error;
mod input;
mod message;
mod model_selector;
mod separator;
mod status_bar;
mod thinking;
mod tool_call;

pub use command_suggestions::{
    count_matching_commands, get_selected_command, render_command_suggestions,
};
pub use error::render_error;
pub use input::render_input;
pub use message::format_message;
pub use model_selector::render_model_selector;
pub use separator::render_separator;
pub use status_bar::render_status_bar;
pub use thinking::render_thinking_indicator;
pub use tool_call::format_tool_start;
