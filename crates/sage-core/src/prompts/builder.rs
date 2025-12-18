//! System prompt builder
//!
//! Provides a fluent API for constructing system prompts dynamically.

use super::system_prompt::SystemPrompt;
use super::system_reminders::SystemReminder;
use crate::tools::types::ToolSchema;

/// Builder for constructing system prompts
#[derive(Debug, Clone, Default)]
pub struct SystemPromptBuilder {
    /// Identity information (model name, provider)
    identity_info: Option<String>,
    /// Task description
    task_description: Option<String>,
    /// Working directory
    working_dir: Option<String>,
    /// Tool schemas for description
    tools: Vec<ToolSchema>,
    /// System reminders to include
    reminders: Vec<SystemReminder>,
    /// Whether in plan mode
    in_plan_mode: bool,
    /// Custom sections to add
    custom_sections: Vec<(String, String)>,
}

impl SystemPromptBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set identity information
    pub fn with_identity(mut self, identity: impl Into<String>) -> Self {
        self.identity_info = Some(identity.into());
        self
    }

    /// Set task description
    pub fn with_task(mut self, description: impl Into<String>) -> Self {
        self.task_description = Some(description.into());
        self
    }

    /// Set working directory
    pub fn with_working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Add tools for description
    pub fn with_tools(mut self, tools: Vec<ToolSchema>) -> Self {
        self.tools = tools;
        self
    }

    /// Add a system reminder
    pub fn with_reminder(mut self, reminder: SystemReminder) -> Self {
        self.reminders.push(reminder);
        self
    }

    /// Add multiple reminders
    pub fn with_reminders(mut self, reminders: Vec<SystemReminder>) -> Self {
        self.reminders.extend(reminders);
        self
    }

    /// Enable plan mode
    pub fn in_plan_mode(mut self, enabled: bool) -> Self {
        self.in_plan_mode = enabled;
        self
    }

    /// Add a custom section
    pub fn with_custom_section(mut self, title: impl Into<String>, content: impl Into<String>) -> Self {
        self.custom_sections.push((title.into(), content.into()));
        self
    }

    /// Build the tools description string
    fn build_tools_description(&self) -> String {
        if self.tools.is_empty() {
            return "No tools available.".to_string();
        }

        self.tools
            .iter()
            .map(|schema| format!("- {}: {}", schema.name, schema.description))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Build the reminders section
    fn build_reminders(&self) -> String {
        if self.reminders.is_empty() {
            return String::new();
        }

        self.reminders
            .iter()
            .map(|r| r.to_prompt_string())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Build custom sections
    fn build_custom_sections(&self) -> String {
        if self.custom_sections.is_empty() {
            return String::new();
        }

        self.custom_sections
            .iter()
            .map(|(title, content)| format!("# {}\n{}", title, content))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Build the plan mode section if enabled
    fn build_plan_mode_section(&self) -> String {
        if !self.in_plan_mode {
            return String::new();
        }

        r#"
# PLAN MODE ACTIVE

You are in plan mode. Follow these guidelines:
1. Focus on understanding and designing, NOT implementing yet
2. Use Explore and Plan agents to gather information
3. Write your plan to the plan file
4. Call ExitPlanMode when ready for user approval
5. Keep planning brief (< 2 minutes) - you'll implement after approval

DO NOT:
- Write code files in plan mode
- Call task_done in plan mode
- Skip directly to implementation
"#.to_string()
    }

    /// Build the complete system prompt
    pub fn build(&self) -> String {
        let identity = self.identity_info.as_deref().unwrap_or("Sage Agent");
        let task = self.task_description.as_deref().unwrap_or("No task specified");
        let working_dir = self.working_dir.as_deref().unwrap_or(".");
        let tools_desc = self.build_tools_description();

        let mut prompt = SystemPrompt::build_full_prompt(
            identity,
            task,
            working_dir,
            &tools_desc,
        );

        // Add plan mode section if active
        let plan_mode_section = self.build_plan_mode_section();
        if !plan_mode_section.is_empty() {
            prompt.push_str("\n\n");
            prompt.push_str(&plan_mode_section);
        }

        // Add custom sections
        let custom_sections = self.build_custom_sections();
        if !custom_sections.is_empty() {
            prompt.push_str("\n\n");
            prompt.push_str(&custom_sections);
        }

        // Add reminders at the end
        let reminders = self.build_reminders();
        if !reminders.is_empty() {
            prompt.push_str("\n\n");
            prompt.push_str(&reminders);
        }

        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let prompt = SystemPromptBuilder::new()
            .with_identity("Test Agent")
            .with_task("Test task")
            .with_working_dir("/test")
            .build();

        assert!(prompt.contains("Test Agent"));
        assert!(prompt.contains("Test task"));
        assert!(prompt.contains("/test"));
        assert!(prompt.contains("CODE-FIRST"));
    }

    #[test]
    fn test_builder_with_tools() {
        let tools = vec![
            ToolSchema::new("tool1", "Description 1", vec![]),
            ToolSchema::new("tool2", "Description 2", vec![]),
        ];

        let prompt = SystemPromptBuilder::new()
            .with_tools(tools)
            .build();

        assert!(prompt.contains("tool1"));
        assert!(prompt.contains("tool2"));
        assert!(prompt.contains("Description 1"));
    }

    #[test]
    fn test_builder_with_reminders() {
        let prompt = SystemPromptBuilder::new()
            .with_reminder(SystemReminder::TaskCompletionReminder)
            .build();

        assert!(prompt.contains("system-reminder"));
        assert!(prompt.contains("task_done"));
    }

    #[test]
    fn test_builder_plan_mode() {
        let prompt = SystemPromptBuilder::new()
            .in_plan_mode(true)
            .build();

        assert!(prompt.contains("PLAN MODE ACTIVE"));
        assert!(prompt.contains("ExitPlanMode"));
    }

    #[test]
    fn test_builder_custom_section() {
        let prompt = SystemPromptBuilder::new()
            .with_custom_section("Custom Rules", "Follow these custom rules...")
            .build();

        assert!(prompt.contains("Custom Rules"));
        assert!(prompt.contains("Follow these custom rules"));
    }
}
