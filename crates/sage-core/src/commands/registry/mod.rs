//! Slash command registry
//!
//! This module provides the command registry for discovering and
//! managing slash commands from various sources.

mod builtins;
mod discovery;
mod types;

#[cfg(test)]
mod tests;

// Re-export the main CommandRegistry type
pub use types::CommandRegistry;
