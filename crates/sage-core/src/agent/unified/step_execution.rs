//! Single step execution logic

use crate::agent::{AgentState, AgentStep};
use crate::error::{SageError, SageResult};
use crate::interrupt::global_interrupt_manager;
use crate::llm::messages::LLMMessage;
use crate::tools::types::ToolSchema;
use crate::ui::animation::AnimationState;
use crate::ui::DisplayManager;
use tokio::select;
use tracing::instrument;

use super::UnifiedExecutor;

impl UnifiedExecutor {
    /// Execute a single step in the loop
    #[instrument(skip(self, messages, tool_schemas, task_scope), fields(step_number = %step_number))]
    pub(super) async fn execute_step(
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
            self.handle_tool_calls(&mut step, &mut new_messages, &llm_response.tool_calls, task_scope)
                .await?;
        }

        // Check for completion indicator in response
        if llm_response.finish_reason == Some("end_turn".to_string())
            && llm_response.tool_calls.is_empty()
        {
            tracing::info!("step indicates task completion");
            step.state = AgentState::Completed;
        }

        Ok((step, new_messages))
    }

    /// Handle tool call execution
    async fn handle_tool_calls(
        &mut self,
        step: &mut AgentStep,
        new_messages: &mut Vec<LLMMessage>,
        tool_calls: &[crate::tools::types::ToolCall],
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

        Ok(())
    }
}
