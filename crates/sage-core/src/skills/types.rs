//! Skill type definitions
//!
//! This module defines types for the AI-activated skills system,
//! providing domain-specific expertise that can be automatically invoked.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A skill definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Skill name
    pub name: String,

    /// Short description
    pub description: String,

    /// Detailed skill prompt (expertise to inject)
    pub prompt: String,

    /// When to activate this skill (triggers)
    pub triggers: Vec<SkillTrigger>,

    /// Tools available to this skill
    pub available_tools: ToolAccess,

    /// Source location of the skill
    pub source: SkillSource,

    /// Priority (higher = checked first)
    pub priority: i32,

    /// Whether this skill is enabled
    pub enabled: bool,

    /// Model override for this skill
    pub model: Option<String>,
}

impl Skill {
    /// Create a new skill
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            prompt: String::new(),
            triggers: Vec::new(),
            available_tools: ToolAccess::All,
            source: SkillSource::Builtin,
            priority: 0,
            enabled: true,
            model: None,
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
    pub fn with_source(mut self, source: SkillSource) -> Self {
        self.source = source;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Disable the skill
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Check if the skill matches a context
    pub fn matches(&self, context: &SkillContext) -> bool {
        if !self.enabled {
            return false;
        }

        self.triggers.iter().any(|trigger| trigger.matches(context))
    }

    /// Get the skill's full prompt including context
    pub fn get_full_prompt(&self, context: &SkillContext) -> String {
        let mut prompt = self.prompt.clone();

        // Replace context variables
        prompt = prompt.replace("$USER_MESSAGE", &context.user_message);
        prompt = prompt.replace("$WORKING_DIR", &context.working_dir.to_string_lossy());

        if let Some(ref file_context) = context.file_context {
            prompt = prompt.replace("$FILE_CONTEXT", file_context);
        }

        prompt
    }
}

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
    TaskType(TaskType),
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
            Self::TaskType(task_type) => context.detected_task_type.as_ref() == Some(task_type),
        }
    }
}

/// Task types that can trigger skills
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
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

/// Tool access control for skills
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolAccess {
    /// All tools available
    All,
    /// Only specific tools
    Only(Vec<String>),
    /// All except specific tools
    Except(Vec<String>),
    /// Read-only tools only
    ReadOnly,
}

impl ToolAccess {
    /// Check if a tool is allowed
    pub fn allows(&self, tool_name: &str) -> bool {
        match self {
            Self::All => true,
            Self::Only(allowed) => allowed.iter().any(|t| t.eq_ignore_ascii_case(tool_name)),
            Self::Except(denied) => !denied.iter().any(|t| t.eq_ignore_ascii_case(tool_name)),
            Self::ReadOnly => {
                let read_only = ["Read", "Glob", "Grep", "WebFetch", "WebSearch"];
                read_only.iter().any(|t| t.eq_ignore_ascii_case(tool_name))
            }
        }
    }
}

impl Default for ToolAccess {
    fn default() -> Self {
        Self::All
    }
}

/// Skill source location
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillSource {
    /// Built-in skill
    Builtin,
    /// Project skill (.sage/skills/)
    Project(PathBuf),
    /// User skill (~/.config/sage/skills/)
    User(PathBuf),
    /// MCP-provided skill
    Mcp(String),
}

impl Default for SkillSource {
    fn default() -> Self {
        Self::Builtin
    }
}

/// Context for skill matching
#[derive(Debug, Clone, Default)]
pub struct SkillContext {
    /// User's message/prompt
    pub user_message: String,

    /// Current working directory
    pub working_dir: PathBuf,

    /// Active files being worked on
    pub active_files: Vec<PathBuf>,

    /// Recently used tools
    pub recent_tools: Vec<String>,

    /// Explicitly requested skill
    pub explicit_skill: Option<String>,

    /// Detected task type
    pub detected_task_type: Option<TaskType>,

    /// Additional file context
    pub file_context: Option<String>,
}

impl SkillContext {
    /// Create a new context
    pub fn new(user_message: impl Into<String>) -> Self {
        Self {
            user_message: user_message.into(),
            ..Default::default()
        }
    }

    /// Set working directory
    pub fn with_working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = dir.into();
        self
    }

    /// Add active file
    pub fn with_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.active_files.push(file.into());
        self
    }

    /// Add recent tool
    pub fn with_recent_tool(mut self, tool: impl Into<String>) -> Self {
        self.recent_tools.push(tool.into());
        self
    }

    /// Set explicit skill request
    pub fn with_explicit_skill(mut self, skill: impl Into<String>) -> Self {
        self.explicit_skill = Some(skill.into());
        self
    }

    /// Set detected task type
    pub fn with_task_type(mut self, task_type: TaskType) -> Self {
        self.detected_task_type = Some(task_type);
        self
    }
}

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
            skill_name: skill.name.clone(),
            injected_prompt: skill.get_full_prompt(context),
            tool_access: skill.available_tools.clone(),
            model: skill.model.clone(),
            status: format!("Activating skill: {}", skill.name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_creation() {
        let skill = Skill::new("rust-expert", "Rust programming expertise")
            .with_prompt("You are an expert in Rust programming...")
            .with_priority(10);

        assert_eq!(skill.name, "rust-expert");
        assert_eq!(skill.priority, 10);
    }

    #[test]
    fn test_skill_trigger_keyword() {
        let trigger = SkillTrigger::Keyword("rust".to_string());
        let context = SkillContext::new("Help me write Rust code");

        assert!(trigger.matches(&context));

        let context2 = SkillContext::new("Help me write Python code");
        assert!(!trigger.matches(&context2));
    }

    #[test]
    fn test_skill_trigger_regex() {
        let trigger = SkillTrigger::Regex(r"(?i)test|spec".to_string());
        let context = SkillContext::new("Write a test for this");

        assert!(trigger.matches(&context));
    }

    #[test]
    fn test_skill_trigger_file_extension() {
        let trigger = SkillTrigger::FileExtension("rs".to_string());
        let context = SkillContext::new("Edit this file").with_file("main.rs");

        assert!(trigger.matches(&context));
    }

    #[test]
    fn test_skill_trigger_tool_usage() {
        let trigger = SkillTrigger::ToolUsage("Bash".to_string());
        let context = SkillContext::new("Run tests").with_recent_tool("Bash");

        assert!(trigger.matches(&context));
    }

    #[test]
    fn test_skill_matches() {
        let skill = Skill::new("testing", "Testing skill")
            .with_trigger(SkillTrigger::Keyword("test".to_string()))
            .with_trigger(SkillTrigger::TaskType(TaskType::Testing));

        let context1 = SkillContext::new("Write a test");
        assert!(skill.matches(&context1));

        let context2 = SkillContext::new("Write code").with_task_type(TaskType::Testing);
        assert!(skill.matches(&context2));

        let context3 = SkillContext::new("Write code");
        assert!(!skill.matches(&context3));
    }

    #[test]
    fn test_skill_disabled() {
        let skill = Skill::new("disabled", "Disabled skill")
            .with_trigger(SkillTrigger::Always)
            .disabled();

        let context = SkillContext::new("Any message");
        assert!(!skill.matches(&context));
    }

    #[test]
    fn test_tool_access_all() {
        let access = ToolAccess::All;
        assert!(access.allows("Read"));
        assert!(access.allows("Write"));
        assert!(access.allows("Bash"));
    }

    #[test]
    fn test_tool_access_only() {
        let access = ToolAccess::Only(vec!["Read".to_string(), "Glob".to_string()]);
        assert!(access.allows("Read"));
        assert!(access.allows("Glob"));
        assert!(!access.allows("Write"));
    }

    #[test]
    fn test_tool_access_except() {
        let access = ToolAccess::Except(vec!["Bash".to_string()]);
        assert!(access.allows("Read"));
        assert!(access.allows("Write"));
        assert!(!access.allows("Bash"));
    }

    #[test]
    fn test_tool_access_read_only() {
        let access = ToolAccess::ReadOnly;
        assert!(access.allows("Read"));
        assert!(access.allows("Grep"));
        assert!(!access.allows("Write"));
        assert!(!access.allows("Bash"));
    }

    #[test]
    fn test_skill_prompt_expansion() {
        let skill =
            Skill::new("test", "Test").with_prompt("User said: $USER_MESSAGE in $WORKING_DIR");

        let context = SkillContext::new("hello").with_working_dir("/project");

        let prompt = skill.get_full_prompt(&context);
        assert!(prompt.contains("hello"));
        assert!(prompt.contains("/project"));
    }

    #[test]
    fn test_skill_activation() {
        let skill = Skill::new("test", "Test")
            .with_prompt("Test prompt")
            .with_model("haiku");

        let context = SkillContext::new("test");
        let activation = SkillActivation::new(&skill, &context);

        assert_eq!(activation.skill_name, "test");
        assert_eq!(activation.model, Some("haiku".to_string()));
    }

    #[test]
    fn test_skill_context_builder() {
        let context = SkillContext::new("message")
            .with_working_dir("/project")
            .with_file("main.rs")
            .with_recent_tool("Read")
            .with_task_type(TaskType::Debugging);

        assert_eq!(context.user_message, "message");
        assert_eq!(context.working_dir, PathBuf::from("/project"));
        assert_eq!(context.active_files.len(), 1);
        assert_eq!(context.recent_tools.len(), 1);
        assert_eq!(context.detected_task_type, Some(TaskType::Debugging));
    }
}
