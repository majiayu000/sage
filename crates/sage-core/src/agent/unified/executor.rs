//! Main execution orchestration for the unified executor

use crate::agent::{AgentExecution, ExecutionOutcome};
use crate::error::SageResult;
use crate::interrupt::{global_interrupt_manager, reset_global_interrupt_manager};
use crate::types::TaskMetadata;
use anyhow::Context;
use tracing::instrument;

use super::UnifiedExecutor;

impl UnifiedExecutor {
    /// Execute a task with the unified execution loop
    ///
    /// This is the main execution method that implements the Claude Code style loop:
    /// - Never exits for user input
    /// - Blocks inline on InputChannel when needed
    /// - Returns only on completion, failure, interrupt, or max steps
    #[instrument(skip(self), fields(task_id = %task.id, task_description = %task.description))]
    pub async fn execute(&mut self, task: TaskMetadata) -> SageResult<ExecutionOutcome> {
        // Reset interrupt manager at start of execution
        reset_global_interrupt_manager();

        // Create a task scope for interrupt handling
        let task_scope = global_interrupt_manager().lock().create_task_scope();

        // Initialize execution state
        let execution = AgentExecution::new(task.clone());

        // Start trajectory recording if available
        if let Some(recorder) = &self.trajectory_recorder {
            let provider = self.config.get_default_provider().to_string();
            let model = self.config.default_model_parameters()?.model.clone();
            recorder
                .lock()
                .await
                .start_recording(task.clone(), provider, model, self.options.max_steps)
                .await
                .context("Failed to start trajectory recording")?;
        }

        // Build system prompt
        let system_prompt = self.build_system_prompt()?;

        // Get tool schemas
        let tool_schemas = self.tool_executor.get_tool_schemas();

        // Initialize conversation with system prompt and task
        let messages = self.build_initial_messages(&system_prompt, &task.description);

        // Record initial user message if session recording is enabled
        if self.current_session_id.is_some() {
            let _ = self.record_user_message(&task.description).await;
        }

        // Start the unified execution loop
        let provider_name = self.config.get_default_provider().to_string();
        let max_steps = self.options.max_steps;

        let outcome = self
            .run_execution_loop(
                execution,
                messages,
                tool_schemas,
                task_scope,
                provider_name,
                max_steps,
            )
            .await?;

        // Stop any running animations
        self.animation_manager.stop_animation().await;

        // Finalize trajectory recording
        if let Some(recorder) = &self.trajectory_recorder {
            recorder
                .lock()
                .await
                .finalize_recording(
                    outcome.is_success(),
                    outcome.execution().final_result.clone(),
                )
                .await?;
        }

        Ok(outcome)
    }
}
