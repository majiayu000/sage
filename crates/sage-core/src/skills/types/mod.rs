//! Skill type definitions
//!
//! This module defines types for the AI-activated skills system,
//! providing domain-specific expertise that can be automatically invoked.
//!
//! ## Claude Code Compatible Features
//!
//! This skill system is designed to be compatible with Claude Code's skill format:
//! - YAML frontmatter with markdown content
//! - `when_to_use` for AI auto-invocation
//! - `user_invocable` for slash command availability
//! - `allowed_tools` for tool access control
//! - `$ARGUMENTS` parameter substitution

mod accessors;
mod activation;
mod context;
mod invocation;
mod metadata;
mod prompt;
mod skill;
mod source;
mod tool_access;
mod trigger;

pub use activation::SkillActivation;
pub use context::SkillContext;
pub use invocation::SkillInvocationConfig;
pub use metadata::SkillMetadata;
pub use skill::Skill;
pub use source::{SkillSource, SkillSourceInfo, SkillSourceType};
pub use tool_access::ToolAccess;
pub use trigger::{SkillTrigger, TaskType};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_skill_creation() {
        let skill = Skill::new("rust-expert", "Rust programming expertise")
            .with_prompt("You are an expert in Rust programming...")
            .with_priority(10);

        assert_eq!(skill.name(), "rust-expert");
        assert_eq!(skill.priority(), 10);
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
