//! Single step execution logic

use crate::agent::{AgentState, AgentStep};
use crate::error::{SageError, SageResult};
use crate::interrupt::global_interrupt_manager;
use crate::llm::messages::LlmMessage;
use crate::tools::types::{ToolCall, ToolSchema};
use crate::trajectory::TokenUsage;
use crate::ui::DisplayManager;
use crate::ui::animation::AnimationState;
use crate::ui::prompt::{PermissionChoice, PermissionDialogConfig, show_permission_dialog};
use tokio::select;
use tracing::instrument;

use super::UnifiedExecutor;

impl UnifiedExecutor {
    /// Execute a single step in the loop
    #[instrument(skip(self, messages, tool_schemas, task_scope), fields(step_number = %step_number))]
    pub(super) async fn execute_step(
        &mut self,
        step_number: u32,
        messages: &[LlmMessage],
        tool_schemas: &[ToolSchema],
        task_scope: &crate::interrupt::TaskScope,
    ) -> SageResult<(AgentStep, Vec<LlmMessage>)> {
        let mut step = AgentStep::new(step_number, AgentState::Thinking);

        // Start thinking animation
        self.animation_manager
            .start_animation(AnimationState::Thinking, "Thinking", "blue")
            .await;

        // Record LLM request before sending
        if let Some(recorder) = &self.session_recorder {
            let input_messages: Vec<serde_json::Value> = messages
                .iter()
                .map(|msg| serde_json::to_value(msg).unwrap_or_default())
                .collect();
            let tools_available: Vec<String> =
                tool_schemas.iter().map(|t| t.name.clone()).collect();
            let _ = recorder
                .lock()
                .await
                .record_llm_request(input_messages, Some(tools_available))
                .await;
        }

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

        // Record LLM response
        if let Some(recorder) = &self.session_recorder {
            let model = self
                .config
                .default_model_parameters()
                .map(|p| p.model.clone())
                .unwrap_or_default();
            let usage = llm_response.usage.as_ref().map(|u| TokenUsage {
                input_tokens: u.prompt_tokens as u64,
                output_tokens: u.completion_tokens as u64,
                cache_read_tokens: u.cache_read_input_tokens.map(|v| v as u64),
                cache_write_tokens: u.cache_creation_input_tokens.map(|v| v as u64),
            });
            let tool_calls = if llm_response.tool_calls.is_empty() {
                None
            } else {
                Some(
                    llm_response
                        .tool_calls
                        .iter()
                        .map(|tc| serde_json::to_value(tc).unwrap_or_default())
                        .collect(),
                )
            };
            let _ = recorder
                .lock()
                .await
                .record_llm_response(&llm_response.content, &model, usage, tool_calls)
                .await;
        }

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
            let mut assistant_msg = LlmMessage::assistant(&llm_response.content);
            if !llm_response.tool_calls.is_empty() {
                assistant_msg.tool_calls = Some(llm_response.tool_calls.clone());
            }
            new_messages.push(assistant_msg);
        }

        // Handle tool calls
        if !llm_response.tool_calls.is_empty() {
            self.handle_tool_calls(
                &mut step,
                &mut new_messages,
                &llm_response.tool_calls,
                task_scope,
            )
            .await?;
        }

        // Check for completion indicator in response
        // Support multiple LLM providers with different finish_reason values:
        // - Anthropic: "end_turn"
        // - OpenAI/GLM/others: "stop"
        // - Google: "STOP"
        let is_natural_end = match llm_response.finish_reason.as_deref() {
            Some("end_turn") | Some("stop") | Some("STOP") => true,
            _ => false,
        };

        if is_natural_end && llm_response.tool_calls.is_empty() {
            tracing::info!(
                finish_reason = ?llm_response.finish_reason,
                "step indicates task completion (natural end)"
            );
            step.state = AgentState::Completed;
        }

        Ok((step, new_messages))
    }

    /// Handle tool call execution
    async fn handle_tool_calls(
        &mut self,
        step: &mut AgentStep,
        new_messages: &mut Vec<LlmMessage>,
        tool_calls: &[ToolCall],
        task_scope: &crate::interrupt::TaskScope,
    ) -> SageResult<()> {
        tracing::info!(tool_count = tool_calls.len(), "executing tools");

        // Start tool animation
        self.animation_manager
            .start_animation(AnimationState::ExecutingTools, "Executing tools", "green")
            .await;

        for tool_call in tool_calls {
            // Check for interrupt before each tool
            if task_scope.is_cancelled() {
                self.animation_manager.stop_animation().await;
                return Err(SageError::agent("Task interrupted during tool execution"));
            }

            // Track files before file-modifying tools execute (for undo capability)
            if crate::tools::names::is_file_modifying_tool(&tool_call.name) {
                if let Some(file_path) = tool_call
                    .arguments
                    .get("file_path")
                    .or_else(|| tool_call.arguments.get("path"))
                    .and_then(|v| v.as_str())
                {
                    let _ = self.file_tracker.track_file(file_path).await;
                }
            }

            // Record tool call before execution
            if let Some(recorder) = &self.session_recorder {
                let tool_input = serde_json::to_value(&tool_call.arguments).unwrap_or_default();
                let _ = recorder
                    .lock()
                    .await
                    .record_tool_call(&tool_call.name, tool_input)
                    .await;
            }

            let tool_start_time = std::time::Instant::now();

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
                // Normal tool execution - may require permission confirmation
                self.execute_tool_with_permission_check(tool_call).await
            };

            // Record tool result after execution
            if let Some(recorder) = &self.session_recorder {
                let execution_time_ms = tool_start_time.elapsed().as_millis() as u64;
                let _ = recorder
                    .lock()
                    .await
                    .record_tool_result(
                        &tool_call.name,
                        tool_result.success,
                        tool_result.output.clone(),
                        tool_result.error.clone(),
                        execution_time_ms,
                    )
                    .await;
            }

            step.tool_results.push(tool_result.clone());

            // Add tool result to messages using LlmMessage::tool
            let tool_name = Some(tool_call.name.clone());
            new_messages.push(LlmMessage::tool(
                tool_result.output.clone().unwrap_or_default(),
                tool_call.id.clone(),
                tool_name,
            ));
        }

        self.animation_manager.stop_animation().await;
        step.state = AgentState::ToolExecution;

        Ok(())
    }

    /// Execute a tool with permission check for dangerous operations
    ///
    /// If the tool returns ConfirmationRequired error, this will:
    /// 1. Stop the animation
    /// 2. Show a permission dialog to the user
    /// 3. If user confirms, re-execute with user_confirmed=true
    /// 4. If user denies, return a rejection message
    async fn execute_tool_with_permission_check(
        &mut self,
        tool_call: &ToolCall,
    ) -> crate::tools::types::ToolResult {
        // First attempt - may fail with ConfirmationRequired
        let result = self.tool_executor.execute_tool(tool_call).await;

        // Check if the result indicates confirmation is required
        // The error message will be in the output field for failed results
        if !result.success {
            if let Some(ref error_msg) = result.error {
                if error_msg.contains("DESTRUCTIVE COMMAND BLOCKED")
                    || error_msg.contains("Confirmation required")
                {
                    // Stop animation to show dialog
                    self.animation_manager.stop_animation().await;

                    // Extract command from tool call
                    let command = tool_call
                        .arguments
                        .get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown command");

                    // Show permission dialog
                    let config = PermissionDialogConfig::new(
                        &tool_call.name,
                        command,
                        "This is a destructive operation that may delete files or make irreversible changes.",
                    );

                    let choice = show_permission_dialog(&config);

                    // Restart animation
                    self.animation_manager
                        .start_animation(AnimationState::ExecutingTools, "Executing tools", "green")
                        .await;

                    match choice {
                        PermissionChoice::YesOnce | PermissionChoice::YesAlways => {
                            // User confirmed - re-execute with user_confirmed=true
                            let mut confirmed_call = tool_call.clone();
                            confirmed_call.arguments.insert(
                                "user_confirmed".to_string(),
                                serde_json::Value::Bool(true),
                            );

                            tracing::info!(
                                tool = %tool_call.name,
                                command = %command,
                                "user confirmed destructive operation"
                            );

                            return self.tool_executor.execute_tool(&confirmed_call).await;
                        }
                        PermissionChoice::NoOnce | PermissionChoice::NoAlways => {
                            tracing::info!(
                                tool = %tool_call.name,
                                command = %command,
                                "user rejected destructive operation"
                            );

                            return crate::tools::types::ToolResult::error(
                                &tool_call.id,
                                &tool_call.name,
                                format!(
                                    "Operation cancelled by user. The user rejected the command: {}",
                                    command
                                ),
                            );
                        }
                        PermissionChoice::Cancelled => {
                            tracing::info!(
                                tool = %tool_call.name,
                                "user cancelled permission dialog"
                            );

                            return crate::tools::types::ToolResult::error(
                                &tool_call.id,
                                &tool_call.name,
                                "Operation cancelled by user (Ctrl+C or empty input).",
                            );
                        }
                    }
                }
            }
        }

        result
    }
}
