//! Message building utilities for the unified executor

use crate::error::SageResult;
use crate::llm::messages::LlmMessage;
use crate::prompts::SystemPromptBuilder;
use std::path::PathBuf;
use tracing::instrument;

use super::UnifiedExecutor;
use super::context_builder::ContextBuilder;

impl UnifiedExecutor {
    /// Build the system prompt
    #[instrument(skip(self))]
    pub(super) async fn build_system_prompt(&self) -> SageResult<String> {
        let model_name = self
            .config
            .default_model_parameters()
            .map(|p| p.model.clone())
            .unwrap_or_else(|_| "unknown".to_string());

        let working_dir_path = self
            .options
            .working_directory
            .clone()
            .or_else(|| self.config.working_directory.clone())
            .unwrap_or_else(|| PathBuf::from("."));

        let working_dir = working_dir_path.to_string_lossy().to_string();

        // Load project context (CLAUDE.md, instructions, git info)
        let context_builder = ContextBuilder::new(&working_dir_path);
        let project_context = context_builder.build_context();

        // Get tool schemas to include in prompt - CRITICAL for AI to know what tools are available
        let tool_schemas = self.tool_orchestrator.tool_executor().get_tool_schemas();

        // Get skills prompt for AI auto-invocation (Claude Code compatible)
        let skills_prompt = {
            let registry = self.skill_registry.read().await;
            registry.generate_skill_tool_prompt()
        };

        // Build system prompt with context
        let mut builder = SystemPromptBuilder::new()
            .with_model_name(&model_name)
            .with_working_dir(&working_dir)
            .with_tools(tool_schemas)
            .with_skills_prompt(skills_prompt);

        // Add git info if available
        if let Some(ref git_info) = project_context.git_info {
            builder = builder.with_git_info(
                git_info.is_repo,
                git_info.branch.as_deref().unwrap_or("unknown"),
                git_info.main_branch.as_deref().unwrap_or("main"),
            );
        }

        // Add CLAUDE.md as custom section (if exists)
        if let Some(ref claude_md) = project_context.claude_md {
            builder = builder.with_custom_section("Project Instructions (CLAUDE.md)", claude_md);
        }

        // Add .sage/instructions.md as custom section (if exists)
        if let Some(ref instructions) = project_context.project_instructions {
            builder = builder.with_custom_section("Project Instructions", instructions);
        }

        let prompt = builder.build();
        Ok(prompt)
    }

    /// Build initial messages with system prompt and task
    pub(super) fn build_initial_messages(
        &self,
        system_prompt: &str,
        task_description: &str,
    ) -> Vec<LlmMessage> {
        vec![
            LlmMessage::system(system_prompt),
            LlmMessage::user(task_description),
        ]
    }
}
