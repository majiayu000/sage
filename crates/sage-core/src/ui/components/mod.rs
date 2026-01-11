//! UI Components - Reusable UI components for Sage
//!
//! All components follow the pattern of taking state and returning Elements.

pub mod input_box;
pub mod message;
pub mod spinner;
pub mod status_bar;
pub mod thinking;
pub mod tool_call;

pub use input_box::InputBox;
pub use message::{MessageList, MessageView};
pub use spinner::Spinner;
pub use status_bar::StatusBar;
pub use thinking::ThinkingIndicator;
pub use tool_call::ToolExecutionView;
