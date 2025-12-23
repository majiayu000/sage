//! Message building utilities for the unified executor

use crate::error::SageResult;
use crate::llm::messages::LlmMessage;
use crate::prompts::SystemPromptBuilder;
use tracing::instrument;

use super::UnifiedExecutor;

impl UnifiedExecutor {
    /// Build the system prompt
    #[instrument(skip(self))]
    pub(super) fn build_system_prompt(&self) -> SageResult<String> {
        let model_name = self
            .config
            .default_model_parameters()
            .map(|p| p.model.clone())
            .unwrap_or_else(|_| "unknown".to_string());

        let working_dir = self
            .options
            .working_directory
            .as_ref()
            .or(self.config.working_directory.as_ref())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());

        // Get tool schemas to include in prompt - CRITICAL for AI to know what tools are available
        let tool_schemas = self.tool_executor.get_tool_schemas();

        let prompt = SystemPromptBuilder::new()
            .with_model_name(&model_name)
            .with_working_dir(&working_dir)
            .with_tools(tool_schemas) // Include tool descriptions in prompt
            .build();

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
