//! Section builders for system prompt construction

use super::system_prompt::{GitPrompts, SecurityPolicy};
use super::system_reminders::SystemReminder;
use super::tool_descriptions::ToolDescriptions;
use super::variables::{PromptVariables, TemplateRenderer};
use crate::tools::types::ToolSchema;

/// Build the tools description string with detailed descriptions
pub(super) fn build_tools_description(tools: &[ToolSchema], variables: &PromptVariables) -> String {
    if tools.is_empty() {
        return "No tools available.".to_string();
    }

    tools
        .iter()
        .map(|schema| {
            let description = ToolDescriptions::for_tool(&schema.name)
                .map(|d| TemplateRenderer::render(d, variables))
                .unwrap_or_else(|| schema.description.clone());

            format!("## {}\n{}", schema.name, description)
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Build the reminders section
pub(super) fn build_reminders(
    reminders: &[SystemReminder],
    in_plan_mode: bool,
    plan_file_path: Option<&str>,
    plan_exists: bool,
    variables: &PromptVariables,
) -> String {
    let mut reminders = reminders.to_vec();

    if in_plan_mode {
        if let Some(plan_path) = plan_file_path {
            reminders.insert(
                0,
                SystemReminder::plan_mode_active(plan_path, plan_exists),
            );
        }
    }

    if reminders.is_empty() {
        return String::new();
    }

    reminders
        .iter()
        .map(|r| TemplateRenderer::render(&r.to_prompt_string(), variables))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Build custom sections
pub(super) fn build_custom_sections(sections: &[(String, String)]) -> String {
    if sections.is_empty() {
        return String::new();
    }

    sections
        .iter()
        .map(|(title, content)| format!("# {}\n{}", title, content))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Build the security and Git sections
pub(super) fn build_additional_sections(
    include_security: bool,
    include_git: bool,
    is_git_repo: bool,
    variables: &PromptVariables,
) -> String {
    let mut sections = Vec::new();

    if include_security {
        sections.push(SecurityPolicy::MAIN.to_string());
    }

    if include_git && is_git_repo {
        sections.push(GitPrompts::SAFETY_PROTOCOL.to_string());
        sections.push(GitPrompts::PR_CREATION.to_string());
    }

    if sections.is_empty() {
        return String::new();
    }

    sections
        .iter()
        .map(|s| TemplateRenderer::render(s, variables))
        .collect::<Vec<_>>()
        .join("\n\n")
}
