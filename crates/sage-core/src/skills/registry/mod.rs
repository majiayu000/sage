//! Skill registry
//!
//! This module provides the skill registry for discovering and
//! managing AI-activated skills.

mod builtins;
mod discovery;
mod lookup;
mod types;

#[cfg(test)]
mod tests;

// Re-export public types
pub use types::SkillRegistry;
