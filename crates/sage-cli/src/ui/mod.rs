//! Sage CLI UI Module
//!
//! Provides streaming UI components for the CLI interface.

mod indicators;
mod streaming;

pub use indicators::{ThinkingIndicator, ToolIndicator};
pub use streaming::{
    print_assistant_response, print_error, print_thinking, print_tool_call, print_tool_result,
    print_user_message, StreamingPrinter,
};

