//! Execution options for the unified execution loop
//!
//! This module provides configuration options for how the agent executes tasks,
//! following Claude Code's unified loop pattern where there's no distinction
//! between "run" and "interactive" modes at the core level.
//!
//! # Design
//!
//! The execution mode determines how user input requests are handled:
//!
//! - `Interactive`: Block and wait for actual user input via InputChannel
//! - `NonInteractive`: Auto-respond to input requests (CI/batch mode)
//! - `Batch`: Fail on any input request (strict headless mode)
//!
//! # Example
//!
//! ```ignore
//! let options = ExecutionOptions::interactive()
//!     .with_step_limit(200)  // Optional: set a step limit
//!     .with_execution_timeout(Duration::from_secs(3600))
//!     .with_trajectory(true);
//!
//! let result = executor.execute_task(task, options).await?;
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Auto-response strategy for non-interactive mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutoResponse {
    /// Always respond with a fixed string
    Fixed(String),
    /// Always select the first option (if options available)
    FirstOption,
    /// Always select the last option
    LastOption,
    /// Always cancel/skip the question
    Cancel,
    /// Use a custom response based on context
    ContextBased {
        /// Default response for text questions
        default_text: String,
        /// Whether to select first option when options available
        prefer_first_option: bool,
    },
}

impl Default for AutoResponse {
    fn default() -> Self {
        Self::ContextBased {
            default_text: "yes".to_string(),
            prefer_first_option: true,
        }
    }
}

impl AutoResponse {
    /// Create a fixed auto-response
    pub fn fixed(response: impl Into<String>) -> Self {
        Self::Fixed(response.into())
    }

    /// Create an auto-response that always selects the first option
    pub fn first_option() -> Self {
        Self::FirstOption
    }

    /// Create an auto-response that always cancels
    pub fn cancel() -> Self {
        Self::Cancel
    }

    /// Get the text response for a question
    pub fn get_text_response(&self) -> &str {
        match self {
            Self::Fixed(s) => s,
            Self::ContextBased { default_text, .. } => default_text,
            Self::FirstOption | Self::LastOption => "selected",
            Self::Cancel => "",
        }
    }

    /// Check if this response cancels
    pub fn is_cancel(&self) -> bool {
        matches!(self, Self::Cancel)
    }

    /// Should select the first option when options are available
    pub fn should_select_first(&self) -> bool {
        matches!(
            self,
            Self::FirstOption
                | Self::ContextBased {
                    prefer_first_option: true,
                    ..
                }
        )
    }

    /// Should select the last option when options are available
    pub fn should_select_last(&self) -> bool {
        matches!(self, Self::LastOption)
    }
}

/// Execution mode determines how user input is handled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Interactive mode: block and wait for real user input
    ///
    /// In this mode, when `ask_user_question` is called, the execution
    /// loop blocks on the InputChannel until the user responds.
    Interactive,

    /// Non-interactive mode: auto-respond to input requests
    ///
    /// Used for CI pipelines, batch processing, or automated testing.
    /// Input requests are automatically answered according to the
    /// auto_response strategy.
    NonInteractive {
        /// Strategy for auto-responding to questions
        auto_response: AutoResponse,
    },

    /// Batch mode: fail on any input request
    ///
    /// Strictest mode - any attempt to prompt for user input
    /// results in an error. Use when you need deterministic
    /// execution without any human interaction.
    Batch,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::Interactive
    }
}

impl ExecutionMode {
    /// Create interactive mode
    pub fn interactive() -> Self {
        Self::Interactive
    }

    /// Create non-interactive mode with default auto-response
    pub fn non_interactive() -> Self {
        Self::NonInteractive {
            auto_response: AutoResponse::default(),
        }
    }

    /// Create non-interactive mode with custom auto-response
    pub fn non_interactive_with(auto_response: AutoResponse) -> Self {
        Self::NonInteractive { auto_response }
    }

    /// Create batch mode (fail on input)
    pub fn batch() -> Self {
        Self::Batch
    }

    /// Check if this is interactive mode
    pub fn is_interactive(&self) -> bool {
        matches!(self, Self::Interactive)
    }

    /// Check if this is non-interactive mode
    pub fn is_non_interactive(&self) -> bool {
        matches!(self, Self::NonInteractive { .. })
    }

    /// Check if this is batch mode
    pub fn is_batch(&self) -> bool {
        matches!(self, Self::Batch)
    }

    /// Get the auto-response strategy if in non-interactive mode
    pub fn auto_response(&self) -> Option<&AutoResponse> {
        match self {
            Self::NonInteractive { auto_response } => Some(auto_response),
            _ => None,
        }
    }
}

/// Options for task execution
///
/// This unified configuration replaces the previous split between
/// "run mode" and "interactive mode" options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOptions {
    /// How to handle user input requests
    pub mode: ExecutionMode,

    /// Maximum number of agent steps before stopping (None = unlimited)
    pub max_steps: Option<u32>,

    /// Maximum total execution time (None = no limit)
    pub execution_timeout: Option<Duration>,

    /// Timeout for individual user prompts (None = wait indefinitely)
    /// Only applies to Interactive mode
    pub prompt_timeout: Option<Duration>,

    /// Whether to record execution trajectory
    pub record_trajectory: bool,

    /// Path for trajectory file (auto-generated if None)
    pub trajectory_path: Option<PathBuf>,

    /// Working directory for tool execution
    pub working_directory: Option<PathBuf>,

    /// Whether to continue on recoverable errors
    pub continue_on_error: bool,

    /// Whether to show verbose output
    pub verbose: bool,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            mode: ExecutionMode::Interactive,
            max_steps: None, // No limit by default - use CLI flag to set limit
            execution_timeout: None,
            prompt_timeout: None,
            record_trajectory: false,
            trajectory_path: None,
            working_directory: None,
            continue_on_error: false,
            verbose: false,
        }
    }
}

impl ExecutionOptions {
    /// Create options for interactive mode
    pub fn interactive() -> Self {
        Self {
            mode: ExecutionMode::Interactive,
            ..Default::default()
        }
    }

    /// Create options for non-interactive mode with auto-response
    pub fn non_interactive(auto_response: impl Into<String>) -> Self {
        Self {
            mode: ExecutionMode::NonInteractive {
                auto_response: AutoResponse::fixed(auto_response),
            },
            ..Default::default()
        }
    }

    /// Create options for batch mode (fail on input)
    pub fn batch() -> Self {
        Self {
            mode: ExecutionMode::Batch,
            ..Default::default()
        }
    }

    /// Set execution mode
    pub fn with_mode(mut self, mode: ExecutionMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set maximum steps (None = unlimited)
    pub fn with_max_steps(mut self, max_steps: Option<u32>) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// Set maximum steps with a specific limit
    pub fn with_step_limit(mut self, limit: u32) -> Self {
        self.max_steps = Some(limit);
        self
    }

    /// Set unlimited steps
    pub fn with_unlimited_steps(mut self) -> Self {
        self.max_steps = None;
        self
    }

    /// Set execution timeout
    pub fn with_execution_timeout(mut self, timeout: Duration) -> Self {
        self.execution_timeout = Some(timeout);
        self
    }

    /// Set prompt timeout (interactive mode only)
    pub fn with_prompt_timeout(mut self, timeout: Duration) -> Self {
        self.prompt_timeout = Some(timeout);
        self
    }

    /// Enable trajectory recording
    pub fn with_trajectory(mut self, record: bool) -> Self {
        self.record_trajectory = record;
        self
    }

    /// Set trajectory file path
    pub fn with_trajectory_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.trajectory_path = Some(path.into());
        self.record_trajectory = true;
        self
    }

    /// Set working directory
    pub fn with_working_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(path.into());
        self
    }

    /// Enable continue on error
    pub fn with_continue_on_error(mut self, continue_on_error: bool) -> Self {
        self.continue_on_error = continue_on_error;
        self
    }

    /// Enable verbose output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Check if running in interactive mode
    pub fn is_interactive(&self) -> bool {
        self.mode.is_interactive()
    }

    /// Check if trajectory recording is enabled
    pub fn should_record_trajectory(&self) -> bool {
        self.record_trajectory
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_mode_creation() {
        let interactive = ExecutionMode::interactive();
        assert!(interactive.is_interactive());

        let non_interactive = ExecutionMode::non_interactive();
        assert!(non_interactive.is_non_interactive());

        let batch = ExecutionMode::batch();
        assert!(batch.is_batch());
    }

    #[test]
    fn test_auto_response() {
        let fixed = AutoResponse::fixed("yes please");
        assert_eq!(fixed.get_text_response(), "yes please");

        let first = AutoResponse::first_option();
        assert!(first.should_select_first());

        let cancel = AutoResponse::cancel();
        assert!(cancel.is_cancel());
    }

    #[test]
    fn test_execution_options_builder() {
        let options = ExecutionOptions::interactive()
            .with_step_limit(50)
            .with_execution_timeout(Duration::from_secs(3600))
            .with_trajectory(true)
            .with_verbose(true);

        assert!(options.is_interactive());
        assert_eq!(options.max_steps, Some(50));
        assert_eq!(options.execution_timeout, Some(Duration::from_secs(3600)));
        assert!(options.record_trajectory);
        assert!(options.verbose);
    }

    #[test]
    fn test_non_interactive_options() {
        let options = ExecutionOptions::non_interactive("auto response");

        assert!(!options.is_interactive());
        if let ExecutionMode::NonInteractive { auto_response } = &options.mode {
            assert_eq!(auto_response.get_text_response(), "auto response");
        } else {
            panic!("Expected NonInteractive mode");
        }
    }

    #[test]
    fn test_batch_options() {
        let options = ExecutionOptions::batch();
        assert!(options.mode.is_batch());
    }

    #[test]
    fn test_trajectory_path_enables_recording() {
        let options = ExecutionOptions::interactive().with_trajectory_path("/tmp/trajectory.json");

        assert!(options.record_trajectory);
        assert_eq!(
            options.trajectory_path,
            Some(PathBuf::from("/tmp/trajectory.json"))
        );
    }
}
