//! Main execution loop logic

use crate::agent::{AgentExecution, AgentState, AgentStep, ExecutionError, ExecutionOutcome};
use crate::error::{SageError, SageResult};
use crate::session::{EnhancedTokenUsage, EnhancedToolCall};
use crate::ui::DisplayManager;

use super::UnifiedExecutor;

impl UnifiedExecutor {
    /// Run the main execution loop
    pub(super) async fn run_execution_loop(
        &mut self,
        mut execution: AgentExecution,
        mut messages: Vec<crate::llm::messages::LlmMessage>,
        tool_schemas: Vec<crate::tools::types::ToolSchema>,
        task_scope: crate::interrupt::TaskScope,
        provider_name: String,
        max_steps: Option<u32>,
    ) -> SageResult<ExecutionOutcome> {
        // Repetition detection: track recent outputs to detect loops
        const MAX_RECENT_OUTPUTS: usize = 3;
        const REPETITION_THRESHOLD: usize = 2; // Force completion after N similar outputs
        let mut recent_outputs: Vec<String> = Vec::with_capacity(MAX_RECENT_OUTPUTS);

        let outcome = 'execution_loop: {
            let mut step_number = 0u32;
            loop {
                step_number += 1;

                // Check max_steps limit (None = unlimited)
                if let Some(max) = max_steps.filter(|&max| step_number > max) {
                    tracing::warn!("Reached maximum steps: {}", max);
                    execution.complete(false, Some("Reached maximum steps".to_string()));
                    break 'execution_loop ExecutionOutcome::MaxStepsReached { execution };
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
                    Ok((mut step, new_messages)) => {
                        // Repetition detection: check if output is repeated
                        // Only check non-empty content - empty strings are normal when AI only calls tools
                        if let Some(ref response) = step.llm_response {
                            let content_trimmed = response.content.trim();

                            // Skip repetition detection for empty or very short content
                            // (AI often returns empty content when only calling tools)
                            if content_trimmed.len() >= 10 {
                                let output_key = if content_trimmed.len() > 200 {
                                    // Use first 200 chars as fingerprint for long outputs
                                    content_trimmed.chars().take(200).collect::<String>()
                                } else {
                                    content_trimmed.to_string()
                                };

                                // Count how many recent outputs are similar
                                let repetition_count =
                                    recent_outputs.iter().filter(|o| *o == &output_key).count();

                                if repetition_count >= REPETITION_THRESHOLD {
                                    tracing::warn!(
                                        repetition_count = repetition_count,
                                        "Detected repeated output, forcing completion to prevent infinite loop"
                                    );
                                    step.state = AgentState::Completed;
                                }

                                // Track this output
                                if recent_outputs.len() >= MAX_RECENT_OUTPUTS {
                                    recent_outputs.remove(0);
                                }
                                recent_outputs.push(output_key);
                            }
                        }

                        let is_completed = step.state == AgentState::Completed;

                        // Session recording is handled by llm_interaction.rs and tool_execution.rs
                        // which record each request/response/tool_call/tool_result individually

                        // Record assistant message in JSONL session
                        self.record_step_in_session(&step).await?;

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

                        // Record error in session
                        if let Some(recorder) = &self.session_recorder {
                            let _ = recorder
                                .lock()
                                .await
                                .record_error("execution_error", &e.to_string())
                                .await;
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

        Ok(outcome)
    }

    /// Record step in JSONL session
    async fn record_step_in_session(&mut self, step: &AgentStep) -> SageResult<()> {
        if self.current_session_id.is_none() {
            return Ok(());
        }

        let Some(llm_response) = step.llm_response.as_ref() else {
            return Ok(());
        };

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
                            .unwrap_or(serde_json::Value::Object(Default::default())),
                    })
                    .collect(),
            )
        } else {
            None
        };

        // Convert usage if available
        let usage = llm_response.usage.as_ref().map(|u| EnhancedTokenUsage {
            input_tokens: u.prompt_tokens as u64,
            output_tokens: u.completion_tokens as u64,
            cache_read_tokens: u.cache_read_input_tokens.unwrap_or(0) as u64,
            cache_write_tokens: u.cache_creation_input_tokens.unwrap_or(0) as u64,
        });

        // Record assistant message and get the message UUID
        if let Ok(Some(msg)) = self
            .record_assistant_message(&llm_response.content, tool_calls, usage)
            .await
        {
            // Record file snapshot if files were tracked
            let _ = self.record_file_snapshot(&msg.uuid).await;
        }

        Ok(())
    }
}
