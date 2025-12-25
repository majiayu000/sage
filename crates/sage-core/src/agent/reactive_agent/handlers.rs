//! Request handling and execution logic

use super::core::ClaudeStyleAgent;
use super::system_prompt::create_system_message;
use super::trait_def::ReactiveAgent;
use super::types::ReactiveResponse;
use crate::config::model::Config;
use crate::error::SageResult;
use crate::llm::messages::LlmMessage;
use crate::types::TaskMetadata;
use async_trait::async_trait;
use std::time::Instant;
use uuid::Uuid;

impl ClaudeStyleAgent {
    /// Execute a single request-response cycle
    pub(super) async fn execute_single_turn(
        &mut self,
        request: &str,
        context: Option<&TaskMetadata>,
    ) -> SageResult<ReactiveResponse> {
        // Check if we can continue (step and budget limits)
        self.can_continue()?;

        let start_time = Instant::now();
        let response_id = Uuid::new_v4();

        // Increment step counter
        self.current_step += 1;

        // Build conversation messages
        let mut messages = vec![create_system_message(&self.batch_executor, context)];
        messages.extend(self.conversation_history.clone());
        messages.push(LlmMessage::user(request));

        // Get tool schemas
        let tool_schemas = self.batch_executor.get_tool_schemas();

        // Call LLM with tools
        let llm_response = self.llm_client.chat(&messages, Some(&tool_schemas)).await?;

        // Track token usage from LLM response
        if let Some(usage) = &llm_response.usage {
            self.token_usage
                .add(usage.prompt_tokens as u64, usage.completion_tokens as u64);
        }

        // Update conversation history
        let mut assistant_msg = LlmMessage::assistant(&llm_response.content);
        if !llm_response.tool_calls.is_empty() {
            assistant_msg.tool_calls = Some(llm_response.tool_calls.clone());
        }
        self.conversation_history.push(LlmMessage::user(request));
        self.conversation_history.push(assistant_msg);

        // Execute tools if present (batch execution)
        let tool_results = if !llm_response.tool_calls.is_empty() {
            self.batch_executor
                .execute_batch(&llm_response.tool_calls)
                .await
        } else {
            Vec::new()
        };

        // Add tool results to conversation history and track file operations
        if !tool_results.is_empty() {
            for result in &tool_results {
                // Track file operations for completion verification
                self.file_tracker.track_tool_call(&result.tool_name, result);

                let content = if result.success {
                    result.output.as_deref().unwrap_or("")
                } else {
                    &format!(
                        "Error: {}",
                        result.error.as_deref().unwrap_or("Unknown error")
                    )
                };
                self.conversation_history.push(LlmMessage::user(content));
            }
        }

        // Determine if task is completed
        // Check if task_done was called
        let task_done_called = llm_response.indicates_completion()
            || tool_results.iter().any(|r| r.tool_name == "task_done");

        // If task_done is called but no file operations were performed,
        // check if this looks like a documentation-only completion
        let completed = if task_done_called {
            // Allow completion if there were file operations OR
            // if the task explicitly doesn't require code (research/analysis tasks)
            // For now, we log a warning but allow completion
            if !self.file_tracker.has_file_operations() {
                tracing::warn!(
                    "Task marked as complete but no file operations were performed. \
                     Created files: {:?}, Modified files: {:?}",
                    self.file_tracker.created_files,
                    self.file_tracker.modified_files
                );
            }
            true
        } else {
            false
        };

        // Generate continuation prompt if needed
        let continuation_prompt = if !completed && !tool_results.is_empty() {
            Some("Continue with the next step based on the tool results.".to_string())
        } else {
            None
        };

        Ok(ReactiveResponse {
            id: response_id,
            request: request.to_string(),
            content: llm_response.content,
            tool_calls: llm_response.tool_calls,
            tool_results,
            duration: start_time.elapsed(),
            completed,
            continuation_prompt,
        })
    }
}

#[async_trait]
impl ReactiveAgent for ClaudeStyleAgent {
    async fn process_request(
        &mut self,
        request: &str,
        context: Option<TaskMetadata>,
    ) -> SageResult<ReactiveResponse> {
        // Clear history for new request if context indicates new task
        self.clear_history_if_new_task(context.as_ref());

        self.execute_single_turn(request, context.as_ref()).await
    }

    async fn continue_conversation(
        &mut self,
        _previous: &ReactiveResponse,
        additional_input: &str,
    ) -> SageResult<ReactiveResponse> {
        // Trim history to prevent context overflow
        self.trim_conversation_history();

        self.execute_single_turn(additional_input, None).await
    }

    fn config(&self) -> &Config {
        &self.config
    }
}
