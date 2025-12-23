//! Execution outcome handling

use crate::console::CliConsole;
use super::session::ConversationSession;
use sage_core::error::SageResult;
use sage_sdk::{ExecutionErrorKind, ExecutionOutcome};

/// Handle execution outcome and display appropriate messages
pub fn handle_execution_outcome(
    console: &CliConsole,
    outcome: &ExecutionOutcome,
    conversation: &mut ConversationSession,
) -> SageResult<()> {
    match outcome {
        ExecutionOutcome::Success(_) => {
            console.success("✓ Task completed successfully!");
        }
        ExecutionOutcome::Failed { error, .. } => {
            console.error("✗ Task failed!");
            console.error(&format!("  Error: {}", error.message));

            match &error.kind {
                ExecutionErrorKind::Authentication => {
                    console.error("  Type: Authentication Error");
                }
                ExecutionErrorKind::RateLimit => {
                    console.warn("  Type: Rate Limit");
                }
                ExecutionErrorKind::ServiceUnavailable => {
                    console.warn("  Type: Service Unavailable");
                }
                ExecutionErrorKind::ToolExecution { tool_name } => {
                    console.error(&format!("  Type: Tool Error ({})", tool_name));
                }
                ExecutionErrorKind::Configuration => {
                    console.error("  Type: Configuration Error");
                }
                ExecutionErrorKind::Network => {
                    console.error("  Type: Network Error");
                }
                ExecutionErrorKind::Timeout => {
                    console.warn("  Type: Timeout");
                }
                _ => {}
            }

            if let Some(provider) = &error.provider {
                console.info(&format!("  Provider: {}", provider));
            }

            if let Some(suggestion) = &error.suggestion {
                console.info(&format!("  💡 {}", suggestion));
            }
        }
        ExecutionOutcome::Interrupted { .. } => {
            console.warn("🛑 Task interrupted by user");
            console.info("ℹ You can continue with a new task or type 'exit' to quit");
        }
        ExecutionOutcome::MaxStepsReached { .. } => {
            console.warn("⚠ Task reached maximum steps without completion");
            console.info("ℹ Consider breaking down the task or increasing max_steps");
        }
        ExecutionOutcome::UserCancelled {
            pending_question, ..
        } => {
            console.warn("⊘ Task cancelled by user");
            if let Some(question) = pending_question {
                console.info(&format!("ℹ Pending question: {}", question));
            }
        }
        ExecutionOutcome::NeedsUserInput { last_response, .. } => {
            console.info("💬 AI is waiting for your response");

            if !last_response.is_empty() {
                conversation.add_assistant_message(last_response);
            }

            console.info("ℹ Type your response to continue the conversation");
        }
    }
    Ok(())
}
