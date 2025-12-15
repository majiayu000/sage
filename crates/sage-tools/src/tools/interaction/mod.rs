//! User interaction tools
//!
//! This module provides tools for interactive communication between the agent
//! and the user during execution. These tools enable the agent to:
//! - Ask clarifying questions
//! - Present choices and gather decisions
//! - Collect user input when multiple approaches are valid

pub mod ask_user;

// Re-export the main tool
pub use ask_user::AskUserQuestionTool;
