//! Skill registry
//!
//! This module provides the skill registry for discovering and
//! managing AI-activated skills.

mod builtins;
pub mod discovery;
mod lookup;
mod types;
pub mod watcher;

#[cfg(test)]
mod tests;

// Re-export public types
pub use discovery::SkillFrontmatter;
pub use types::SkillRegistry;
pub use watcher::{SkillChangeEvent, SkillHotReloader, SkillWatcher, SkillWatcherConfig};
