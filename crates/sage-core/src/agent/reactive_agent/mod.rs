//! Reactive Agent - Claude Code style execution model
//!
//! This module implements a lightweight, response-driven execution model
//! inspired by Claude Code's design philosophy.

mod agent;
mod execution;
mod trait_def;
mod types;

// Re-export public types
pub use agent::ClaudeStyleAgent;
pub use execution::ReactiveExecutionManager;
pub use trait_def::ReactiveAgent;
pub use types::{ReactiveResponse, TokenUsage};
