//! Task execution functions

use super::outcome::handle_execution_outcome;
use super::session::ConversationSession;
use crate::console::CliConsole;
use crate::signal_handler::{AppState, set_global_app_state};
use sage_core::error::{SageError, SageResult};
use sage_core::types::TaskMetadata;
use sage_sdk::{RunOptions, SageAgentSdk};

/// Execute a new conversation task
pub async fn execute_conversation_task(
    console: &CliConsole,
    sdk: &SageAgentSdk,
    conversation: &mut ConversationSession,
    task: &TaskMetadata,
) -> SageResult<()> {
    let start_time = std::time::Instant::now();

    console.info("🤔 Starting conversation...");

    set_global_app_state(AppState::ExecutingTask);

    let run_options = RunOptions::new().with_trajectory(true);

    match tokio::time::timeout(
        std::time::Duration::from_secs(300),
        sdk.run_with_options(&task.description, run_options),
    )
    .await
    {
        Ok(result) => match result {
            Ok(execution_result) => {
                let duration = start_time.elapsed();
                conversation.execution = Some(execution_result.execution().clone());
                conversation.mark_first_message_processed();

                if let Some(final_result) = &execution_result.execution().final_result {
                    conversation.add_assistant_message(final_result);
                }

                handle_execution_outcome(console, &execution_result.outcome, conversation)?;

                console.info(&format!("ℹ Execution time: {:.2}s", duration.as_secs_f64()));
                console.info(&format!(
                    "ℹ Steps: {}",
                    execution_result.execution().steps.len()
                ));
                console.info(&format!(
                    "ℹ Tokens: {}",
                    execution_result.execution().total_usage.total_tokens
                ));

                if let Some(trajectory_path) = &execution_result.trajectory_path {
                    console.info(&format!(
                        "ℹ Trajectory saved: {}",
                        trajectory_path.display()
                    ));
                }

                Ok(())
            }
            Err(e) => {
                let duration = start_time.elapsed();
                console.error("✗ System error!");
                console.error(&format!("ℹ Execution time: {:.2}s", duration.as_secs_f64()));
                console.error(&format!("ℹ Error: {e}"));
                Err(e)
            }
        },
        Err(_) => {
            let duration = start_time.elapsed();
            console.error(&format!(
                "Conversation timed out after {:.2}s",
                duration.as_secs_f64()
            ));
            Err(SageError::timeout(300))
        }
    }
}

/// Execute conversation continuation (for follow-up messages)
///
/// Note: With the unified executor, each message is treated as a new task.
/// The conversation context is maintained through the session, not through
/// the executor's internal state.
pub async fn execute_conversation_continuation(
    console: &CliConsole,
    sdk: &SageAgentSdk,
    conversation: &mut ConversationSession,
    task: &TaskMetadata,
) -> SageResult<()> {
    // With unified executor, continuation is just a new task execution
    // The user's follow-up message becomes the new task description
    let user_message = conversation
        .messages
        .last()
        .map(|msg| msg.content.clone())
        .unwrap_or_else(|| task.description.clone());

    let new_task = TaskMetadata::new(&user_message, &task.working_dir);
    execute_conversation_task(console, sdk, conversation, &new_task).await
}
