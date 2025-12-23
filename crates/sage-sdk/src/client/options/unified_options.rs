//! Unified run options

use std::collections::HashMap;
use std::path::PathBuf;

/// Options for running tasks with the unified executor.
///
/// Extends `RunOptions` with support for non-interactive mode and advanced
/// execution control. Used with `execute_unified` and `execute_non_interactive`.
///
/// # Examples
///
/// ```no_run
/// use sage_sdk::UnifiedRunOptions;
///
/// let options = UnifiedRunOptions::new()
///     .with_non_interactive(true)
///     .with_max_steps(50);
/// ```
#[derive(Debug, Clone, Default)]
pub struct UnifiedRunOptions {
    /// Working directory for the task
    pub working_directory: Option<PathBuf>,
    /// Maximum number of steps
    pub max_steps: Option<u32>,
    /// Enable trajectory recording
    pub enable_trajectory: bool,
    /// Custom trajectory file path
    pub trajectory_path: Option<PathBuf>,
    /// Non-interactive mode (auto-respond to user questions)
    pub non_interactive: bool,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl UnifiedRunOptions {
    /// Create new unified run options with default values.
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_sdk::UnifiedRunOptions;
    ///
    /// let options = UnifiedRunOptions::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Set working directory for task execution.
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_sdk::UnifiedRunOptions;
    ///
    /// let options = UnifiedRunOptions::new()
    ///     .with_working_directory("/path/to/project");
    /// ```
    pub fn with_working_directory<P: Into<PathBuf>>(mut self, working_dir: P) -> Self {
        self.working_directory = Some(working_dir.into());
        self
    }

    /// Set maximum number of execution steps.
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_sdk::UnifiedRunOptions;
    ///
    /// let options = UnifiedRunOptions::new()
    ///     .with_max_steps(100);
    /// ```
    pub fn with_max_steps(mut self, max_steps: u32) -> Self {
        self.max_steps = Some(max_steps);
        self
    }

    /// Enable or disable trajectory recording.
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_sdk::UnifiedRunOptions;
    ///
    /// let options = UnifiedRunOptions::new()
    ///     .with_trajectory(true);
    /// ```
    pub fn with_trajectory(mut self, enabled: bool) -> Self {
        self.enable_trajectory = enabled;
        self
    }

    /// Set custom trajectory file path.
    ///
    /// Automatically enables trajectory recording.
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_sdk::UnifiedRunOptions;
    ///
    /// let options = UnifiedRunOptions::new()
    ///     .with_trajectory_path("logs/execution.json");
    /// ```
    pub fn with_trajectory_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.trajectory_path = Some(path.into());
        self.enable_trajectory = true;
        self
    }

    /// Set non-interactive mode.
    ///
    /// When enabled, the agent will automatically respond to user input prompts
    /// with default values instead of blocking for user input.
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_sdk::UnifiedRunOptions;
    ///
    /// let options = UnifiedRunOptions::new()
    ///     .with_non_interactive(true);
    /// ```
    pub fn with_non_interactive(mut self, non_interactive: bool) -> Self {
        self.non_interactive = non_interactive;
        self
    }

    /// Add custom metadata to the execution.
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_sdk::UnifiedRunOptions;
    ///
    /// let options = UnifiedRunOptions::new()
    ///     .with_metadata("task_id", "task-123")
    ///     .with_metadata("priority", 1);
    /// ```
    pub fn with_metadata<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<serde_json::Value>,
    {
        self.metadata.insert(key.into(), value.into());
        self
    }
}
