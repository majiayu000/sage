//! SDK configuration builder methods

use crate::client::SageAgentSdk;
use sage_core::error::SageResult;
use std::path::PathBuf;

impl SageAgentSdk {
    /// Set provider and model.
    ///
    /// Configures the LLM provider and model to use for task execution.
    /// Optionally provide an API key (otherwise uses environment variables).
    ///
    /// # Arguments
    ///
    /// * `provider` - Provider name (e.g., "anthropic", "openai", "google")
    /// * `model` - Model identifier (e.g., "claude-3-5-sonnet-20241022")
    /// * `api_key` - Optional API key (defaults to environment variable)
    ///
    /// # Errors
    ///
    /// This method currently does not return errors but may in future versions
    /// if provider validation is added.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_sdk::SageAgentSdk;
    ///
    /// let sdk = SageAgentSdk::new()?
    ///     .with_provider_and_model("anthropic", "claude-3-5-sonnet-20241022", None)?;
    /// # Ok::<(), sage_sdk::SageError>(())
    /// ```
    pub fn with_provider_and_model(
        mut self,
        provider: &str,
        model: &str,
        api_key: Option<&str>,
    ) -> SageResult<Self> {
        // Update configuration
        if let Some(params) = self.config.model_providers.get_mut(provider) {
            params.model = model.to_string();
            if let Some(key) = api_key {
                params.api_key = Some(key.to_string());
            }
        } else {
            let params = sage_core::config::model::ModelParameters {
                model: model.to_string(),
                api_key: api_key.map(|k| k.to_string()),
                ..Default::default()
            };
            self.config
                .model_providers
                .insert(provider.to_string(), params);
        }

        self.config.default_provider = provider.to_string();
        Ok(self)
    }

    /// Set working directory.
    ///
    /// The working directory is where tools like Bash and Edit will operate.
    /// If not set, uses the current working directory.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_sdk::SageAgentSdk;
    ///
    /// let sdk = SageAgentSdk::new()?
    ///     .with_working_directory("/path/to/project");
    /// # Ok::<(), sage_sdk::SageError>(())
    /// ```
    pub fn with_working_directory<P: Into<PathBuf>>(mut self, working_dir: P) -> Self {
        self.config.working_directory = Some(working_dir.into());
        self
    }

    /// Set maximum steps (None = unlimited).
    ///
    /// Limits the number of agent reasoning steps to prevent infinite loops.
    /// Set to `None` for unlimited steps (use with caution).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_sdk::SageAgentSdk;
    ///
    /// let sdk = SageAgentSdk::new()?
    ///     .with_max_steps(Some(100));
    /// # Ok::<(), sage_sdk::SageError>(())
    /// ```
    pub fn with_max_steps(mut self, max_steps: Option<u32>) -> Self {
        self.config.max_steps = max_steps;
        self
    }

    /// Set a specific step limit.
    ///
    /// Convenience method for setting a specific maximum step count.
    /// Equivalent to `with_max_steps(Some(limit))`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_sdk::SageAgentSdk;
    ///
    /// let sdk = SageAgentSdk::new()?
    ///     .with_step_limit(50);
    /// # Ok::<(), sage_sdk::SageError>(())
    /// ```
    pub fn with_step_limit(mut self, limit: u32) -> Self {
        self.config.max_steps = Some(limit);
        self
    }
}
