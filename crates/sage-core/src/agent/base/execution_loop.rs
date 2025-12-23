//! Main task execution loop

use crate::agent::{AgentExecution, AgentState, AgentStep, ExecutionError, ExecutionOutcome};
use crate::interrupt::global_interrupt_manager;
use crate::llm::client::LlmClient;
use crate::llm::messages::LlmMessage;
use crate::tools::executor::ToolExecutor;
use crate::tools::types::ToolSchema;
use crate::trajectory::recorder::TrajectoryRecorder;
use crate::ui::{AnimationManager, DisplayManager};
use crate::config::model::Config;
use colored::*;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::messages::build_messages;
use super::step_execution::execute_step;

/// Execute the main task loop
pub(super) async fn execute_task_loop(
    execution: &mut AgentExecution,
    system_message: &LlmMessage,
    tool_schemas: &[ToolSchema],
    max_steps: u32,
    llm_client: &mut LlmClient,
    tool_executor: &ToolExecutor,
    animation_manager: &mut AnimationManager,
    trajectory_recorder: &Option<Arc<Mutex<TrajectoryRecorder>>>,
    config: &Config,
    provider_name: &str,
) -> ExecutionOutcome {
    let task_scope = global_interrupt_manager().lock().create_task_scope();

    'execution_loop: {
        for step_number in 1..=max_steps {
            // Check for interruption before each step
            if task_scope.is_cancelled() {
                // Stop animation on interruption
                animation_manager.stop_animation().await;

                // Print interruption message
                DisplayManager::print_separator("Task Interrupted", "yellow");
                println!("{}", "🛑 Task interrupted by user (Ctrl+C)".yellow().bold());
                println!("{}", "   Task execution stopped gracefully.".dimmed());

                let interrupt_step = AgentStep::new(step_number, AgentState::Error)
                    .with_error("Task interrupted by user".to_string());

                // Record interrupt step
                if let Some(recorder) = trajectory_recorder {
                    let _ = recorder
                        .lock()
                        .await
                        .record_step(interrupt_step.clone())
                        .await;
                }

                execution.add_step(interrupt_step);
                execution.complete(false, Some("Task interrupted by user".to_string()));
                break 'execution_loop ExecutionOutcome::Interrupted {
                    execution: execution.clone(),
                };
            }

            let messages = build_messages(execution, system_message);

            match execute_step(
                step_number,
                &messages,
                tool_schemas,
                llm_client,
                tool_executor,
                animation_manager,
                trajectory_recorder,
                config,
            )
            .await
            {
                Ok(step) => {
                    let is_completed = step.state == AgentState::Completed;

                    // Check if model needs user input
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
                    if let Some(recorder) = trajectory_recorder {
                        let _ = recorder.lock().await.record_step(step.clone()).await;
                    }

                    execution.add_step(step);

                    if is_completed {
                        tracing::info!(
                            steps = execution.steps.len(),
                            total_tokens = execution.total_usage.total_tokens,
                            "task completed successfully"
                        );
                        execution.complete(true, Some("Task completed successfully".to_string()));
                        break 'execution_loop ExecutionOutcome::Success(execution.clone());
                    }

                    // If model needs user input, return NeedsUserInput outcome
                    if needs_input {
                        DisplayManager::print_separator("Waiting for Input", "yellow");
                        break 'execution_loop ExecutionOutcome::NeedsUserInput {
                            execution: execution.clone(),
                            last_response: last_response_content,
                        };
                    }
                }
                Err(e) => {
                    // Stop animation on error
                    animation_manager.stop_animation().await;

                    tracing::error!(
                        step = step_number,
                        error = %e,
                        "execution step failed"
                    );

                    let error_step =
                        AgentStep::new(step_number, AgentState::Error).with_error(e.to_string());

                    // Record error step
                    if let Some(recorder) = trajectory_recorder {
                        let _ = recorder.lock().await.record_step(error_step.clone()).await;
                    }

                    execution.add_step(error_step);
                    execution.complete(false, Some(format!("Task failed: {}", e)));

                    // Create structured error
                    let exec_error = ExecutionError::from_sage_error(&e, Some(provider_name.to_string()));
                    break 'execution_loop ExecutionOutcome::Failed {
                        execution: execution.clone(),
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
        ExecutionOutcome::MaxStepsReached {
            execution: execution.clone(),
        }
    }
}
