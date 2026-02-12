//! AskUserQuestion tool for interactive user input during agent execution
//!
//! This module provides functionality for the agent to interactively gather
//! information from the user when it needs clarification or choices to be made.

mod schema;
mod tool;
mod validation;

#[cfg(test)]
mod tests;

// Re-export public items
pub use sage_core::input::{Question, QuestionOption};
pub use tool::AskUserQuestionTool;
