//! SDK client implementation

use sage_core::{config::model::Config, error::SageResult};
use std::path::PathBuf;

// Module declarations - now using subdirectories
mod builder;
mod execution;
mod options;
mod result;

// Re-export public types
pub use options::{RunOptions, UnifiedRunOptions};
pub use result::ExecutionResult;

// Import and re-export outcome types from core
pub use sage_core::agent::{ExecutionError, ExecutionErrorKind, ExecutionOutcome};
pub use sage_core::input::InputRequest;

/// High-level SDK client for interacting with Sage Agent.
///
/// `SageAgentSdk` provides a fluent API for configuring and executing agent tasks.
/// It handles configuration loading, tool registration, trajectory recording,
/// and execution management.
///
/// # Examples
///
/// ```no_run
/// use sage_sdk::SageAgentSdk;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create SDK with default configuration
/// let sdk = SageAgentSdk::new()?;
///
/// // Execute a task
/// let result = sdk.run("Fix the bug in src/main.rs").await?;
/// println!("Task completed: {}", result.is_success());
/// # Ok(())
/// # }
/// ```
///
/// # Builder Pattern
///
/// The SDK uses a builder pattern for configuration:
///
/// ```no_run
/// use sage_sdk::SageAgentSdk;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let sdk = SageAgentSdk::new()?
///     .with_working_directory("/path/to/project")
///     .with_max_steps(Some(50))
///     .with_trajectory_path("output/trajectory.json");
///
/// let result = sdk.run("Implement new feature").await?;
/// # Ok(())
/// # }
/// ```
pub struct SageAgentSdk {
    pub(crate) config: Config,
    pub(crate) trajectory_path: Option<PathBuf>,
}

impl SageAgentSdk {
    /// Get the current configuration.
    ///
    /// Returns a reference to the SDK's configuration, allowing inspection
    /// of provider settings, model parameters, and other options.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_sdk::SageAgentSdk;
    ///
    /// let sdk = SageAgentSdk::new()?;
    /// let config = sdk.config();
    /// println!("Provider: {}", config.get_default_provider());
    /// # Ok::<(), sage_sdk::SageError>(())
    /// ```
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Validate the current configuration.
    ///
    /// Checks that the configuration is valid and can be used for execution.
    /// This includes verifying provider settings, API keys, and other required fields.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required provider configuration is missing
    /// - API keys are not set and not available in environment
    /// - Model parameters are invalid
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_sdk::SageAgentSdk;
    ///
    /// let sdk = SageAgentSdk::new()?;
    /// sdk.validate_config()?;
    /// # Ok::<(), sage_sdk::SageError>(())
    /// ```
    pub fn validate_config(&self) -> SageResult<()> {
        self.config.validate()
    }

    /// Get the current SDK API version
    ///
    /// Returns the semantic version of the SDK's public API.
    ///
    /// # Example
    ///
    /// ```
    /// use sage_sdk::SageAgentSdk;
    ///
    /// let sdk = SageAgentSdk::new().unwrap();
    /// let version = sdk.api_version();
    /// println!("SDK API Version: {}", version);
    /// ```
    pub fn api_version(&self) -> crate::version::Version {
        crate::version::API_VERSION
    }

    /// Get version information string
    ///
    /// Returns a formatted string with SDK version details.
    ///
    /// # Example
    ///
    /// ```
    /// use sage_sdk::SageAgentSdk;
    ///
    /// let sdk = SageAgentSdk::new().unwrap();
    /// println!("{}", sdk.version_info());
    /// ```
    pub fn version_info(&self) -> String {
        crate::version::version_info()
    }

    /// Check if a client version is compatible with this SDK
    ///
    /// Returns `true` if the specified client version can safely use this SDK.
    ///
    /// # Example
    ///
    /// ```
    /// use sage_sdk::{SageAgentSdk, version::Version};
    ///
    /// let sdk = SageAgentSdk::new().unwrap();
    /// let client_version = Version::new(0, 1, 0);
    /// assert!(sdk.is_compatible_with(&client_version));
    /// ```
    pub fn is_compatible_with(&self, client_version: &crate::version::Version) -> bool {
        crate::version::is_compatible(client_version)
    }
}
