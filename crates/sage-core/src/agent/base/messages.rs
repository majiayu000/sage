//! Message building for agent conversations

use crate::agent::AgentExecution;
use crate::llm::messages::LlmMessage;

/// Build conversation messages from execution history
pub(super) fn build_messages(
    execution: &AgentExecution,
    system_message: &LlmMessage,
) -> Vec<LlmMessage> {
    let mut messages = vec![system_message.clone()];

    // ALWAYS add the initial task as the first user message
    // This ensures the conversation history is complete when continuing
    let initial_user_message = LlmMessage::user(&execution.task.description);
    messages.push(initial_user_message);

    for step in &execution.steps {
        // Add LLM response as assistant message
        if let Some(response) = &step.llm_response {
            let mut assistant_msg = LlmMessage::assistant(&response.content);
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
                // Use LlmMessage::tool to properly link to the tool call
                let tool_msg = LlmMessage::tool(
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
