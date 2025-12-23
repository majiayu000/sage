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

use crate::agent::subagent::init_global_runner_from_config;
use crate::agent::{
    AgentExecution, AgentState, AgentStep, ExecutionError, ExecutionMode, ExecutionOptions,
    ExecutionOutcome,
};
use crate::config::model::Config;
use crate::config::provider::ProviderConfig;
use crate::error::{SageError, SageResult};
use crate::input::{
    AutoResponse, InputChannel, InputRequest, InputRequestKind, InputResponse, Question,
    QuestionOption,
};
use crate::interrupt::{global_interrupt_manager, reset_global_interrupt_manager};
use crate::llm::client::LLMClient;
use crate::llm::messages::LLMMessage;
use crate::llm::provider_types::{LLMProvider, TimeoutConfig};
use crate::prompts::SystemPromptBuilder;
use crate::session::{
    EnhancedMessage, EnhancedTokenUsage, EnhancedToolCall, FileSnapshotTracker,
    JsonlSessionStorage, MessageChainTracker, SessionContext, TodoItem,
};
use crate::tools::executor::ToolExecutor;
use crate::tools::types::ToolSchema;
use crate::trajectory::recorder::TrajectoryRecorder;
use crate::types::{Id, TaskMetadata};
use crate::ui::animation::AnimationState;
use crate::ui::{AnimationManager, DisplayManager};
use anyhow::Context;
use std::sync::Arc;
use tokio::select;
use tracing::instrument;
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
    /// JSONL session storage for enhanced messages
    jsonl_storage: Option<Arc<JsonlSessionStorage>>,
    /// Message chain tracker for building message relationships
    message_tracker: MessageChainTracker,
    /// Current session ID
    current_session_id: Option<String>,
    /// File snapshot tracker for undo capability
    file_tracker: FileSnapshotTracker,
}

impl UnifiedExecutor {
    /// Create a new unified executor with default options
    pub fn new(config: Config) -> SageResult<Self> {
        Self::with_options(config, ExecutionOptions::default())
    }

    /// Create a new unified executor with custom options
    pub fn with_options(config: Config, options: ExecutionOptions) -> SageResult<Self> {
        // Get default provider configuration
        let default_params = config.default_model_parameters()
            .context("Failed to retrieve default model parameters from configuration")?;
        let provider_name = config.get_default_provider();

        tracing::info!(
            "Creating unified executor with provider: {}, model: {}",
            provider_name,
            default_params.model
        );

        // Parse provider
        let provider: LLMProvider = provider_name
            .parse()
            .map_err(|_| SageError::config(format!("Invalid provider: {}", provider_name)))
            .context(format!("Failed to parse provider name '{}' into a valid LLM provider", provider_name))?;

        // Create provider config
        let mut provider_config = ProviderConfig::new(provider_name)
            .with_api_key(default_params.get_api_key().unwrap_or_default())
            .with_timeouts(TimeoutConfig::new().with_request_timeout_secs(60))
            .with_max_retries(3);

        // Apply custom base_url if configured
        if let Some(base_url) = &default_params.base_url {
            provider_config = provider_config.with_base_url(base_url.clone());
        }

        // Create model parameters
        let model_params = default_params.to_llm_parameters();

        // Create LLM client
        let llm_client = LLMClient::new(provider, provider_config, model_params)
            .context(format!("Failed to create LLM client for provider: {}", provider_name))?;

        // Create tool executor
        let tool_executor = ToolExecutor::new();

        // Create input channel based on mode
        let input_channel = match &options.mode {
            ExecutionMode::Interactive => None, // Will be set externally
            ExecutionMode::NonInteractive { auto_response } => {
                // Convert from agent::AutoResponse to input::AutoResponse
                let input_auto_response = match auto_response {
                    crate::agent::AutoResponse::Fixed(text) => {
                        let text = text.clone();
                        AutoResponse::Custom(std::sync::Arc::new(move |req: &InputRequest| {
                            InputResponse::text(req.id, text.clone())
                        }))
                    }
                    crate::agent::AutoResponse::FirstOption => AutoResponse::AlwaysAllow,
                    crate::agent::AutoResponse::LastOption => AutoResponse::AlwaysAllow,
                    crate::agent::AutoResponse::Cancel => AutoResponse::AlwaysDeny,
                    crate::agent::AutoResponse::ContextBased {
                        default_text,
                        prefer_first_option,
                    } => {
                        let text = default_text.clone();
                        let prefer_first = *prefer_first_option;
                        AutoResponse::Custom(std::sync::Arc::new(move |req: &InputRequest| {
                            match &req.kind {
                                InputRequestKind::Questions { questions } if prefer_first => {
                                    // Select first option for each question
                                    let answers: std::collections::HashMap<String, String> =
                                        questions
                                            .iter()
                                            .map(|q| {
                                                let answer = q
                                                    .options
                                                    .first()
                                                    .map(|o| o.label.clone())
                                                    .unwrap_or_default();
                                                (q.question.clone(), answer)
                                            })
                                            .collect();
                                    InputResponse::question_answers(req.id, answers)
                                }
                                InputRequestKind::Simple {
                                    options: Some(_), ..
                                } if prefer_first => {
                                    InputResponse::selected(req.id, 0, "auto-selected")
                                }
                                _ => InputResponse::text(req.id, text.clone()),
                            }
                        }))
                    }
                };
                Some(InputChannel::non_interactive(input_auto_response))
            }
            ExecutionMode::Batch => Some(InputChannel::fail_on_input()),
        };

        // Create animation manager
        let animation_manager = AnimationManager::new();

        // Create JSONL storage (optional, can be enabled later)
        let jsonl_storage = JsonlSessionStorage::default_path().ok().map(Arc::new);

        // Get working directory for context
        let working_dir = options
            .working_directory
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Create message chain tracker
        let context = SessionContext::new(working_dir);
        let message_tracker = MessageChainTracker::new().with_context(context);

        Ok(Self {
            id: uuid::Uuid::new_v4(),
            config,
            llm_client,
            tool_executor,
            options,
            input_channel,
            trajectory_recorder: None,
            animation_manager,
            jsonl_storage,
            message_tracker,
            current_session_id: None,
            file_tracker: FileSnapshotTracker::default_tracker(),
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

    /// Enable JSONL session recording
    ///
    /// Creates a new session and starts recording enhanced messages.
    pub async fn enable_session_recording(&mut self) -> SageResult<String> {
        let session_id = uuid::Uuid::new_v4().to_string();

        if let Some(storage) = &self.jsonl_storage {
            let working_dir = self
                .options
                .working_directory
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

            // Create session
            let mut metadata = storage
                .create_session(&session_id, working_dir.clone())
                .await
                .context(format!("Failed to create JSONL session with ID: {}", session_id))?;

            // Set model info
            if let Ok(params) = self.config.default_model_parameters() {
                metadata = metadata.with_model(&params.model);
            }
            storage.save_metadata(&session_id, &metadata).await
                .context(format!("Failed to save session metadata for session: {}", session_id))?;

            // Update tracker
            let mut context = SessionContext::new(working_dir);
            context.detect_git_branch();
            self.message_tracker = MessageChainTracker::new()
                .with_session(&session_id)
                .with_context(context);

            self.current_session_id = Some(session_id.clone());

            tracing::info!("Started JSONL session recording: {}", session_id);
        }

        Ok(self.current_session_id.clone().unwrap_or_default())
    }

    /// Get current session ID
    pub fn current_session_id(&self) -> Option<&str> {
        self.current_session_id.as_deref()
    }

    /// Record a user message
    async fn record_user_message(&mut self, content: &str) -> SageResult<Option<EnhancedMessage>> {
        if self.current_session_id.is_none() || self.jsonl_storage.is_none() {
            return Ok(None);
        }

        let msg = self.message_tracker.create_user_message(content);

        if let Some(storage) = &self.jsonl_storage {
            if let Some(session_id) = &self.current_session_id {
                storage.append_message(session_id, &msg).await?;
            }
        }

        Ok(Some(msg))
    }

    /// Record an assistant message
    async fn record_assistant_message(
        &mut self,
        content: &str,
        tool_calls: Option<Vec<EnhancedToolCall>>,
        usage: Option<EnhancedTokenUsage>,
    ) -> SageResult<Option<EnhancedMessage>> {
        if self.current_session_id.is_none() || self.jsonl_storage.is_none() {
            return Ok(None);
        }

        let mut msg = self.message_tracker.create_assistant_message(content);

        if let Some(calls) = tool_calls {
            msg = msg.with_tool_calls(calls);
        }
        if let Some(u) = usage {
            msg = msg.with_usage(u);
        }

        if let Some(storage) = &self.jsonl_storage {
            if let Some(session_id) = &self.current_session_id {
                storage.append_message(session_id, &msg).await?;
            }
        }

        Ok(Some(msg))
    }

    /// Update todos in the message tracker
    pub fn update_todos(&mut self, todos: Vec<TodoItem>) {
        self.message_tracker.set_todos(todos);
    }

    /// Track a file for snapshot capability
    ///
    /// Call this before modifying files to enable undo.
    pub async fn track_file(&mut self, path: impl AsRef<std::path::Path>) -> SageResult<()> {
        self.file_tracker.track_file(path).await
    }

    /// Create and record a file snapshot for the current message
    async fn record_file_snapshot(&mut self, message_uuid: &str) -> SageResult<()> {
        if self.current_session_id.is_none() || self.jsonl_storage.is_none() {
            return Ok(());
        }

        // Only create snapshot if files were tracked
        if self.file_tracker.is_empty() {
            return Ok(());
        }

        let snapshot = self.file_tracker.create_snapshot(message_uuid).await?;

        if let Some(storage) = &self.jsonl_storage {
            if let Some(session_id) = &self.current_session_id {
                storage.append_snapshot(session_id, &snapshot).await?;
            }
        }

        // Clear tracker for next round
        self.file_tracker.clear();

        Ok(())
    }

    /// Get the file tracker for external file tracking
    pub fn file_tracker_mut(&mut self) -> &mut FileSnapshotTracker {
        &mut self.file_tracker
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

    /// Initialize sub-agent support
    ///
    /// This should be called after all tools are registered to enable
    /// the Task tool to execute sub-agents (Explore, Plan, etc.)
    pub fn init_subagent_support(&self) -> SageResult<()> {
        // Get all registered tools from the executor
        let tool_names = self.tool_executor.tool_names();
        let tools: Vec<Arc<dyn crate::tools::base::Tool>> = tool_names
            .iter()
            .filter_map(|name| self.tool_executor.get_tool(name).cloned())
            .collect();

        tracing::info!("Initializing sub-agent support with {} tools", tools.len());

        init_global_runner_from_config(&self.config, tools)
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

    /// Graceful shutdown - cleanup resources and save state
    ///
    /// This method should be called when the executor is shutting down
    /// to ensure all resources are properly cleaned up.
    pub async fn shutdown(&mut self) -> SageResult<()> {
        tracing::info!("Initiating graceful shutdown of UnifiedExecutor");

        // Stop any animations
        self.animation_manager.stop_animation().await;

        // Finalize trajectory recording if present
        if let Some(recorder) = &self.trajectory_recorder {
            tracing::debug!("Finalizing trajectory recording");
            let mut recorder_guard = recorder.lock().await;
            if let Err(e) = recorder_guard.finalize_recording(false, Some("Shutdown".to_string())).await {
                tracing::warn!("Failed to finalize trajectory recording: {}", e);
            }
        }

        // Log session cleanup
        if let Some(session_id) = &self.current_session_id {
            tracing::debug!("Session {} shutdown complete", session_id);
        }

        tracing::info!("Graceful shutdown complete");
        Ok(())
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
    #[instrument(skip(self), fields(task_id = %task.id, task_description = %task.description))]
    pub async fn execute(&mut self, task: TaskMetadata) -> SageResult<ExecutionOutcome> {
        // Reset interrupt manager at start of execution
        reset_global_interrupt_manager();

        // Create a task scope for interrupt handling
        let task_scope = global_interrupt_manager().lock().create_task_scope();

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

        // Record initial user message if session recording is enabled
        if self.current_session_id.is_some() {
            let _ = self.record_user_message(&task.description).await;
        }

        // Start the unified execution loop
        let provider_name = self.config.get_default_provider().to_string();
        let max_steps = self.options.max_steps;

        let outcome = 'execution_loop: {
            let mut step_number = 0u32;
            loop {
                step_number += 1;

                // Check max_steps limit (None = unlimited)
                if let Some(max) = max_steps {
                    if step_number > max {
                        tracing::warn!("Reached maximum steps: {}", max);
                        execution.complete(false, Some("Reached maximum steps".to_string()));
                        break 'execution_loop ExecutionOutcome::MaxStepsReached { execution };
                    }
                }

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

                        // Record assistant message in JSONL session
                        if self.current_session_id.is_some() {
                            if let Some(ref llm_response) = step.llm_response {
                                // Convert tool calls if any exist
                                let tool_calls = if !llm_response.tool_calls.is_empty() {
                                    Some(
                                        llm_response
                                            .tool_calls
                                            .iter()
                                            .map(|c| EnhancedToolCall {
                                                id: c.id.clone(),
                                                name: c.name.clone(),
                                                arguments: serde_json::to_value(&c.arguments)
                                                    .unwrap_or(serde_json::Value::Object(
                                                        Default::default(),
                                                    )),
                                            })
                                            .collect(),
                                    )
                                } else {
                                    None
                                };

                                // Convert usage if available
                                let usage =
                                    llm_response.usage.as_ref().map(|u| EnhancedTokenUsage {
                                        input_tokens: u.prompt_tokens as u64,
                                        output_tokens: u.completion_tokens as u64,
                                        cache_read_tokens: u.cache_read_input_tokens.unwrap_or(0)
                                            as u64,
                                        cache_write_tokens: u
                                            .cache_creation_input_tokens
                                            .unwrap_or(0)
                                            as u64,
                                    });

                                // Record assistant message and get the message UUID
                                if let Ok(Some(msg)) = self
                                    .record_assistant_message(
                                        &llm_response.content,
                                        tool_calls,
                                        usage,
                                    )
                                    .await
                                {
                                    // Record file snapshot if files were tracked
                                    let _ = self.record_file_snapshot(&msg.uuid).await;
                                }
                            }
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

            // This is unreachable since the loop only exits via break statements
            // The compiler requires this branch for exhaustiveness
            #[allow(unreachable_code)]
            {
                unreachable!("Execution loop should exit via break statements")
            }
        };

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

        // Get tool schemas to include in prompt - CRITICAL for AI to know what tools are available
        let tool_schemas = self.tool_executor.get_tool_schemas();

        let prompt = SystemPromptBuilder::new()
            .with_model_name(&model_name)
            .with_working_dir(&working_dir)
            .with_tools(tool_schemas) // Include tool descriptions in prompt
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
        use crate::tools::types::ToolResult;

        // Stop animation while waiting for user input
        self.animation_manager.stop_animation().await;

        // Parse questions from the tool call arguments
        let questions_value = tool_call
            .arguments
            .get("questions")
            .ok_or_else(|| SageError::agent("ask_user_question missing 'questions' parameter"))?;

        // Build the input request from the questions
        let raw_questions: Vec<serde_json::Value> = serde_json::from_value(questions_value.clone())
            .map_err(|e| SageError::agent(format!("Invalid questions format: {}", e)))?;

        // Convert to Question structs
        let mut questions: Vec<Question> = Vec::new();
        let mut question_text = String::from("User Input Required:\n\n");

        for q in raw_questions.iter() {
            let question_str = q.get("question").and_then(|v| v.as_str()).unwrap_or("");
            let header = q
                .get("header")
                .and_then(|v| v.as_str())
                .unwrap_or("Question");
            let multi_select = q
                .get("multi_select")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            question_text.push_str(&format!("[{}] {}\n", header, question_str));

            let mut options: Vec<QuestionOption> = Vec::new();
            if let Some(opts) = q.get("options").and_then(|v| v.as_array()) {
                for (opt_idx, opt) in opts.iter().enumerate() {
                    let label = opt.get("label").and_then(|v| v.as_str()).unwrap_or("");
                    let description = opt
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    question_text.push_str(&format!(
                        "  {}. {}: {}\n",
                        opt_idx + 1,
                        label,
                        description
                    ));
                    options.push(QuestionOption::new(label, description));
                }
            }

            let mut question = Question::new(question_str, header, options);
            if multi_select {
                question = question.with_multi_select();
            }
            questions.push(question);
            question_text.push('\n');
        }

        // Create input request with structured questions
        let request = InputRequest::questions(questions);

        // Print the question
        println!("\n{}", question_text);

        // Block and wait for user input via InputChannel
        let response = self.request_user_input(request).await?;

        // Check if user cancelled
        if response.is_cancelled() {
            return Err(SageError::Cancelled);
        }

        // Format the response for the agent
        let result_text = if let Some(answers) = response.get_answers() {
            let answers_str: Vec<String> = answers
                .iter()
                .map(|(q, a)| format!("Q: {} -> A: {}", q, a))
                .collect();
            format!("User answered:\n{}", answers_str.join("\n"))
        } else if let Some(text) = response.get_text() {
            format!("User Response:\n\n{}", text)
        } else {
            "User provided no response".to_string()
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
        let cancellation_token = global_interrupt_manager().lock().cancellation_token();

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

        // Convert messages to JSON for recording
        let messages_json: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| serde_json::to_value(m).unwrap_or_default())
            .collect();

        // Add input messages and LLM response to step
        step = step
            .with_llm_messages(messages_json)
            .with_llm_response(llm_response.clone());

        // Process response
        let mut new_messages = messages.to_vec();

        // Display assistant response
        if !llm_response.content.is_empty() {
            println!("\n AI Response:");
            DisplayManager::print_markdown(&llm_response.content);
        }

        // Add assistant message with tool_calls if present
        // CRITICAL: The assistant message MUST include tool_calls for the subsequent
        // tool messages to reference via tool_call_id. OpenRouter/Anthropic API requires
        // each tool_result to have a corresponding tool_use in the previous message.
        if !llm_response.tool_calls.is_empty() || !llm_response.content.is_empty() {
            let mut assistant_msg = LLMMessage::assistant(&llm_response.content);
            if !llm_response.tool_calls.is_empty() {
                assistant_msg.tool_calls = Some(llm_response.tool_calls.clone());
            }
            new_messages.push(assistant_msg);
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

                // Track files before file-modifying tools execute (for undo capability)
                if matches!(tool_call.name.as_str(), "edit" | "write" | "multi_edit") {
                    if let Some(file_path) = tool_call
                        .arguments
                        .get("file_path")
                        .or_else(|| tool_call.arguments.get("path"))
                        .and_then(|v| v.as_str())
                    {
                        let _ = self.file_tracker.track_file(file_path).await;
                    }
                }

                // Check if this tool requires user interaction (blocking input)
                let requires_interaction = self
                    .tool_executor
                    .get_tool(&tool_call.name)
                    .map(|t| t.requires_user_interaction())
                    .unwrap_or(false);

                // Handle tools that require user interaction with blocking input
                let tool_result = if requires_interaction && tool_call.name == "ask_user_question" {
                    // Use specialized handler for ask_user_question
                    self.handle_ask_user_question(tool_call).await?
                } else if requires_interaction {
                    // Generic handling for other interactive tools
                    // For now, just execute normally - can be extended later
                    self.tool_executor.execute_tool(tool_call).await
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

    /// Set max steps (None = unlimited)
    pub fn with_max_steps(mut self, max_steps: Option<u32>) -> Self {
        self.options.max_steps = max_steps;
        self
    }

    /// Set a specific step limit
    pub fn with_step_limit(mut self, limit: u32) -> Self {
        self.options.max_steps = Some(limit);
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
        let options = ExecutionOptions::interactive().with_step_limit(50);
        assert_eq!(options.max_steps, Some(50));
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
