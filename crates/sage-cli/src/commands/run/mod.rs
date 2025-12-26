//! Run command implementation
//!
//! This module handles single-task execution mode (non-interactive).

mod execution;
mod result_display;
mod resume;
mod types;

pub use execution::execute;
pub use types::RunArgs;
