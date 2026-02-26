//! Core system prompt definitions
//!
//! This module contains the core system prompts used by Sage Agent,
//! following Claude Code's design principles with modular, detailed prompts.

/// Core system prompt components - following Claude Code's structure
pub struct SystemPrompt;

impl SystemPrompt {
    /// Prompt system version for tracking changes
    pub const VERSION: &'static str = "1.0.0";

    /// Main system prompt identity - the core behavior definition
    pub const IDENTITY: &'static str = include_str!("../../prompts/system-prompt/identity.md");

    /// Help and feedback information
    pub const HELP_AND_FEEDBACK: &'static str =
        include_str!("../../prompts/system-prompt/help-and-feedback.md");

    /// Documentation lookup guidance
    pub const DOCUMENTATION_LOOKUP: &'static str =
        include_str!("../../prompts/system-prompt/documentation-lookup.md");

    /// Tone and style guidelines
    pub const TONE_AND_STYLE: &'static str =
        include_str!("../../prompts/system-prompt/tone-and-style.md");

    /// Professional objectivity guidelines
    pub const PROFESSIONAL_OBJECTIVITY: &'static str =
        include_str!("../../prompts/system-prompt/professional-objectivity.md");

    /// Planning without timelines
    pub const PLANNING_WITHOUT_TIMELINES: &'static str =
        include_str!("../../prompts/system-prompt/planning-without-timelines.md");

    /// Task management section (conditional on TodoWrite availability)
    pub const TASK_MANAGEMENT: &'static str =
        include_str!("../../prompts/system-prompt/task-management.md");

    /// Asking questions section - with proactive action principle
    pub const ASKING_QUESTIONS: &'static str =
        include_str!("../../prompts/system-prompt/asking-questions.md");

    /// Hooks section
    pub const HOOKS: &'static str = include_str!("../../prompts/system-prompt/hooks.md");

    /// Doing tasks section - the core coding instructions
    pub const DOING_TASKS: &'static str =
        include_str!("../../prompts/system-prompt/doing-tasks.md");

    /// System reminders info
    pub const SYSTEM_REMINDERS_INFO: &'static str =
        include_str!("../../prompts/system-prompt/system-reminders-info.md");

    /// Tool usage policy
    pub const TOOL_USAGE_POLICY: &'static str =
        include_str!("../../prompts/system-prompt/tool-usage-policy.md");

    /// Code references section
    pub const CODE_REFERENCES: &'static str =
        include_str!("../../prompts/system-prompt/code-references.md");

    /// Environment info section
    pub const ENVIRONMENT_INFO: &'static str =
        include_str!("../../prompts/system-prompt/environment-info.md");

    /// Git status section (conditional)
    pub const GIT_STATUS_SECTION: &'static str =
        include_str!("../../prompts/system-prompt/git-status-section.md");

    /// Strip optional YAML frontmatter from markdown prompt files.
    fn prompt_body(raw: &'static str) -> &'static str {
        let trimmed = raw.trim();
        if let Some(rest) = trimmed.strip_prefix("---")
            && let Some(end_idx) = rest.find("\n---")
        {
            return rest[end_idx + 4..].trim();
        }
        trimmed
    }

    /// Build the complete main system prompt
    pub fn build_main_prompt() -> String {
        format!(
            r#"{identity}

{help_and_feedback}

{documentation_lookup}

{tone_and_style}

{professional_objectivity}

{planning_without_timelines}

${{HAS_TOOL_TODOWRITE?`{task_management}
`:``}}
${{HAS_TOOL_ASKUSERQUESTION?`{asking_questions}
`:``}}
{hooks}

{doing_tasks}

{system_reminders_info}

{tool_usage_policy}

{code_references}

{environment_info}

{git_status}"#,
            identity = Self::prompt_body(Self::IDENTITY),
            help_and_feedback = Self::prompt_body(Self::HELP_AND_FEEDBACK),
            documentation_lookup = Self::prompt_body(Self::DOCUMENTATION_LOOKUP),
            tone_and_style = Self::prompt_body(Self::TONE_AND_STYLE),
            professional_objectivity = Self::prompt_body(Self::PROFESSIONAL_OBJECTIVITY),
            planning_without_timelines = Self::prompt_body(Self::PLANNING_WITHOUT_TIMELINES),
            task_management = Self::prompt_body(Self::TASK_MANAGEMENT),
            asking_questions = Self::prompt_body(Self::ASKING_QUESTIONS),
            hooks = Self::prompt_body(Self::HOOKS),
            doing_tasks = Self::prompt_body(Self::DOING_TASKS),
            system_reminders_info = Self::prompt_body(Self::SYSTEM_REMINDERS_INFO),
            tool_usage_policy = Self::prompt_body(Self::TOOL_USAGE_POLICY),
            code_references = Self::prompt_body(Self::CODE_REFERENCES),
            environment_info = Self::prompt_body(Self::ENVIRONMENT_INFO),
            git_status = Self::prompt_body(Self::GIT_STATUS_SECTION),
        )
    }
}

// SecurityPolicy and GitPrompts are in their own submodules
pub use super::git_prompts::GitPrompts;
pub use super::security_policy::SecurityPolicy;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_contains_agent_name_variable() {
        assert!(SystemPrompt::IDENTITY.contains("${AGENT_NAME}"));
    }

    #[test]
    fn test_tone_and_style_contains_tool_variable() {
        assert!(SystemPrompt::TONE_AND_STYLE.contains("${BASH_TOOL_NAME}"));
    }

    #[test]
    fn test_task_management_contains_todo_variable() {
        assert!(SystemPrompt::TASK_MANAGEMENT.contains("${TODO_TOOL_NAME}"));
    }

    #[test]
    fn test_tool_usage_policy_contains_task_variable() {
        assert!(SystemPrompt::TOOL_USAGE_POLICY.contains("${TASK_TOOL_NAME}"));
        assert!(SystemPrompt::TOOL_USAGE_POLICY.contains("${EXPLORE_AGENT_TYPE}"));
    }

    #[test]
    fn test_environment_info_contains_variables() {
        assert!(SystemPrompt::ENVIRONMENT_INFO.contains("${WORKING_DIR}"));
        assert!(SystemPrompt::ENVIRONMENT_INFO.contains("${PLATFORM}"));
        assert!(SystemPrompt::ENVIRONMENT_INFO.contains("${CURRENT_DATE}"));
    }

    #[test]
    fn test_git_safety_protocol_exists() {
        assert!(GitPrompts::SAFETY_PROTOCOL.contains("Git Safety Protocol"));
        assert!(GitPrompts::SAFETY_PROTOCOL.contains("NEVER"));
    }

    #[test]
    fn test_pr_creation_instructions() {
        assert!(GitPrompts::PR_CREATION.contains("gh pr create"));
        assert!(GitPrompts::PR_CREATION.contains("HEREDOC"));
    }

    #[test]
    fn test_security_policy_exists() {
        assert!(SecurityPolicy::MAIN.contains("security"));
        assert!(SecurityPolicy::CODE_SECURITY.contains("OWASP"));
    }

    #[test]
    fn test_build_main_prompt() {
        let prompt = SystemPrompt::build_main_prompt();
        assert!(prompt.contains("${AGENT_NAME}"));
        assert!(prompt.contains("${BASH_TOOL_NAME}"));
        assert!(prompt.contains("Tone and style"));
        assert!(prompt.contains("Professional objectivity"));
        assert!(!prompt.contains("---\nname:"));
    }

    #[test]
    fn test_doing_tasks_contains_anti_over_engineering() {
        // Verify the Claude Code style anti-over-engineering guidelines are included
        assert!(SystemPrompt::DOING_TASKS.contains("Anti-Over-Engineering"));
        assert!(
            SystemPrompt::DOING_TASKS.contains("bug fix doesn't need surrounding code cleaned up")
        );
        assert!(
            SystemPrompt::DOING_TASKS.contains("Don't design for hypothetical future requirements")
        );
    }
}
