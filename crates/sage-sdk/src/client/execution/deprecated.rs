//! Deprecated execution methods

use crate::client::SageAgentSdk;
use sage_core::{
    agent::{Agent, AgentExecution, base::BaseAgent},
    error::SageResult,
    tools::executor::ToolExecutorBuilder,
    trajectory::SessionRecorder,
};
use sage_tools::get_default_tools;
use std::sync::Arc;
use tokio::sync::Mutex;

impl SageAgentSdk {
    /// Continue an existing execution with a new user message
    ///
    /// **Deprecated**: This method uses the old exit-resume pattern.
    /// Consider using `execute_unified` with an InputChannel instead.
    #[deprecated(
        since = "0.2.0",
        note = "Use execute_unified with InputChannel for better user interaction handling"
    )]
    pub async fn continue_execution(
        &self,
        execution: &mut AgentExecution,
        user_message: &str,
    ) -> SageResult<()> {
        // Create agent
        let mut agent = BaseAgent::new(self.config.clone())?;

        // Set up tool executor with default tools
        let tool_executor = ToolExecutorBuilder::new()
            .with_tools(get_default_tools())
            .with_max_execution_time(std::time::Duration::from_secs(
                self.config.tools.max_execution_time,
            ))
            .with_parallel_execution(self.config.tools.allow_parallel_execution)
            .build();

        agent.set_tool_executor(tool_executor);

        // Set up session recording if enabled
        let working_dir = execution.task.working_dir.clone();
        if self.config.trajectory.is_enabled() {
            if let Ok(recorder) = SessionRecorder::new(&working_dir) {
                agent.set_session_recorder(Arc::new(Mutex::new(recorder)));
            }
        }

        // Continue the execution
        agent.continue_execution(execution, user_message).await
    }
}
