//! Skill activation result

use super::context::SkillContext;
use super::skill::Skill;
use super::tool_access::ToolAccess;

/// Result of skill activation
#[derive(Debug, Clone)]
pub struct SkillActivation {
    /// The activated skill
    pub skill_name: String,

    /// Prompt to inject
    pub injected_prompt: String,

    /// Tools available
    pub tool_access: ToolAccess,

    /// Model to use
    pub model: Option<String>,

    /// Status message
    pub status: String,
}

impl SkillActivation {
    /// Create a new activation
    pub fn new(skill: &Skill, context: &SkillContext) -> Self {
        Self {
            skill_name: skill.name().to_string(),
            injected_prompt: skill.get_full_prompt(context),
            tool_access: skill.available_tools.clone(),
            model: skill.model().map(|s| s.to_string()),
            status: format!("Activating skill: {}", skill.name()),
        }
    }
}
