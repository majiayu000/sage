//! Slash command executor
//!
//! This module provides the command executor for processing
//! and executing slash commands.

mod executor;
mod handlers;
mod types;

#[cfg(test)]
mod tests;

pub use types::CommandExecutor;
