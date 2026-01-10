//! Main Skill type definition

use super::invocation::SkillInvocationConfig;
use super::metadata::SkillMetadata;
use super::source::{SkillSourceInfo, SkillSourceType};
use super::tool_access::ToolAccess;
use super::trigger::SkillTrigger;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A skill definition
///
/// Skills can be defined in markdown files with YAML frontmatter:
/// ```markdown
/// ---
/// description: Code review skill
/// when_to_use: When user asks for code review
/// allowed_tools: [Read, Grep, Glob]
/// user_invocable: true
/// argument_hint: "[file path]"
/// ---
///
/// Please review the code at: $ARGUMENTS
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Skill metadata (name, display_name, description, version)
    pub metadata: SkillMetadata,

    /// Detailed skill prompt (expertise to inject)
    /// Supports $ARGUMENTS, $USER_MESSAGE, $WORKING_DIR, $FILE_CONTEXT
    pub prompt: String,

    /// When to activate this skill (triggers) - legacy trigger system
    pub triggers: Vec<SkillTrigger>,

    /// When to use this skill - AI auto-invocation hint (Claude Code compatible)
    /// If set, AI can automatically invoke this skill when the condition matches
    pub when_to_use: Option<String>,

    /// Tools available to this skill
    pub available_tools: ToolAccess,

    /// Source information (source type and base directory)
    pub source_info: SkillSourceInfo,

    /// Invocation configuration (priority, enabled, invocability, etc.)
    pub invocation: SkillInvocationConfig,
}

// Builder methods
impl Skill {
    /// Create a new skill
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            metadata: SkillMetadata::new(name, description),
            prompt: String::new(),
            triggers: Vec::new(),
            when_to_use: None,
            available_tools: ToolAccess::All,
            source_info: SkillSourceInfo::default(),
            invocation: SkillInvocationConfig::default(),
        }
    }

    /// Set the skill prompt
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// Add a trigger
    pub fn with_trigger(mut self, trigger: SkillTrigger) -> Self {
        self.triggers.push(trigger);
        self
    }

    /// Set available tools
    pub fn with_tools(mut self, tools: ToolAccess) -> Self {
        self.available_tools = tools;
        self
    }

    /// Set source
    pub fn with_source(mut self, source: SkillSourceType) -> Self {
        self.source_info.source = source;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.invocation.priority = priority;
        self
    }

    /// Set model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.invocation.model = Some(model.into());
        self
    }

    /// Disable the skill
    pub fn disabled(mut self) -> Self {
        self.invocation.enabled = false;
        self
    }

    /// Set when to use (Claude Code compatible)
    pub fn with_when_to_use(mut self, when_to_use: impl Into<String>) -> Self {
        self.when_to_use = Some(when_to_use.into());
        self
    }

    /// Set display name
    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.metadata.display_name = Some(name.into());
        self
    }

    /// Set user invocable (can be called via /skill-name)
    pub fn set_user_invocable(mut self) -> Self {
        self.invocation.user_invocable = true;
        self
    }

    /// Disable model invocation
    pub fn disable_model_invocation(mut self) -> Self {
        self.invocation.model_invocable = false;
        self
    }

    /// Set argument hint
    pub fn with_argument_hint(mut self, hint: impl Into<String>) -> Self {
        self.invocation.argument_hint = Some(hint.into());
        self
    }

    /// Set base directory
    pub fn with_base_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.source_info.base_dir = Some(dir.into());
        self
    }

    /// Set version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.metadata.version = Some(version.into());
        self
    }
}
