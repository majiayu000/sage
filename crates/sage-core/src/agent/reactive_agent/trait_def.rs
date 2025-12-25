//! Reactive agent trait definition

use super::types::ReactiveResponse;
use crate::config::model::Config;
use crate::error::SageResult;
use crate::types::TaskMetadata;
use async_trait::async_trait;

/// Reactive agent trait - simplified Claude Code style interface
#[async_trait]
pub trait ReactiveAgent: Send + Sync {
    /// Process a user request and return a response
    async fn process_request(
        &mut self,
        request: &str,
        context: Option<TaskMetadata>,
    ) -> SageResult<ReactiveResponse>;

    /// Continue a conversation with additional context
    async fn continue_conversation(
        &mut self,
        previous: &ReactiveResponse,
        additional_input: &str,
    ) -> SageResult<ReactiveResponse>;

    /// Get agent configuration
    fn config(&self) -> &Config;
}
