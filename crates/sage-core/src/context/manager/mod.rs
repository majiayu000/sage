//! Context window manager
//!
//! This module provides the main ContextManager that orchestrates token estimation,
//! message pruning, and conversation summarization to manage the LLM context window.

mod core;
mod operations;
#[cfg(test)]
mod tests;
mod types;

pub use core::ContextManager;
pub use types::{ContextUsageStats, PrepareResult};
