//! Continue execution implementation

use crate::agent::{AgentExecution, AgentState, AgentStep};
use crate::error::SageResult;
use crate::llm::client::LlmClient;
use crate::llm::messages::LlmMessage;
use crate::tools::executor::ToolExecutor;
use crate::tools::types::ToolSchema;
use crate::trajectory::recorder::TrajectoryRecorder;
use crate::ui::AnimationManager;
use crate::config::model::Config;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::messages::build_messages;
use super::step_execution::execute_step;

/// Continue execution with new user message
pub(super) async fn continue_execution_impl(
    execution: &mut AgentExecution,
    user_message: &str,
    system_message: &LlmMessage,
    tool_schemas: &[ToolSchema],
    max_steps: u32,
    llm_client: &mut LlmClient,
    tool_executor: &ToolExecutor,
    animation_manager: &mut AnimationManager,
    trajectory_recorder: &Option<Arc<Mutex<TrajectoryRecorder>>>,
    config: &Config,
) -> SageResult<()> {
    // Build messages including the new user message
    let mut messages = build_messages(execution, system_message);
    messages.push(LlmMessage::user(user_message));

    // Continue execution from where we left off
    let start_step = (execution.steps.len() + 1) as u32;
    let max_step = start_step + max_steps - 1;

    for step_number in start_step..=max_step {
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

                // Record step in trajectory
                if let Some(recorder) = trajectory_recorder {
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
                let updated_messages = build_messages(execution, system_message);
                messages.clear();
                messages.extend(updated_messages);
            }
            Err(e) => {
                // Stop animation on error
                animation_manager.stop_animation().await;

                let error_step =
                    AgentStep::new(step_number, AgentState::Error).with_error(e.to_string());

                // Record error step
                if let Some(recorder) = trajectory_recorder {
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
