//! Skill trigger types

use super::context::SkillContext;
use serde::{Deserialize, Serialize};

/// Skill trigger definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillTrigger {
    /// Trigger on keyword match in user message
    Keyword(String),

    /// Trigger on regex match in user message
    Regex(String),

    /// Trigger on file extension being worked on
    FileExtension(String),

    /// Trigger on specific tool being used
    ToolUsage(String),

    /// Trigger on explicit skill invocation
    Explicit,

    /// Always trigger (for default skills)
    Always,

    /// Trigger on task type
    SkillTaskType(SkillTaskType),
}

impl SkillTrigger {
    /// Check if the trigger matches a context
    pub fn matches(&self, context: &SkillContext) -> bool {
        match self {
            Self::Keyword(kw) => {
                let lower_msg = context.user_message.to_lowercase();
                let lower_kw = kw.to_lowercase();
                lower_msg.contains(&lower_kw)
            }
            Self::Regex(pattern) => regex::Regex::new(pattern)
                .map(|re| re.is_match(&context.user_message))
                .unwrap_or(false),
            Self::FileExtension(ext) => context
                .active_files
                .iter()
                .any(|f| f.extension().map_or(false, |e| e == ext.as_str())),
            Self::ToolUsage(tool) => context.recent_tools.contains(tool),
            Self::Explicit => context.explicit_skill.as_ref() == Some(&context.user_message),
            Self::Always => true,
            Self::SkillTaskType(task_type) => context.detected_task_type.as_ref() == Some(task_type),
        }
    }
}

/// Task types that can trigger skills
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillTaskType {
    /// Writing new code
    CodeWriting,
    /// Fixing bugs
    Debugging,
    /// Refactoring code
    Refactoring,
    /// Writing tests
    Testing,
    /// Code review
    Review,
    /// Documentation
    Documentation,
    /// Performance optimization
    Optimization,
    /// Security analysis
    Security,
    /// Architecture design
    Architecture,
    /// Build/deployment
    DevOps,
}
