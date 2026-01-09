//! Trait implementations for existing types
//!
//! This module provides implementations of the core traits for existing types:
//! - [`LlmService`] for [`LlmClient`]
//! - [`ToolService`] for [`ToolExecutor`]
//! - [`UserInteractionService`] for [`InputChannel`]

use crate::agent::traits::{LlmService, ToolService, UserInteractionService};
use crate::error::SageResult;
use crate::input::{InputChannel, InputRequest, InputResponse};
use crate::llm::client::LlmClient;
use crate::llm::messages::{LlmMessage, LlmResponse};
use crate::llm::streaming::{LlmStream, StreamingLlmClient};
use crate::tools::base::Tool;
use crate::tools::executor::ToolExecutor;
use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::sync::Arc;

// ============================================================================
// LlmService Implementation for LlmClient
// ============================================================================

#[async_trait]
impl LlmService for LlmClient {
    async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmResponse> {
        // Delegate to existing LlmClient::chat method
        LlmClient::chat(self, messages, tools).await
    }

    async fn chat_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream> {
        // Delegate to StreamingLlmClient implementation
        StreamingLlmClient::chat_stream(self, messages, tools).await
    }

    fn provider(&self) -> &str {
        self.provider().name()
    }

    fn model(&self) -> &str {
        LlmClient::model(self)
    }
}

// ============================================================================
// ToolService Implementation for ToolExecutor
// ============================================================================

#[async_trait]
impl ToolService for ToolExecutor {
    async fn execute_tool(&self, call: &ToolCall) -> ToolResult {
        ToolExecutor::execute_tool(self, call).await
    }

    async fn execute_tools(&self, calls: &[ToolCall]) -> Vec<ToolResult> {
        ToolExecutor::execute_tools(self, calls).await
    }

    fn get_tool_schemas(&self) -> Vec<ToolSchema> {
        ToolExecutor::get_tool_schemas(self)
    }

    fn register_tool(&mut self, tool: Arc<dyn Tool>) {
        ToolExecutor::register_tool(self, tool)
    }

    fn has_tool(&self, name: &str) -> bool {
        ToolExecutor::has_tool(self, name)
    }
}

// ============================================================================
// UserInteractionService Implementation for InputChannel
// ============================================================================

#[async_trait]
impl UserInteractionService for InputChannel {
    async fn request_input(&mut self, request: InputRequest) -> SageResult<InputResponse> {
        InputChannel::request_input(self, request).await
    }

    fn is_interactive(&self) -> bool {
        !self.is_non_interactive()
    }
}

#[cfg(test)]
mod tests {
    use crate::agent::traits::{NoopProgressReporter, NoopSessionRecorder, SessionRecorderService};

    #[tokio::test]
    async fn test_noop_session_recorder() {
        let mut recorder = NoopSessionRecorder;

        // All operations should succeed without errors
        recorder
            .record_session_start("test task", "test provider", "test model")
            .await
            .unwrap();
        recorder.record_user_message("test message").await.unwrap();
        recorder
            .record_assistant_message("test response", None)
            .await
            .unwrap();
        recorder
            .record_session_end(true, Some("done".to_string()))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_noop_progress_reporter() {
        use crate::agent::traits::ProgressReporter;

        let reporter = NoopProgressReporter;

        // All operations should complete without errors
        reporter.report_step_start(1).await;
        reporter.report_tool_start("test_tool", "params").await;
        reporter.report_tool_end("test_tool", true).await;
        reporter.report_message("test message").await;
        reporter.report_thinking().await;
        reporter.stop().await;
    }
}
