//! JSON editing tool using JSONPath
//!
//! **STATUS: DISABLED** - This is a Sage-specific tool not present in Claude Code.
//! Kept for potential future use but not registered in the default tool set.

mod operations;
mod schema;
mod types;

#[cfg(test)]
mod tests;

// Re-export public types
pub use types::JsonEditTool;
