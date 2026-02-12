//! Core execution result type

use sage_core::{
    agent::{AgentExecution, ExecutionError, ExecutionOutcome},
    config::model::Config,
};

/// Result of task execution.
///
/// Contains the execution outcome and the configuration used for execution.
/// Provides convenient methods for checking execution status and extracting details.
///
/// # Examples
///
/// ```no_run
/// use sage_sdk::SageAgentSdk;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let sdk = SageAgentSdk::new()?;
/// let result = sdk.run("Complete the task").await?;
///
/// if result.is_success() {
///     println!("Final result: {:?}", result.final_result());
///     println!("Statistics: {:?}", result.statistics());
/// } else if result.is_failed() {
///     println!("Error: {:?}", result.error());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// The execution outcome (success, failure, interrupted, or max steps)
    pub outcome: ExecutionOutcome,
    /// Configuration used for execution
    pub config_used: Config,
}

impl ExecutionResult {
    /// Create a new execution result.
    pub fn new(outcome: ExecutionOutcome, config_used: Config) -> Self {
        Self {
            outcome,
            config_used,
        }
    }

    /// Check if the execution completed successfully.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use sage_sdk::SageAgentSdk;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let sdk = SageAgentSdk::new()?;
    /// let result = sdk.run("task").await?;
    /// if result.is_success() {
    ///     println!("Task completed successfully");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_success(&self) -> bool {
        self.outcome.is_success()
    }

    /// Check if the execution failed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use sage_sdk::SageAgentSdk;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let sdk = SageAgentSdk::new()?;
    /// let result = sdk.run("task").await?;
    /// if result.is_failed() {
    ///     println!("Error: {:?}", result.error());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_failed(&self) -> bool {
        self.outcome.is_failed()
    }

    /// Check if the execution was interrupted.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use sage_sdk::SageAgentSdk;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let sdk = SageAgentSdk::new()?;
    /// let result = sdk.run("task").await?;
    /// if result.is_interrupted() {
    ///     println!("Execution was interrupted");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_interrupted(&self) -> bool {
        self.outcome.is_interrupted()
    }

    /// Get the execution outcome.
    ///
    /// Returns a reference to the underlying `ExecutionOutcome` which contains
    /// detailed information about how the execution completed.
    pub fn outcome(&self) -> &ExecutionOutcome {
        &self.outcome
    }

    /// Get the underlying execution (regardless of outcome).
    ///
    /// Returns the complete execution state including all steps, messages,
    /// and tool interactions.
    pub fn execution(&self) -> &AgentExecution {
        self.outcome.execution()
    }

    /// Get the error if the execution failed.
    ///
    /// Returns `None` if the execution did not fail.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use sage_sdk::SageAgentSdk;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let sdk = SageAgentSdk::new()?;
    /// let result = sdk.run("task").await?;
    /// if let Some(error) = result.error() {
    ///     println!("Execution error: {}", error);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn error(&self) -> Option<&ExecutionError> {
        self.outcome.error()
    }

    /// Get the final result message from the agent.
    ///
    /// Returns `None` if no final result was produced.
    pub fn final_result(&self) -> Option<&str> {
        self.outcome.execution().final_result.as_deref()
    }

    /// Get a user-friendly status message.
    ///
    /// Returns a short status description suitable for display to users.
    pub fn status_message(&self) -> &'static str {
        self.outcome.status_message()
    }

    /// Get the status icon for CLI display.
    ///
    /// Returns an icon (emoji or symbol) representing the execution status.
    pub fn status_icon(&self) -> &'static str {
        self.outcome.status_icon()
    }
}
