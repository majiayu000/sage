//! Unified executor for the agent execution loop
//!
//! This module implements the Claude Code style unified execution loop where:
//! - There's no distinction between "run" and "interactive" modes at the core level
//! - User input is handled via InputChannel which blocks within the loop
//! - The loop never exits for user input - it waits inline
//!
//! # Design
//!
//! ```text
//! User Input → execute_task(options, input_channel) → Execution Loop
//!     → Tool calls (including ask_user_question) → Block on InputChannel
//!     → User responds → Loop continues (no exit/resume)
//!     → Task completes → Return ExecutionOutcome
//! ```

use crate::agent::{
    AgentExecution, AgentState, AgentStep, ExecutionError, ExecutionMode, ExecutionOptions,
    ExecutionOutcome,
};
use crate::config::model::Config;
use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::input::{InputChannel, InputRequest, InputResponse};
use crate::interrupt::{global_interrupt_manager, reset_global_interrupt_manager};
use crate::llm::client::LLMClient;
use crate::llm::messages::LLMMessage;
use crate::llm::providers::LLMProvider;
use crate::prompts::SystemPromptBuilder;
use crate::tools::executor::ToolExecutor;
use crate::tools::types::ToolSchema;
use crate::trajectory::recorder::TrajectoryRecorder;
use crate::types::{Id, TaskMetadata};
use crate::ui::animation::AnimationState;
use crate::ui::{AnimationManager, DisplayManager};
use std::sync::Arc;
use tokio::select;
use tokio::sync::Mutex;

/// Unified executor that implements the Claude Code style execution loop
pub struct UnifiedExecutor {
    /// Unique identifier
    id: Id,
    /// Configuration
    config: Config,
    /// LLM client for model interactions
    llm_client: LLMClient,
    /// Tool executor for running tools
    tool_executor: ToolExecutor,
    /// Execution options
    options: ExecutionOptions,
    /// Input channel for blocking user input (None for batch mode)
    input_channel: Option<InputChannel>,
    /// Trajectory recorder
    trajectory_recorder: Option<Arc<Mutex<TrajectoryRecorder>>>,
    /// Animation manager
    animation_manager: AnimationManager,
}

impl UnifiedExecutor {
    /// Create a new unified executor with default options
    pub fn new(config: Config) -> SageResult<Self> {
        Self::with_options(config, ExecutionOptions::default())
    }

    /// Create a new unified executor with custom options
    pub fn with_options(config: Config, options: ExecutionOptions) -> SageResult<Self> {
        // Get default provider configuration
        let default_params = config.default_model_parameters()?;
        let provider_name = config.get_default_provider();

        tracing::info!(
            "Creating unified executor with provider: {}, model: {}",
            provider_name,
            default_params.model
        );

        // Parse provider
        let provider: LLMProvider = provider_name
            .parse()
            .map_err(|_| SageError::config(format!("Invalid provider: {}", provider_name)))?;

        // Create provider config
        let mut provider_config = ProviderConfig::new(provider_name)
            .with_api_key(default_params.get_api_key().unwrap_or_default())
            .with_timeout(60)
            .with_max_retries(3);

        // Apply custom base_url if configured
        if let Some(base_url) = &default_params.base_url {
            provider_config = provider_config.with_base_url(base_url.clone());
        }

        // Create model parameters
        let model_params = default_params.to_llm_parameters();

        // Create LLM client
        let llm_client = LLMClient::new(provider, provider_config, model_params)?;

        // Create tool executor
        let tool_executor = ToolExecutor::new();

        // Create input channel based on mode
        let input_channel = match &options.mode {
            ExecutionMode::Interactive => None, // Will be set externally
            ExecutionMode::NonInteractive { auto_response } => {
                let response = auto_response.get_text_response().to_string();
                Some(InputChannel::non_interactive(response))
            }
            ExecutionMode::Batch => Some(InputChannel::fail_on_input()),
        };

        // Create animation manager
        let animation_manager = AnimationManager::new();

        Ok(Self {
            id: uuid::Uuid::new_v4(),
            config,
            llm_client,
            tool_executor,
            options,
            input_channel,
            trajectory_recorder: None,
            animation_manager,
        })
    }

    /// Set the input channel for interactive mode
    pub fn set_input_channel(&mut self, channel: InputChannel) {
        self.input_channel = Some(channel);
    }

    /// Set trajectory recorder
    pub fn set_trajectory_recorder(&mut self, recorder: Arc<Mutex<TrajectoryRecorder>>) {
        self.trajectory_recorder = Some(recorder);
    }

    /// Register a tool with the executor
    pub fn register_tool(&mut self, tool: Arc<dyn crate::tools::base::Tool>) {
        self.tool_executor.register_tool(tool);
    }

    /// Register multiple tools with the executor
    pub fn register_tools(&mut self, tools: Vec<Arc<dyn crate::tools::base::Tool>>) {
        for tool in tools {
            self.tool_executor.register_tool(tool);
        }
    }

    /// Get the executor ID
    pub fn id(&self) -> Id {
        self.id.clone()
    }

    /// Get configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get execution options
    pub fn options(&self) -> &ExecutionOptions {
        &self.options
    }

    /// Request user input via the input channel
    ///
    /// This method blocks until the user responds (or auto-responds in non-interactive mode).
    /// If no input channel is set (batch mode without channel), returns an error.
    pub async fn request_user_input(&mut self, request: InputRequest) -> SageResult<InputResponse> {
        match &mut self.input_channel {
            Some(channel) => channel.request_input(request).await,
            None => Err(SageError::agent(
                "No input channel configured - cannot request user input",
            )),
        }
    }

    /// Execute a task with the unified execution loop
    ///
    /// This is the main execution method that implements the Claude Code style loop:
    /// - Never exits for user input
    /// - Blocks inline on InputChannel when needed
    /// - Returns only on completion, failure, interrupt, or max steps
    pub async fn execute(&mut self, task: TaskMetadata) -> SageResult<ExecutionOutcome> {
        // Reset interrupt manager at start of execution
        reset_global_interrupt_manager();

        // Create a task scope for interrupt handling
        let task_scope = global_interrupt_manager()
            .lock()
            .map_err(|_| SageError::agent("Failed to acquire interrupt manager lock"))?
            .create_task_scope();

        // Initialize execution state
        let mut execution = AgentExecution::new(task.clone());

        // Start trajectory recording if available
        if let Some(recorder) = &self.trajectory_recorder {
            let provider = self.config.get_default_provider().to_string();
            let model = self.config.default_model_parameters()?.model.clone();
            recorder
                .lock()
                .await
                .start_recording(task.clone(), provider, model, self.options.max_steps)
                .await?;
        }

        // Build system prompt
        let system_prompt = self.build_system_prompt()?;

        // Get tool schemas
        let tool_schemas = self.tool_executor.get_tool_schemas();

        // Initialize conversation with system prompt and task
        let mut messages = self.build_initial_messages(&system_prompt, &task.description);

        // Start the unified execution loop
        let provider_name = self.config.get_default_provider().to_string();
        let max_steps = self.options.max_steps;

        let outcome = 'execution_loop: {
            for step_number in 1..=max_steps {
                // Check for interrupt before each step
                if task_scope.is_cancelled() {
                    self.animation_manager.stop_animation().await;
                    DisplayManager::print_separator("Task Interrupted", "yellow");
                    execution.complete(false, Some("Interrupted by user".to_string()));
                    break 'execution_loop ExecutionOutcome::Interrupted { execution };
                }

                // Execute one step
                match self
                    .execute_step(step_number, &messages, &tool_schemas, &task_scope)
                    .await
                {
                    Ok((step, new_messages)) => {
                        let is_completed = step.state == AgentState::Completed;

                        // Record step in trajectory
                        if let Some(recorder) = &self.trajectory_recorder {
                            recorder.lock().await.record_step(step.clone()).await?;
                        }

                        execution.add_step(step);

                        // Update messages for next iteration
                        messages = new_messages;

                        if is_completed {
                            execution
                                .complete(true, Some("Task completed successfully".to_string()));
                            break 'execution_loop ExecutionOutcome::Success(execution);
                        }
                    }
                    Err(e) => {
                        self.animation_manager.stop_animation().await;

                        // Check if this is a user cancellation
                        if matches!(e, SageError::Cancelled) {
                            execution.complete(false, Some("Cancelled by user".to_string()));
                            break 'execution_loop ExecutionOutcome::UserCancelled {
                                execution,
                                pending_question: None,
                            };
                        }

                        let error_step = AgentStep::new(step_number, AgentState::Error)
                            .with_error(e.to_string());

                        if let Some(recorder) = &self.trajectory_recorder {
                            recorder
                                .lock()
                                .await
                                .record_step(error_step.clone())
                                .await?;
                        }

                        execution.add_step(error_step);
                        execution.complete(false, Some(format!("Task failed: {}", e)));

                        let exec_error =
                            ExecutionError::from_sage_error(&e, Some(provider_name.clone()));
                        break 'execution_loop ExecutionOutcome::Failed {
                            execution,
                            error: exec_error,
                        };
                    }
                }
            }

            // Reached max steps
            tracing::warn!("Reached maximum steps: {}", max_steps);
            execution.complete(false, Some("Reached maximum steps".to_string()));
            ExecutionOutcome::MaxStepsReached { execution }
        };

        // Stop any running animations
        self.animation_manager.stop_animation().await;

        // Finalize trajectory recording
        if let Some(recorder) = &self.trajectory_recorder {
            recorder
                .lock()
                .await
                .finalize_recording(outcome.is_success(), outcome.execution().final_result.clone())
                .await?;
        }

        Ok(outcome)
    }

    /// Build the system prompt
    fn build_system_prompt(&self) -> SageResult<String> {
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

        let prompt = SystemPromptBuilder::new()
            .with_model_name(&model_name)
            .with_working_dir(&working_dir)
            .build();

        Ok(prompt)
    }

    /// Handle ask_user_question tool call with blocking input
    ///
    /// This method intercepts ask_user_question tool calls and uses the InputChannel
    /// to actually block and wait for user input, implementing the unified loop pattern.
    async fn handle_ask_user_question(
        &mut self,
        tool_call: &crate::tools::types::ToolCall,
    ) -> SageResult<crate::tools::types::ToolResult> {
        use crate::input::{InputContext, InputOption};
        use crate::tools::types::ToolResult;

        // Stop animation while waiting for user input
        self.animation_manager.stop_animation().await;

        // Parse questions from the tool call arguments
        let questions_value = tool_call.arguments.get("questions").ok_or_else(|| {
            SageError::agent("ask_user_question missing 'questions' parameter")
        })?;

        // Build the input request from the questions
        let questions: Vec<serde_json::Value> =
            serde_json::from_value(questions_value.clone()).map_err(|e| {
                SageError::agent(format!("Invalid questions format: {}", e))
            })?;

        // Format question display text
        let mut question_text = String::from("User Input Required:\n\n");
        let mut all_options = Vec::new();

        for (idx, q) in questions.iter().enumerate() {
            if let Some(question_str) = q.get("question").and_then(|v| v.as_str()) {
                let header = q
                    .get("header")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Question");
                question_text.push_str(&format!("[{}] {}\n", header, question_str));

                if let Some(options) = q.get("options").and_then(|v| v.as_array()) {
                    for (opt_idx, opt) in options.iter().enumerate() {
                        let label = opt.get("label").and_then(|v| v.as_str()).unwrap_or("");
                        let description =
                            opt.get("description").and_then(|v| v.as_str()).unwrap_or("");
                        question_text.push_str(&format!("  {}. {}: {}\n", opt_idx + 1, label, description));
                        all_options.push(InputOption::new(label, description));
                    }
                }
                question_text.push('\n');
            }
        }

        // Create input request
        let request = InputRequest::new(&question_text)
            .with_context(InputContext::Decision)
            .with_options(all_options);

        // Print the question
        println!("\n{}", question_text);

        // Block and wait for user input via InputChannel
        let response = self.request_user_input(request).await?;

        // Check if user cancelled
        if response.cancelled {
            return Err(SageError::Cancelled);
        }

        // Format the response for the agent
        let result_text = if response.selected_indices.is_some() {
            format!("User Response:\n\nSelected: {}", response.content)
        } else {
            format!("User Response:\n\n{}", response.content)
        };

        Ok(ToolResult::success(
            &tool_call.id,
            "ask_user_question",
            result_text,
        ))
    }

    /// Build initial messages with system prompt and task
    fn build_initial_messages(
        &self,
        system_prompt: &str,
        task_description: &str,
    ) -> Vec<LLMMessage> {
        vec![
            LLMMessage::system(system_prompt),
            LLMMessage::user(task_description),
        ]
    }

    /// Execute a single step in the loop
    async fn execute_step(
        &mut self,
        step_number: u32,
        messages: &[LLMMessage],
        tool_schemas: &[ToolSchema],
        task_scope: &crate::interrupt::TaskScope,
    ) -> SageResult<(AgentStep, Vec<LLMMessage>)> {
        // Print step separator
        DisplayManager::print_separator(&format!("Step {} - AI Thinking", step_number), "blue");

        let mut step = AgentStep::new(step_number, AgentState::Thinking);

        // Start thinking animation
        self.animation_manager
            .start_animation(AnimationState::Thinking, "Thinking", "blue")
            .await;

        // Get cancellation token for interrupt handling
        let cancellation_token = global_interrupt_manager()
            .lock()
            .map_err(|_| SageError::agent("Failed to acquire interrupt manager lock"))?
            .cancellation_token();

        // Execute LLM call with interrupt support
        let llm_response = select! {
            response = self.llm_client.chat(messages, Some(tool_schemas)) => {
                response?
            }
            _ = cancellation_token.cancelled() => {
                self.animation_manager.stop_animation().await;
                return Err(SageError::agent("Task interrupted during LLM call"));
            }
        };

        // Stop animation
        self.animation_manager.stop_animation().await;

        // Add LLM response to step
        step = step.with_llm_response(llm_response.clone());

        // Process response
        let mut new_messages = messages.to_vec();

        // Display assistant response
        if !llm_response.content.is_empty() {
            println!("\n AI Response:");
            DisplayManager::print_markdown(&llm_response.content);
            new_messages.push(LLMMessage::assistant(&llm_response.content));
        }

        // Handle tool calls
        if !llm_response.tool_calls.is_empty() {
            // Start tool animation
            self.animation_manager
                .start_animation(AnimationState::ExecutingTools, "Executing tools", "green")
                .await;

            for tool_call in &llm_response.tool_calls {
                // Check for interrupt before each tool
                if task_scope.is_cancelled() {
                    self.animation_manager.stop_animation().await;
                    return Err(SageError::agent("Task interrupted during tool execution"));
                }

                // Special handling for ask_user_question - this is the key to unified loop
                let tool_result = if tool_call.name == "ask_user_question" {
                    self.handle_ask_user_question(tool_call).await?
                } else {
                    // Normal tool execution
                    self.tool_executor.execute_tool(tool_call).await
                };

                step.tool_results.push(tool_result.clone());

                // Add tool result to messages using LLMMessage::tool
                let tool_name = Some(tool_call.name.clone());
                new_messages.push(LLMMessage::tool(
                    tool_result.output.clone().unwrap_or_default(),
                    tool_call.id.clone(),
                    tool_name,
                ));
            }

            self.animation_manager.stop_animation().await;
            step.state = AgentState::ToolExecution;
        }

        // Check for completion indicator in response
        if llm_response.finish_reason == Some("end_turn".to_string())
            && llm_response.tool_calls.is_empty()
        {
            step.state = AgentState::Completed;
        }

        Ok((step, new_messages))
    }
}

/// Builder for creating unified executors with fluent API
pub struct UnifiedExecutorBuilder {
    config: Config,
    options: ExecutionOptions,
    input_channel: Option<InputChannel>,
    trajectory_recorder: Option<Arc<Mutex<TrajectoryRecorder>>>,
}

impl UnifiedExecutorBuilder {
    /// Create a new builder with configuration
    pub fn new(config: Config) -> Self {
        Self {
            config,
            options: ExecutionOptions::default(),
            input_channel: None,
            trajectory_recorder: None,
        }
    }

    /// Set execution options
    pub fn with_options(mut self, options: ExecutionOptions) -> Self {
        self.options = options;
        self
    }

    /// Set execution mode
    pub fn with_mode(mut self, mode: ExecutionMode) -> Self {
        self.options.mode = mode;
        self
    }

    /// Set input channel for interactive mode
    pub fn with_input_channel(mut self, channel: InputChannel) -> Self {
        self.input_channel = Some(channel);
        self
    }

    /// Set trajectory recorder
    pub fn with_trajectory_recorder(mut self, recorder: Arc<Mutex<TrajectoryRecorder>>) -> Self {
        self.trajectory_recorder = Some(recorder);
        self
    }

    /// Set max steps
    pub fn with_max_steps(mut self, max_steps: u32) -> Self {
        self.options.max_steps = max_steps;
        self
    }

    /// Set working directory
    pub fn with_working_directory(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.options.working_directory = Some(path.into());
        self
    }

    /// Build the unified executor
    pub fn build(self) -> SageResult<UnifiedExecutor> {
        let mut executor = UnifiedExecutor::with_options(self.config, self.options)?;

        if let Some(channel) = self.input_channel {
            executor.set_input_channel(channel);
        }

        if let Some(recorder) = self.trajectory_recorder {
            executor.set_trajectory_recorder(recorder);
        }

        Ok(executor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_executor_builder() {
        // This test would need a valid config, so we just test the builder pattern
        let options = ExecutionOptions::interactive().with_max_steps(50);
        assert_eq!(options.max_steps, 50);
        assert!(options.is_interactive());
    }

    #[test]
    fn test_execution_modes() {
        let interactive = ExecutionMode::interactive();
        assert!(interactive.is_interactive());

        let non_interactive = ExecutionMode::non_interactive();
        assert!(non_interactive.is_non_interactive());

        let batch = ExecutionMode::batch();
        assert!(batch.is_batch());
    }
}
