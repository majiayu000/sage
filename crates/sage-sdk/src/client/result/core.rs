//! Core execution result type

use sage_core::{
    agent::{AgentExecution, ExecutionError, ExecutionOutcome},
    config::model::Config,
};
use std::path::PathBuf;

/// Result of task execution.
///
/// Contains the execution outcome and a safe SDK-owned execution config summary.
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
    /// Safe configuration summary used for execution.
    pub config_summary: ExecutionConfigSummary,
}

/// SDK-owned summary of the configuration used for execution.
///
/// This type intentionally exposes only non-secret execution context. It must
/// not grow provider credential fields such as API keys, base URLs, headers, or
/// complete `sage_core::config::Config` values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionConfigSummary {
    /// Selected provider id.
    pub provider: String,
    /// Selected model id, if known.
    pub model: Option<String>,
    /// Working directory used for execution.
    pub working_directory: PathBuf,
    /// Effective step limit.
    pub max_steps: Option<u32>,
    /// Whether execution ran without interactive input.
    pub non_interactive: bool,
}

impl ExecutionConfigSummary {
    /// Build a safe summary from the SDK config and resolved execution context.
    pub(crate) fn from_config(
        config: &Config,
        working_directory: PathBuf,
        max_steps: Option<u32>,
        non_interactive: bool,
    ) -> Self {
        let provider = config.default_provider.clone();
        let model = config
            .model_providers
            .get(&provider)
            .map(|params| params.model.trim().to_string())
            .filter(|model| !model.is_empty());

        Self {
            provider,
            model,
            working_directory,
            max_steps,
            non_interactive,
        }
    }
}

impl ExecutionResult {
    /// Create a new execution result.
    pub fn new(outcome: ExecutionOutcome, config_summary: ExecutionConfigSummary) -> Self {
        Self {
            outcome,
            config_summary,
        }
    }

    /// Get the safe configuration summary used for execution.
    pub fn config_summary(&self) -> &ExecutionConfigSummary {
        &self.config_summary
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

#[cfg(test)]
mod tests {
    use super::*;
    use sage_core::config::model::Config;

    #[test]
    fn config_summary_exposes_only_safe_fields() {
        let mut config = Config::default();
        let provider = config.default_provider.clone();
        let Some(params) = config.model_providers.get_mut(&provider) else {
            panic!("default provider should exist in default config");
        };
        params.model = "safe-model".to_string();
        params.api_key = Some("sk-secret-value".to_string());
        params.base_url = Some("https://secret.example.test".to_string());

        let summary = ExecutionConfigSummary::from_config(
            &config,
            PathBuf::from("/tmp/sage-sdk"),
            Some(10),
            true,
        );

        assert_eq!(summary.provider, provider);
        assert_eq!(summary.model.as_deref(), Some("safe-model"));
        assert_eq!(summary.max_steps, Some(10));
        assert!(summary.non_interactive);

        let debug = format!("{summary:?}");
        assert!(!debug.contains("sk-secret-value"));
        assert!(!debug.contains("secret.example.test"));
    }
}
