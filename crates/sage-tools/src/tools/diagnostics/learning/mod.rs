//! Learning mode tools
//!
//! Provides tools for learning from user corrections and preferences,
//! similar to Claude Code's learning capabilities.

mod analyzer;
mod learn_tool;
mod patterns_tool;
mod schema;
mod tests;
mod types;

// Re-export public API
pub use learn_tool::LearnTool;
pub use patterns_tool::LearningPatternsTool;
pub use types::{
    get_global_learning_engine, get_learning_patterns_for_context, init_global_learning_engine,
};
