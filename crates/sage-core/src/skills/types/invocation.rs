//! Skill invocation configuration

use serde::{Deserialize, Serialize};

/// Skill invocation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInvocationConfig {
    /// Priority (higher = checked first)
    pub priority: i32,

    /// Whether this skill is enabled
    pub enabled: bool,

    /// Whether AI can auto-invoke this skill (default: true if when_to_use is set)
    pub model_invocable: bool,

    /// Whether user can invoke via slash command (e.g., /skill-name)
    pub user_invocable: bool,

    /// Argument hint shown to user (e.g., "[file path]")
    pub argument_hint: Option<String>,

    /// Model override for this skill
    pub model: Option<String>,
}

impl Default for SkillInvocationConfig {
    fn default() -> Self {
        Self {
            priority: 0,
            enabled: true,
            model_invocable: true,
            user_invocable: false,
            argument_hint: None,
            model: None,
        }
    }
}
