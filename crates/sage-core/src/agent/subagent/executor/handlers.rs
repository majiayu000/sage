//! Step execution and tool handling for sub-agent executor

use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use super::types::StepResult;
use crate::error::{SageError, SageResult};
use crate::llm::client::LlmClient;
use crate::llm::messages::{LlmMessage, MessageRole};
use crate::tools::base::Tool;
use crate::tools::types::{ToolCall, ToolResult};

/// Helper for executing individual steps
pub(super) struct StepExecutor {
    llm_client: Arc<LlmClient>,
}

impl StepExecutor {
    /// Create a new step executor
    pub fn new(llm_client: Arc<LlmClient>) -> Self {
        Self { llm_client }
    }

    /// Execute single step
    pub async fn execute_step(
        &self,
        messages: &mut Vec<LlmMessage>,
        tools: &[Arc<dyn Tool>],
        cancel: &CancellationToken,
    ) -> SageResult<StepResult> {
        // Check cancellation
        if cancel.is_cancelled() {
            return Err(SageError::Cancelled);
        }

        // Get tool schemas
        let tool_schemas: Vec<_> = tools.iter().map(|t| t.schema()).collect();

        // Call LLM
        let response = self.llm_client.chat(messages, Some(&tool_schemas)).await?;

        // Check if there are tool calls
        if !response.tool_calls.is_empty() {
            // Add assistant message with tool calls
            let assistant_msg = LlmMessage {
                role: MessageRole::Assistant,
                content: response.content.clone(),
                tool_calls: Some(response.tool_calls.clone()),
                tool_call_id: None,
                cache_control: None,
                name: None,
                metadata: Default::default(),
            };
            messages.push(assistant_msg);

            // Execute tool calls
            for call in &response.tool_calls {
                let result = self.execute_tool_call(call, tools, cancel).await?;

                // Add tool result message
                let tool_msg = LlmMessage::tool(
                    result
                        .output
                        .unwrap_or_else(|| result.error.unwrap_or_default()),
                    call.id.clone(),
                    Some(call.name.clone()),
                );
                messages.push(tool_msg);
            }

            Ok(StepResult::Continue)
        } else {
            // No tool calls - this is the final response
            let assistant_msg = LlmMessage::assistant(&response.content);
            messages.push(assistant_msg);

            // Check if this indicates completion
            if response.finish_reason.as_deref() == Some("stop") {
                Ok(StepResult::Completed(response.content))
            } else {
                Ok(StepResult::NeedsMoreSteps)
            }
        }
    }

    /// Execute a tool call
    async fn execute_tool_call(
        &self,
        call: &ToolCall,
        tools: &[Arc<dyn Tool>],
        cancel: &CancellationToken,
    ) -> SageResult<ToolResult> {
        // Check cancellation
        if cancel.is_cancelled() {
            return Err(SageError::Cancelled);
        }

        // Find the tool
        let tool = tools
            .iter()
            .find(|t| t.name() == call.name)
            .ok_or_else(|| SageError::tool(&call.name, "Tool not found"))?;

        // Execute the tool
        let result = tool.execute_with_timing(call).await;

        Ok(result)
    }
}
