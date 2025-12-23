//! Base agent implementation

use crate::agent::{AgentExecution, AgentState, AgentStep, ExecutionError, ExecutionOutcome};
use crate::config::model::Config;
use crate::error::{SageError, SageResult};
use crate::interrupt::{global_interrupt_manager, reset_global_interrupt_manager};
use crate::llm::client::LLMClient;
use crate::llm::messages::LLMMessage;
use crate::llm::provider_types::{LLMProvider, TimeoutConfig};
use crate::prompts::SystemPromptBuilder;
use crate::tools::executor::ToolExecutor;
use crate::tools::types::ToolSchema;
use crate::trajectory::recorder::TrajectoryRecorder;
use crate::types::{Id, TaskMetadata};
use crate::ui::animation::AnimationState;
use crate::ui::{AnimationManager, DisplayManager};
use anyhow::Context;
use async_trait::async_trait;
use colored::*;
use std::sync::Arc;
use tokio::select;
use tokio::sync::Mutex;
use tracing::instrument;

/// Model identity information for system prompt
#[derive(Debug, Clone)]
#[allow(dead_code)] // Reserved for future system prompt customization
struct ModelIdentity {
    base_model_info: String,
    model_name: String,
}

/// Base agent trait
#[async_trait]
pub trait Agent: Send + Sync {
    /// Execute a task and return an explicit outcome
    ///
    /// Returns `ExecutionOutcome` which clearly indicates success, failure,
    /// interruption, or max steps reached, while preserving the full execution trace.
    async fn execute_task(&mut self, task: TaskMetadata) -> SageResult<ExecutionOutcome>;

    /// Continue an existing execution with new user message
    async fn continue_execution(
        &mut self,
        execution: &mut AgentExecution,
        user_message: &str,
    ) -> SageResult<()>;

    /// Get the agent's configuration
    fn config(&self) -> &Config;

    /// Get the agent's ID
    fn id(&self) -> Id;
}

/// Base agent implementation
pub struct BaseAgent {
    id: Id,
    config: Config,
    llm_client: LLMClient,
    tool_executor: ToolExecutor,
    trajectory_recorder: Option<Arc<Mutex<TrajectoryRecorder>>>,
    max_steps: u32,
    animation_manager: AnimationManager,
}

impl BaseAgent {
    /// Check if content contains markdown formatting
    pub fn is_markdown_content(content: &str) -> bool {
        // Simple heuristics to detect markdown content
        content.contains("# ") ||           // Headers
        content.contains("## ") ||          // Headers
        content.contains("### ") ||         // Headers
        content.contains("* ") ||           // Lists
        content.contains("- ") ||           // Lists
        content.contains("```") ||          // Code blocks
        content.contains("`") ||            // Inline code
        content.contains("**") ||           // Bold
        content.contains("*") ||            // Italic
        content.contains("[") && content.contains("](") || // Links
        content.contains("> ") ||           // Blockquotes
        content.lines().count() > 3 // Multi-line content is likely markdown
    }

    /// Create a new base agent
    pub fn new(config: Config) -> SageResult<Self> {
        // Get default provider configuration
        let default_params = config.default_model_parameters()
            .context("Failed to retrieve default model parameters from configuration")?;
        let provider_name = config.get_default_provider();

        // Debug logging
        tracing::info!("Creating agent with provider: {}", provider_name);
        tracing::info!("Model: {}", default_params.model);
        tracing::info!("API key set: {}", default_params.api_key.is_some());

        // Parse provider
        let provider: LLMProvider = provider_name
            .parse()
            .map_err(|_| SageError::config(format!("Invalid provider: {}", provider_name)))
            .context(format!("Failed to parse provider name '{}' into a valid LLM provider", provider_name))?;

        tracing::info!("Parsed provider: {:?}", provider);

        // Create provider config
        let mut provider_config = crate::config::provider::ProviderConfig::new(provider_name)
            .with_api_key(default_params.get_api_key().unwrap_or_default())
            .with_timeouts(TimeoutConfig::new().with_request_timeout_secs(60))
            .with_max_retries(3);

        // Apply custom base_url if configured (for OpenRouter, etc.)
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

        Ok(Self {
            id: uuid::Uuid::new_v4(),
            config,
            llm_client,
            tool_executor,
            trajectory_recorder: None,
            max_steps: 20,
            animation_manager: AnimationManager::new(),
        })
    }

    /// Set trajectory recorder
    pub fn set_trajectory_recorder(&mut self, recorder: Arc<Mutex<TrajectoryRecorder>>) {
        self.trajectory_recorder = Some(recorder);
    }

    /// Set tool executor
    pub fn set_tool_executor(&mut self, executor: ToolExecutor) {
        self.tool_executor = executor;
    }

    /// Set max steps
    pub fn set_max_steps(&mut self, max_steps: u32) {
        self.max_steps = max_steps;
    }

    // Note: Display methods moved to CLI layer for better separation of concerns

    /// Get model identity information based on current configuration
    fn get_model_identity(&self) -> ModelIdentity {
        let default_provider = self.config.get_default_provider();
        let default_params = crate::config::model::ModelParameters::default();
        let model_params = self
            .config
            .default_model_parameters()
            .unwrap_or(&default_params);

        match default_provider {
            "anthropic" => {
                let base_model_info = match model_params.model.as_str() {
                    "claude-3-sonnet-20240229" => "The base model is Claude 3 Sonnet by Anthropic.",
                    "claude-3-opus-20240229" => "The base model is Claude 3 Opus by Anthropic.",
                    "claude-3-haiku-20240307" => "The base model is Claude 3 Haiku by Anthropic.",
                    "claude-sonnet-4-20250514" => "The base model is Claude Sonnet 4 by Anthropic.",
                    _ => "The base model is Claude by Anthropic.",
                };
                ModelIdentity {
                    base_model_info: base_model_info.to_string(),
                    model_name: format!("{} by Anthropic", model_params.model),
                }
            }
            "openai" => {
                let base_model_info = match model_params.model.as_str() {
                    "gpt-4" => "The base model is GPT-4 by OpenAI.",
                    "gpt-4-turbo" => "The base model is GPT-4 Turbo by OpenAI.",
                    "gpt-3.5-turbo" => "The base model is GPT-3.5 Turbo by OpenAI.",
                    _ => "The base model is GPT by OpenAI.",
                };
                ModelIdentity {
                    base_model_info: base_model_info.to_string(),
                    model_name: format!("{} by OpenAI", model_params.model),
                }
            }
            "google" => {
                let base_model_info = match model_params.model.as_str() {
                    "gemini-2.5-pro" => "The base model is Gemini 2.5 Pro by Google.",
                    "gemini-1.5-pro" => "The base model is Gemini 1.5 Pro by Google.",
                    "gemini-1.0-pro" => "The base model is Gemini 1.0 Pro by Google.",
                    _ => "The base model is Gemini by Google.",
                };
                ModelIdentity {
                    base_model_info: base_model_info.to_string(),
                    model_name: format!("{} by Google", model_params.model),
                }
            }
            _ => ModelIdentity {
                base_model_info: "The base model information is not available.".to_string(),
                model_name: model_params.model.clone(),
            },
        }
    }

    /// Create initial system message using the new modular prompt system
    fn create_system_message(&self, task: &TaskMetadata) -> LLMMessage {
        // Get current model info for the identity section
        let model_info = self.get_model_identity();

        // Get tool schemas
        let tool_schemas = self.tool_executor.get_tool_schemas();

        // Check if working directory is a git repo
        let is_git_repo = std::path::Path::new(&task.working_dir)
            .join(".git")
            .exists();

        // Get current git branch if in a git repo
        let (git_branch, main_branch) = if is_git_repo {
            let branch = std::process::Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .current_dir(&task.working_dir)
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "main".to_string());
            (branch, "main".to_string())
        } else {
            ("main".to_string(), "main".to_string())
        };

        // Get platform info
        let platform = std::env::consts::OS.to_string();
        let os_version = std::process::Command::new("uname")
            .arg("-r")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        // Build system prompt using the new modular system
        let system_prompt = SystemPromptBuilder::new()
            .with_agent_name("Sage Agent")
            .with_agent_version(env!("CARGO_PKG_VERSION"))
            .with_model_name(&model_info.model_name)
            .with_task(&task.description)
            .with_working_dir(&task.working_dir)
            .with_git_info(is_git_repo, &git_branch, &main_branch)
            .with_platform(&platform, &os_version)
            .with_tools(tool_schemas)
            .with_git_instructions(is_git_repo)
            .with_security_policy(true)
            .build();

        LLMMessage::system(system_prompt)
    }

    /// Get tool schemas from the executor
    pub fn get_tool_schemas(&self) -> Vec<ToolSchema> {
        self.tool_executor.get_tool_schemas()
    }

    /// Execute a single step
    #[instrument(skip(self, messages, tools), fields(step_number = %step_number))]
    async fn execute_step(
        &mut self,
        step_number: u32,
        messages: &[LLMMessage],
        tools: &[ToolSchema],
    ) -> SageResult<AgentStep> {
        // Print step separator
        DisplayManager::print_separator(&format!("Step {} - AI Thinking", step_number), "blue");

        let mut step = AgentStep::new(step_number, AgentState::Thinking);

        // Get LLM response with timing and animation
        let start_time = std::time::Instant::now();

        // Start thinking animation
        self.animation_manager
            .start_animation(AnimationState::Thinking, "🤖 Thinking", "blue")
            .await;

        // Get cancellation token for interrupt handling
        let cancellation_token = global_interrupt_manager().lock().cancellation_token();

        // Execute LLM call with interrupt support
        let llm_response = select! {
            response = self.llm_client.chat(messages, Some(tools)) => {
                response?
            }
            _ = cancellation_token.cancelled() => {
                // Stop animation on interruption
                self.animation_manager.stop_animation().await;
                return Err(SageError::agent("Task interrupted during LLM call"));
            }
        };

        let api_duration = start_time.elapsed();

        // Stop animation and show timing
        self.animation_manager.stop_animation().await;
        DisplayManager::print_timing("🤖 AI Response", api_duration);

        step = step.with_llm_response(llm_response.clone());

        // Record LLM interaction in trajectory
        if let Some(recorder) = &self.trajectory_recorder {
            let input_messages: Vec<serde_json::Value> = messages
                .iter()
                .map(|msg| serde_json::to_value(msg).unwrap_or_default())
                .collect();

            let response_record = crate::trajectory::recorder::LLMResponseRecord {
                content: llm_response.content.clone(),
                model: llm_response.model.clone(),
                finish_reason: llm_response.finish_reason.clone(),
                usage: llm_response.usage.as_ref().map(|u| {
                    crate::trajectory::recorder::TokenUsageRecord {
                        input_tokens: u.prompt_tokens,
                        output_tokens: u.completion_tokens,
                        cache_creation_input_tokens: None,
                        cache_read_input_tokens: None,
                        reasoning_tokens: None,
                    }
                }),
                tool_calls: if llm_response.tool_calls.is_empty() {
                    None
                } else {
                    Some(
                        llm_response
                            .tool_calls
                            .iter()
                            .map(|tc| serde_json::to_value(tc).unwrap_or_default())
                            .collect(),
                    )
                },
            };

            let tools_available: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
            let provider = self.config.get_default_provider().to_string();
            let model = self.config.default_model_parameters()?.model.clone();

            // Clone the recorder to avoid holding the reference across await
            let recorder_clone = recorder.clone();
            recorder_clone
                .lock()
                .await
                .record_llm_interaction(
                    provider,
                    model,
                    input_messages,
                    response_record,
                    Some(tools_available),
                )
                .await?;
        }

        // Show AI response with markdown rendering
        if !llm_response.content.is_empty() {
            if Self::is_markdown_content(&llm_response.content) {
                println!("\n🤖 AI Response:");
                DisplayManager::print_markdown(&llm_response.content);
            } else {
                println!("\n🤖 {}", llm_response.content.trim());
            }
        }

        // Check if there are tool calls
        if !llm_response.tool_calls.is_empty() {
            tracing::info!(
                tool_count = llm_response.tool_calls.len(),
                "executing tools"
            );
            step.state = AgentState::ToolExecution;

            // Print tool execution separator
            DisplayManager::print_separator("Tool Execution", "cyan");

            // Show and execute tools
            for tool_call in &llm_response.tool_calls {
                // Show what the tool is doing
                let action = match tool_call.name.as_str() {
                    "bash" => {
                        if let Some(command) = tool_call.arguments.get("command") {
                            let cmd_str = command.as_str().unwrap_or("");
                            if cmd_str.chars().count() > 60 {
                                let truncated: String = cmd_str.chars().take(57).collect();
                                format!("🖥️  {}...", truncated)
                            } else {
                                format!("🖥️  {}", cmd_str)
                            }
                        } else {
                            "🖥️  Running command".to_string()
                        }
                    }
                    "str_replace_based_edit_tool" => {
                        if let Some(action) = tool_call.arguments.get("action") {
                            match action.as_str().unwrap_or("") {
                                "view" => {
                                    if let Some(path) = tool_call.arguments.get("path") {
                                        format!("📖 Reading: {}", path.as_str().unwrap_or(""))
                                    } else {
                                        "📖 Reading file".to_string()
                                    }
                                }
                                "create" => {
                                    if let Some(path) = tool_call.arguments.get("path") {
                                        let content_preview = if let Some(content) =
                                            tool_call.arguments.get("file_text")
                                        {
                                            let content_str = content.as_str().unwrap_or("");
                                            if content_str.len() > 50 {
                                                format!(" ({}...)", &content_str[..47])
                                            } else if !content_str.is_empty() {
                                                format!(" ({})", content_str)
                                            } else {
                                                "".to_string()
                                            }
                                        } else {
                                            "".to_string()
                                        };
                                        format!(
                                            "📝 Creating: {}{}",
                                            path.as_str().unwrap_or(""),
                                            content_preview
                                        )
                                    } else {
                                        "📝 Creating file".to_string()
                                    }
                                }
                                "str_replace" => {
                                    if let Some(path) = tool_call.arguments.get("path") {
                                        format!("✏️ Editing: {}", path.as_str().unwrap_or(""))
                                    } else {
                                        "✏️ Editing file".to_string()
                                    }
                                }
                                _ => {
                                    if let Some(path) = tool_call.arguments.get("path") {
                                        format!("📄 File op: {}", path.as_str().unwrap_or(""))
                                    } else {
                                        "📄 File operation".to_string()
                                    }
                                }
                            }
                        } else {
                            "📄 File operation".to_string()
                        }
                    }
                    "task_done" => {
                        if let Some(summary) = tool_call.arguments.get("summary") {
                            let summary_str = summary.as_str().unwrap_or("");
                            if summary_str.chars().count() > 50 {
                                let truncated: String = summary_str.chars().take(47).collect();
                                format!("✅ Done: {}...", truncated)
                            } else {
                                format!("✅ Done: {}", summary_str)
                            }
                        } else {
                            "✅ Task completed".to_string()
                        }
                    }
                    "sequentialthinking" => {
                        if let Some(thought) = tool_call.arguments.get("thought") {
                            let thought_str = thought.as_str().unwrap_or("");
                            if thought_str.chars().count() > 50 {
                                let truncated: String = thought_str.chars().take(47).collect();
                                format!("🧠 Thinking: {}...", truncated)
                            } else {
                                format!("🧠 Thinking: {}", thought_str)
                            }
                        } else {
                            "🧠 Thinking step by step".to_string()
                        }
                    }
                    "json_edit_tool" => {
                        if let Some(path) = tool_call.arguments.get("path") {
                            format!("📝 JSON edit: {}", path.as_str().unwrap_or(""))
                        } else {
                            "📝 JSON operation".to_string()
                        }
                    }
                    _ => format!("🔧 Using {}", tool_call.name),
                };
                println!("{}", action.blue());
            }

            // Execute tools with timing and animation
            let tool_start_time = std::time::Instant::now();

            // Start tool execution animation
            self.animation_manager
                .start_animation(AnimationState::ExecutingTools, "⚡ Executing tools", "cyan")
                .await;

            // Execute tools with interrupt support
            let tool_results = select! {
                results = self.tool_executor.execute_tools(&llm_response.tool_calls) => {
                    results
                }
                _ = cancellation_token.cancelled() => {
                    // Stop animation on interruption
                    self.animation_manager.stop_animation().await;
                    return Err(SageError::agent("Task interrupted during tool execution"));
                }
            };

            let tool_duration = tool_start_time.elapsed();

            // Stop animation and show timing if significant
            self.animation_manager.stop_animation().await;
            if tool_duration.as_millis() > 1000 {
                DisplayManager::print_timing("⚡ Tools", tool_duration);
            }

            // Show tool results briefly
            for result in &tool_results {
                if !result.success {
                    println!(
                        "❌ Error: {}",
                        result.error.as_deref().unwrap_or("Unknown error")
                    );
                } else if let Some(output) = &result.output {
                    // Only show output for certain tools or if it's short
                    if result.tool_name == "task_done" || output.len() < 100 {
                        println!("✅ {}", output.trim());
                    }
                }
            }

            step = step.with_tool_results(tool_results);
        }

        // Check if task is completed
        if llm_response.indicates_completion() {
            tracing::info!("step indicates task completion");
            step.state = AgentState::Completed;
            DisplayManager::print_separator("Task Completed", "green");
        }

        step.complete();
        Ok(step)
    }

    /// Build conversation messages from execution history
    fn build_messages(
        &self,
        execution: &AgentExecution,
        system_message: &LLMMessage,
    ) -> Vec<LLMMessage> {
        let mut messages = vec![system_message.clone()];

        // ALWAYS add the initial task as the first user message
        // This ensures the conversation history is complete when continuing
        let initial_user_message = LLMMessage::user(&execution.task.description);
        messages.push(initial_user_message);

        for step in &execution.steps {
            // Add LLM response as assistant message
            if let Some(response) = &step.llm_response {
                let mut assistant_msg = LLMMessage::assistant(&response.content);
                if !response.tool_calls.is_empty() {
                    assistant_msg.tool_calls = Some(response.tool_calls.clone());
                }
                messages.push(assistant_msg);

                // Add tool results as tool messages with proper tool_call_id
                for result in &step.tool_results {
                    let content = if result.success {
                        result.output.clone().unwrap_or_default()
                    } else {
                        // Format error in Claude Code style
                        format!(
                            "<tool_use_error>{}</tool_use_error>",
                            result.error.as_deref().unwrap_or("Unknown error")
                        )
                    };
                    // Use LLMMessage::tool to properly link to the tool call
                    let tool_msg = LLMMessage::tool(
                        content,
                        result.call_id.clone(),
                        Some(result.tool_name.clone()),
                    );
                    messages.push(tool_msg);
                }
            }
        }

        messages
    }
}

#[async_trait]
impl Agent for BaseAgent {
    #[instrument(skip(self), fields(task_id = %task.id, task_description = %task.description, max_steps = %self.max_steps))]
    async fn execute_task(&mut self, task: TaskMetadata) -> SageResult<ExecutionOutcome> {
        tracing::info!("starting agent execution");
        let mut execution = AgentExecution::new(task.clone());

        // Reset the global interrupt manager for this new task
        reset_global_interrupt_manager();

        // Create a task scope for interrupt handling
        let task_scope = global_interrupt_manager().lock().create_task_scope();

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

        let system_message = self.create_system_message(&task);
        let tool_schemas = self.tool_executor.get_tool_schemas();
        let provider_name = self.config.get_default_provider().to_string();

        // Main execution loop - returns the final outcome
        let final_outcome = 'execution_loop: {
            for step_number in 1..=self.max_steps {
                // Check for interruption before each step
                if task_scope.is_cancelled() {
                    // Stop animation on interruption
                    self.animation_manager.stop_animation().await;

                    // Print interruption message
                    DisplayManager::print_separator("Task Interrupted", "yellow");
                    println!("{}", "🛑 Task interrupted by user (Ctrl+C)".yellow().bold());
                    println!("{}", "   Task execution stopped gracefully.".dimmed());

                    let interrupt_step = AgentStep::new(step_number, AgentState::Error)
                        .with_error("Task interrupted by user".to_string());

                    // Record interrupt step
                    if let Some(recorder) = &self.trajectory_recorder {
                        recorder
                            .lock()
                            .await
                            .record_step(interrupt_step.clone())
                            .await?;
                    }

                    execution.add_step(interrupt_step);
                    execution.complete(false, Some("Task interrupted by user".to_string()));
                    break 'execution_loop ExecutionOutcome::Interrupted { execution };
                }

                let messages = self.build_messages(&execution, &system_message);

                match self
                    .execute_step(step_number, &messages, &tool_schemas)
                    .await
                {
                    Ok(step) => {
                        let is_completed = step.state == AgentState::Completed;

                        // Check if model needs user input (output text without tool calls)
                        let needs_input = step
                            .llm_response
                            .as_ref()
                            .map(|r| r.needs_user_input())
                            .unwrap_or(false);
                        let last_response_content = step
                            .llm_response
                            .as_ref()
                            .map(|r| r.content.clone())
                            .unwrap_or_default();

                        // Record step in trajectory
                        if let Some(recorder) = &self.trajectory_recorder {
                            recorder.lock().await.record_step(step.clone()).await?;
                        }

                        execution.add_step(step);

                        if is_completed {
                            tracing::info!(
                                steps = execution.steps.len(),
                                total_tokens = execution.total_usage.total_tokens,
                                "task completed successfully"
                            );
                            execution
                                .complete(true, Some("Task completed successfully".to_string()));
                            break 'execution_loop ExecutionOutcome::Success(execution);
                        }

                        // If model needs user input, return NeedsUserInput outcome
                        // This prevents the loop from continuing without user response
                        if needs_input {
                            DisplayManager::print_separator("Waiting for Input", "yellow");
                            break 'execution_loop ExecutionOutcome::NeedsUserInput {
                                execution,
                                last_response: last_response_content,
                            };
                        }
                    }
                    Err(e) => {
                        // Stop animation on error
                        self.animation_manager.stop_animation().await;

                        tracing::error!(
                            step = step_number,
                            error = %e,
                            "execution step failed"
                        );

                        let error_step = AgentStep::new(step_number, AgentState::Error)
                            .with_error(e.to_string());

                        // Record error step
                        if let Some(recorder) = &self.trajectory_recorder {
                            recorder
                                .lock()
                                .await
                                .record_step(error_step.clone())
                                .await?;
                        }

                        execution.add_step(error_step);
                        execution.complete(false, Some(format!("Task failed: {}", e)));

                        // Create structured error with classification and suggestions
                        let exec_error =
                            ExecutionError::from_sage_error(&e, Some(provider_name.clone()));
                        break 'execution_loop ExecutionOutcome::Failed {
                            execution,
                            error: exec_error,
                        };
                    }
                }
            }

            // Max steps reached without completion
            execution.complete(
                false,
                Some("Task execution reached maximum steps".to_string()),
            );
            ExecutionOutcome::MaxStepsReached { execution }
        };

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
        // Add user message to the execution context
        let system_message = self.create_system_message(&execution.task);
        let tool_schemas = self.tool_executor.get_tool_schemas();

        // Build messages including the new user message
        let mut messages = self.build_messages(execution, &system_message);
        messages.push(LLMMessage::user(user_message));

        // Continue execution from where we left off
        let start_step = (execution.steps.len() + 1) as u32;
        let max_step = start_step + self.max_steps - 1;

        for step_number in start_step..=max_step {
            match self
                .execute_step(step_number, &messages, &tool_schemas)
                .await
            {
                Ok(step) => {
                    let is_completed = step.state == AgentState::Completed;

                    // Record step in trajectory
                    if let Some(recorder) = &self.trajectory_recorder {
                        recorder.lock().await.record_step(step.clone()).await?;
                    }

                    execution.add_step(step);

                    if is_completed {
                        execution.complete(
                            true,
                            Some("Conversation continued successfully".to_string()),
                        );
                        break;
                    }

                    // Update messages for next iteration
                    let updated_messages = self.build_messages(execution, &system_message);
                    messages.clear();
                    messages.extend(updated_messages);
                }
                Err(e) => {
                    // Stop animation on error
                    self.animation_manager.stop_animation().await;

                    let error_step =
                        AgentStep::new(step_number, AgentState::Error).with_error(e.to_string());

                    // Record error step
                    if let Some(recorder) = &self.trajectory_recorder {
                        recorder
                            .lock()
                            .await
                            .record_step(error_step.clone())
                            .await?;
                    }

                    execution.add_step(error_step);
                    execution.complete(
                        false,
                        Some(format!("Conversation continuation failed: {}", e)),
                    );
                    return Err(e);
                }
            }
        }

        // If we reached max steps without completion
        if !execution.is_completed() {
            execution.complete(
                false,
                Some("Conversation continuation reached maximum steps".to_string()),
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_markdown_content_headers() {
        assert!(BaseAgent::is_markdown_content("# Header"));
        assert!(BaseAgent::is_markdown_content("## Sub Header"));
        assert!(BaseAgent::is_markdown_content("### Sub Sub Header"));
    }

    #[test]
    fn test_is_markdown_content_lists() {
        assert!(BaseAgent::is_markdown_content("* Item 1\n* Item 2"));
        assert!(BaseAgent::is_markdown_content("- Item 1\n- Item 2"));
    }

    #[test]
    fn test_is_markdown_content_code_blocks() {
        assert!(BaseAgent::is_markdown_content("```rust\nfn main() {}\n```"));
        assert!(BaseAgent::is_markdown_content("Some `inline code` here"));
    }

    #[test]
    fn test_is_markdown_content_formatting() {
        assert!(BaseAgent::is_markdown_content("**bold text**"));
        assert!(BaseAgent::is_markdown_content("*italic text*"));
        assert!(BaseAgent::is_markdown_content("[link](https://example.com)"));
        assert!(BaseAgent::is_markdown_content("> blockquote"));
    }

    #[test]
    fn test_is_markdown_content_multiline() {
        let multiline = "Line 1\nLine 2\nLine 3\nLine 4";
        assert!(BaseAgent::is_markdown_content(multiline));
    }

    #[test]
    fn test_is_not_markdown_content() {
        assert!(!BaseAgent::is_markdown_content("Simple text"));
        assert!(!BaseAgent::is_markdown_content("Just a sentence."));
    }

    #[test]
    fn test_model_identity_display() {
        let identity = ModelIdentity {
            base_model_info: "Test model info".to_string(),
            model_name: "test-model".to_string(),
        };

        assert_eq!(identity.base_model_info, "Test model info");
        assert_eq!(identity.model_name, "test-model");
    }

    #[test]
    fn test_markdown_edge_cases() {
        // Test edge cases for markdown detection
        // Empty string is not markdown
        assert!(!BaseAgent::is_markdown_content(""));

        // Single line without markdown
        assert!(!BaseAgent::is_markdown_content("Hello"));

        // Contains asterisk but is markdown
        assert!(BaseAgent::is_markdown_content("* List item"));
        assert!(BaseAgent::is_markdown_content("**bold**"));

        // Multiple lines without markdown triggers multiline heuristic
        let multiline_plain = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        assert!(BaseAgent::is_markdown_content(multiline_plain));
    }
}
