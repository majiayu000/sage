//! Write tool for creating or overwriting files
//!
//! This tool follows Claude Code's design pattern for the Write tool,
//! which allows creating new files or overwriting existing files with
//! proper validation and security checks.

mod schema;
mod types;
mod writer;

#[cfg(test)]
mod tests;

// Re-export public items
pub use types::WriteTool;
