//! Execution analysis methods

use super::core::ExecutionResult;

impl ExecutionResult {
    /// Get execution statistics.
    ///
    /// Returns metrics including step count, tool usage, and timing information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use sage_sdk::SageAgentSdk;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let sdk = SageAgentSdk::new()?;
    /// let result = sdk.run("task").await?;
    /// let stats = result.statistics();
    /// println!("Total steps: {}", stats.total_steps);
    /// # Ok(())
    /// # }
    /// ```
    pub fn statistics(&self) -> sage_core::agent::execution::ExecutionStatistics {
        self.outcome.execution().statistics()
    }

    /// Get a human-readable summary of the execution.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use sage_sdk::SageAgentSdk;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let sdk = SageAgentSdk::new()?;
    /// let result = sdk.run("task").await?;
    /// println!("{}", result.summary());
    /// # Ok(())
    /// # }
    /// ```
    pub fn summary(&self) -> String {
        self.outcome.execution().summary()
    }

    /// Get all tool calls made during execution.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use sage_sdk::SageAgentSdk;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let sdk = SageAgentSdk::new()?;
    /// let result = sdk.run("task").await?;
    /// for call in result.tool_calls() {
    ///     println!("Tool: {}", call.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn tool_calls(&self) -> Vec<&sage_core::tools::types::ToolCall> {
        self.outcome.execution().all_tool_calls().collect()
    }

    /// Get all tool results from execution.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use sage_sdk::SageAgentSdk;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let sdk = SageAgentSdk::new()?;
    /// let result = sdk.run("task").await?;
    /// for tool_result in result.tool_results() {
    ///     println!("Tool result: {:?}", tool_result);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn tool_results(&self) -> Vec<&sage_core::tools::types::ToolResult> {
        self.outcome.execution().all_tool_results().collect()
    }

    /// Get steps that had errors.
    ///
    /// Returns a list of agent steps where tool execution or other errors occurred.
    pub fn error_steps(&self) -> Vec<&sage_core::agent::AgentStep> {
        self.outcome.execution().error_steps().collect()
    }
}
