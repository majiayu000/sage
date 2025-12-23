//! Agent trait implementation for BaseAgent

use crate::agent::{Agent, AgentExecution, ExecutionOutcome};
use crate::config::model::Config;
use crate::error::SageResult;
use crate::interrupt::reset_global_interrupt_manager;
use crate::types::{Id, TaskMetadata};
use async_trait::async_trait;
use tracing::instrument;

use super::agent_impl::BaseAgent;
use super::continue_execution::continue_execution_impl;
use super::execution_loop::execute_task_loop;
use super::system_prompt::create_system_message;

#[async_trait]
impl Agent for BaseAgent {
    #[instrument(skip(self), fields(task_id = %task.id, task_description = %task.description, max_steps = %self.max_steps))]
    async fn execute_task(&mut self, task: TaskMetadata) -> SageResult<ExecutionOutcome> {
        tracing::info!("starting agent execution");
        let mut execution = AgentExecution::new(task.clone());

        // Reset the global interrupt manager for this new task
        reset_global_interrupt_manager();

        // Start trajectory recording if available
        if let Some(recorder) = &self.trajectory_recorder {
            let provider = self.config.get_default_provider().to_string();
            let model = self.config.default_model_parameters()?.model.clone();
            recorder
                .lock()
                .await
                .start_recording(task.clone(), provider, model, self.config.max_steps)
                .await?;
        }

        let tool_schemas = self.tool_executor.get_tool_schemas();
        let system_message = create_system_message(&task, &tool_schemas, &self.config);
        let provider_name = self.config.get_default_provider().to_string();

        // Main execution loop
        let final_outcome = execute_task_loop(
            &mut execution,
            &system_message,
            &tool_schemas,
            self.max_steps,
            &mut self.llm_client,
            &self.tool_executor,
            &mut self.animation_manager,
            &self.trajectory_recorder,
            &self.config,
            &provider_name,
        )
        .await;

        // Finalize trajectory recording
        if let Some(recorder) = &self.trajectory_recorder {
            recorder
                .lock()
                .await
                .finalize_recording(
                    final_outcome.is_success(),
                    final_outcome.execution().final_result.clone(),
                )
                .await?;
        }

        Ok(final_outcome)
    }

    fn config(&self) -> &Config {
        &self.config
    }

    fn id(&self) -> Id {
        self.id
    }

    async fn continue_execution(
        &mut self,
        execution: &mut AgentExecution,
        user_message: &str,
    ) -> SageResult<()> {
        let tool_schemas = self.tool_executor.get_tool_schemas();
        let system_message = create_system_message(&execution.task, &tool_schemas, &self.config);

        continue_execution_impl(
            execution,
            user_message,
            &system_message,
            &tool_schemas,
            self.max_steps,
            &mut self.llm_client,
            &self.tool_executor,
            &mut self.animation_manager,
            &self.trajectory_recorder,
            &self.config,
        )
        .await
    }
}
