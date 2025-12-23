//! Deprecated execution methods

use crate::client::SageAgentSDK;
use sage_core::{
    agent::{Agent, AgentExecution, base::BaseAgent},
    error::SageResult,
    tools::executor::ToolExecutorBuilder,
    trajectory::recorder::TrajectoryRecorder,
};
use sage_tools::get_default_tools;
use std::sync::Arc;
use tokio::sync::Mutex;

impl SageAgentSDK {
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

        // Set up trajectory recording if enabled
        if let Some(trajectory_path) = &self.trajectory_path {
            let recorder = Arc::new(Mutex::new(TrajectoryRecorder::new(
                trajectory_path.clone(),
            )?));
            agent.set_trajectory_recorder(recorder);
        }

        // Continue the execution
        agent.continue_execution(execution, user_message).await
    }
}
