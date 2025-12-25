//! Conversation execution and handling

use super::execution::{execute_conversation_continuation, execute_conversation_task};
use super::session::ConversationSession;
use crate::console::CliConsole;
use sage_core::error::SageResult;
use sage_core::types::TaskMetadata;
use sage_sdk::SageAgentSdk;

/// Handle conversation mode - supports continuous dialogue
pub async fn handle_conversation(
    console: &CliConsole,
    sdk: &SageAgentSdk,
    conversation: &mut ConversationSession,
    user_input: &str,
) -> SageResult<()> {
    conversation.add_user_message(user_input);

    if conversation.is_new_conversation() {
        console.print_header("New Conversation");
        console.info(&format!("Message: {user_input}"));

        let working_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .to_string_lossy()
            .to_string();

        let task = TaskMetadata::new(user_input, &working_dir);
        conversation.task = Some(task.clone());

        execute_conversation_task(console, sdk, conversation, &task).await
    } else {
        console.print_header("Continuing Conversation");
        console.info(&format!("Message: {user_input}"));

        if let Some(task) = conversation.task.clone() {
            execute_conversation_continuation(console, sdk, conversation, &task).await
        } else {
            let working_dir = std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .to_string_lossy()
                .to_string();

            let task = TaskMetadata::new(user_input, &working_dir);
            conversation.task = Some(task.clone());
            execute_conversation_task(console, sdk, conversation, &task).await
        }
    }
}
