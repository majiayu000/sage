//! Output type definitions
//!
//! This module defines types for structured output formatting,
//! supporting text, JSON, and streaming JSON modes.

mod base;
mod events;
mod formatting;

#[cfg(test)]
mod tests;

// Re-export all public types
pub use base::{CostInfo, OutputFormat, ToolCallSummary};
pub use events::{
    AssistantEvent, ErrorEvent, OutputEvent, ResultEvent, SystemEvent, ToolCallResultEvent,
    ToolCallStartEvent, UserPromptEvent,
};
pub use formatting::JsonOutput;
