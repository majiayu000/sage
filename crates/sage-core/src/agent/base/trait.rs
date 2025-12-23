//! Agent trait definition

use crate::agent::{AgentExecution, ExecutionOutcome};
use crate::config::model::Config;
use crate::error::SageResult;
use crate::types::{Id, TaskMetadata};
use async_trait::async_trait;

/// Base agent trait
#[async_trait]
pub trait Agent: Send + Sync {
    /// Execute a task and return an explicit outcome
    ///
    /// Returns `ExecutionOutcome` which clearly indicates success, failure,
    /// interruption, or max steps reached, while preserving the full execution trace.
    async fn execute_task(&mut self, task: TaskMetadata) -> SageResult<ExecutionOutcome>;

    /// Continue an existing execution with new user message
    async fn continue_execution(
        &mut self,
        execution: &mut AgentExecution,
        user_message: &str,
    ) -> SageResult<()>;

    /// Get the agent's configuration
    fn config(&self) -> &Config;

    /// Get the agent's ID
    fn id(&self) -> Id;
}
