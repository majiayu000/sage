//! Skill registry
//!
//! This module provides the skill registry for discovering and
//! managing AI-activated skills.

mod types;
mod lookup;
mod builtins;
mod discovery;

#[cfg(test)]
mod tests;

// Re-export public types
pub use types::SkillRegistry;
