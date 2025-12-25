//! Memory and session notes tools
//!
//! Provides tools for storing and retrieving memories that persist across sessions.

mod analyzer;
mod schema;
#[cfg(test)]
mod tests;
mod tool;
mod types;

// Re-export public API
pub use analyzer::get_memories_for_context;
pub use tool::{RememberTool, SessionNotesTool};
pub use types::{get_global_memory_manager, init_global_memory_manager};
