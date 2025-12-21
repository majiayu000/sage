//! System prompt builder
//!
//! Provides a fluent API for constructing system prompts dynamically,
//! following Claude Code's design pattern with template variables.

use super::system_prompt::{GitPrompts, SecurityPolicy, SystemPrompt};
use super::system_reminders::SystemReminder;
use super::tool_descriptions::ToolDescriptions;
use super::variables::{PromptVariables, TemplateRenderer};
use crate::tools::types::ToolSchema;

/// Builder for constructing system prompts
#[derive(Debug, Clone)]
pub struct SystemPromptBuilder {
    /// Variables for template rendering
    variables: PromptVariables,
    /// Tool schemas for description
    tools: Vec<ToolSchema>,
    /// System reminders to include
    reminders: Vec<SystemReminder>,
    /// Whether in plan mode
    in_plan_mode: bool,
    /// Plan file path (for plan mode)
    plan_file_path: Option<String>,
    /// Whether plan file exists
    plan_exists: bool,
    /// Include Git instructions
    include_git_instructions: bool,
    /// Include security policy
    include_security_policy: bool,
    /// Custom sections to add
    custom_sections: Vec<(String, String)>,
}

impl Default for SystemPromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemPromptBuilder {
    /// Create a new builder with default variables
    pub fn new() -> Self {
        Self {
            variables: PromptVariables::new(),
            tools: Vec::new(),
            reminders: Vec::new(),
            in_plan_mode: false,
            plan_file_path: None,
            plan_exists: false,
            include_git_instructions: true,
            include_security_policy: true,
            custom_sections: Vec::new(),
        }
    }

    /// Set the agent name
    pub fn with_agent_name(mut self, name: impl Into<String>) -> Self {
        self.variables.agent_name = name.into();
        self
    }

    /// Set the agent version
    pub fn with_agent_version(mut self, version: impl Into<String>) -> Self {
        self.variables.agent_version = version.into();
        self
    }

    /// Set the model name
    pub fn with_model_name(mut self, model: impl Into<String>) -> Self {
        self.variables.model_name = model.into();
        self
    }

    /// Set task description
    pub fn with_task(mut self, description: impl Into<String>) -> Self {
        self.variables.task_description = description.into();
        self
    }

    /// Set working directory
    pub fn with_working_dir(mut self, dir: impl Into<String>) -> Self {
        self.variables.working_dir = dir.into();
        self
    }

    /// Set Git information
    pub fn with_git_info(
        mut self,
        is_repo: bool,
        branch: impl Into<String>,
        main_branch: impl Into<String>,
    ) -> Self {
        self.variables.is_git_repo = is_repo;
        self.variables.git_branch = branch.into();
        self.variables.main_branch = main_branch.into();
        self
    }

    /// Add tools for description and register them as available
    pub fn with_tools(mut self, tools: Vec<ToolSchema>) -> Self {
        // Register each tool as available
        for tool in &tools {
            self.variables.add_tool(&tool.name);
        }
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
        self.variables.in_plan_mode = enabled;
        self
    }

    /// Set plan file path
    pub fn with_plan_file(mut self, path: impl Into<String>, exists: bool) -> Self {
        let path = path.into();
        self.plan_file_path = Some(path.clone());
        self.plan_exists = exists;
        self.variables.plan_file_path = path;
        self.variables.plan_exists = exists;
        self
    }

    /// Include Git instructions
    pub fn with_git_instructions(mut self, include: bool) -> Self {
        self.include_git_instructions = include;
        self
    }

    /// Include security policy
    pub fn with_security_policy(mut self, include: bool) -> Self {
        self.include_security_policy = include;
        self
    }

    /// Add a custom section
    pub fn with_custom_section(
        mut self,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        self.custom_sections.push((title.into(), content.into()));
        self
    }

    /// Set a custom variable
    pub fn with_variable(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.set(key, value);
        self
    }

    /// Set platform info
    pub fn with_platform(
        mut self,
        platform: impl Into<String>,
        os_version: impl Into<String>,
    ) -> Self {
        self.variables.platform = platform.into();
        self.variables.os_version = os_version.into();
        self
    }

    /// Build the tools description string with detailed descriptions
    fn build_tools_description(&self) -> String {
        if self.tools.is_empty() {
            return "No tools available.".to_string();
        }

        self.tools
            .iter()
            .map(|schema| {
                // Try to get detailed description from ToolDescriptions
                let description = ToolDescriptions::for_tool(&schema.name)
                    .map(|d| TemplateRenderer::render(d, &self.variables))
                    .unwrap_or_else(|| schema.description.clone());

                format!("## {}\n{}", schema.name, description)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Build the reminders section
    fn build_reminders(&self) -> String {
        let mut reminders = self.reminders.clone();

        // Add plan mode reminder if in plan mode
        if self.in_plan_mode {
            if let Some(plan_path) = &self.plan_file_path {
                reminders.insert(
                    0,
                    SystemReminder::plan_mode_active(plan_path, self.plan_exists),
                );
            }
        }

        if reminders.is_empty() {
            return String::new();
        }

        reminders
            .iter()
            .map(|r| TemplateRenderer::render(&r.to_prompt_string(), &self.variables))
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

    /// Build the security and Git sections
    fn build_additional_sections(&self) -> String {
        let mut sections = Vec::new();

        if self.include_security_policy {
            sections.push(SecurityPolicy::MAIN.to_string());
        }

        if self.include_git_instructions && self.variables.is_git_repo {
            sections.push(GitPrompts::SAFETY_PROTOCOL.to_string());
            sections.push(GitPrompts::PR_CREATION.to_string());
        }

        if sections.is_empty() {
            return String::new();
        }

        sections
            .iter()
            .map(|s| TemplateRenderer::render(s, &self.variables))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Build the complete system prompt
    pub fn build(&self) -> String {
        // Start with the main system prompt template
        let main_prompt = SystemPrompt::build_main_prompt();

        // Render with variables
        let mut prompt = TemplateRenderer::render(&main_prompt, &self.variables);

        // Add tools section
        let tools_desc = self.build_tools_description();
        if !tools_desc.is_empty() {
            prompt.push_str("\n\n# Available Tools\n\n");
            prompt.push_str(&tools_desc);
        }

        // Add Git and security sections
        let additional = self.build_additional_sections();
        if !additional.is_empty() {
            prompt.push_str("\n\n");
            prompt.push_str(&additional);
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

    /// Build a prompt for a specific agent type
    pub fn build_for_agent(&self, agent_type: &str) -> String {
        use super::agent_prompts::AgentPrompts;

        let agent_prompt =
            AgentPrompts::for_agent_type(agent_type).unwrap_or(AgentPrompts::GENERAL_PURPOSE);

        let mut prompt = TemplateRenderer::render(agent_prompt, &self.variables);

        // Add tools if not a read-only agent
        if !AgentPrompts::is_read_only(agent_type) {
            let tools_desc = self.build_tools_description();
            if !tools_desc.is_empty() {
                prompt.push_str("\n\n# Available Tools\n\n");
                prompt.push_str(&tools_desc);
            }
        }

        // Add reminders
        let reminders = self.build_reminders();
        if !reminders.is_empty() {
            prompt.push_str("\n\n");
            prompt.push_str(&reminders);
        }

        prompt
    }

    /// Get a reference to the variables
    pub fn variables(&self) -> &PromptVariables {
        &self.variables
    }

    /// Get a mutable reference to the variables
    pub fn variables_mut(&mut self) -> &mut PromptVariables {
        &mut self.variables
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let prompt = SystemPromptBuilder::new()
            .with_agent_name("Test Agent")
            .with_task("Test task")
            .with_working_dir("/test")
            .build();

        assert!(prompt.contains("Test Agent"));
        assert!(prompt.contains("Tone and style"));
        assert!(prompt.contains("Professional objectivity"));
    }

    #[test]
    fn test_builder_with_tools() {
        let tools = vec![
            ToolSchema::new("Read", "Reads files", vec![]),
            ToolSchema::new("Write", "Writes files", vec![]),
        ];

        let prompt = SystemPromptBuilder::new().with_tools(tools).build();

        assert!(prompt.contains("Read"));
        assert!(prompt.contains("Write"));
        assert!(prompt.contains("Available Tools"));
    }

    #[test]
    fn test_builder_with_git_info() {
        let prompt = SystemPromptBuilder::new()
            .with_git_info(true, "feature-branch", "main")
            .build();

        assert!(prompt.contains("Yes")); // Is git repo
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
            .with_plan_file("/tmp/plan.md", false)
            .build();

        assert!(prompt.contains("Plan mode is active"));
        assert!(prompt.contains("Phase 1: Initial Understanding"));
    }

    #[test]
    fn test_builder_custom_section() {
        let prompt = SystemPromptBuilder::new()
            .with_custom_section("Custom Rules", "Follow these custom rules...")
            .build();

        assert!(prompt.contains("Custom Rules"));
        assert!(prompt.contains("Follow these custom rules"));
    }

    #[test]
    fn test_builder_agent_prompt() {
        let builder = SystemPromptBuilder::new().with_agent_name("Sage Agent");

        let explore_prompt = builder.build_for_agent("explore");
        assert!(explore_prompt.contains("file search specialist"));
        assert!(explore_prompt.contains("READ-ONLY"));
        assert!(explore_prompt.contains("Sage Agent"));

        let general_prompt = builder.build_for_agent("general");
        assert!(general_prompt.contains("general-purpose agent"));
    }

    #[test]
    fn test_builder_variable_rendering() {
        let prompt = SystemPromptBuilder::new()
            .with_agent_name("MyAgent")
            .with_model_name("claude-3")
            .with_working_dir("/home/user/project")
            .build();

        assert!(prompt.contains("MyAgent"));
        assert!(prompt.contains("/home/user/project"));
    }

    #[test]
    fn test_builder_with_security_policy() {
        let prompt = SystemPromptBuilder::new()
            .with_security_policy(true)
            .build();

        assert!(prompt.contains("security"));
    }

    #[test]
    fn test_builder_tools_registered_as_available() {
        let tools = vec![
            ToolSchema::new("Bash", "Execute commands", vec![]),
            ToolSchema::new("TodoWrite", "Manage tasks", vec![]),
        ];

        let builder = SystemPromptBuilder::new().with_tools(tools);

        assert!(builder.variables.has_tool("Bash"));
        assert!(builder.variables.has_tool("TodoWrite"));
    }
}
