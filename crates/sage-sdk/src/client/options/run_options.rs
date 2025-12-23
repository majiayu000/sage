//! Basic run options

use std::collections::HashMap;
use std::path::PathBuf;

/// Options for running tasks.
///
/// Provides fine-grained control over task execution behavior including
/// working directory, step limits, trajectory recording, and metadata.
///
/// # Examples
///
/// ```no_run
/// use sage_sdk::RunOptions;
///
/// let options = RunOptions::new()
///     .with_working_directory("/path/to/project")
///     .with_max_steps(50)
///     .with_trajectory(true);
/// ```
#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    /// Working directory for the task
    pub working_directory: Option<PathBuf>,
    /// Maximum number of steps
    pub max_steps: Option<u32>,
    /// Enable trajectory recording (kept for compatibility; recording is always on)
    pub enable_trajectory: bool,
    /// Custom trajectory file path
    pub trajectory_path: Option<PathBuf>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl RunOptions {
    /// Create new run options with default values.
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_sdk::RunOptions;
    ///
    /// let options = RunOptions::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Set working directory for task execution.
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_sdk::RunOptions;
    ///
    /// let options = RunOptions::new()
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
    /// use sage_sdk::RunOptions;
    ///
    /// let options = RunOptions::new()
    ///     .with_max_steps(100);
    /// ```
    pub fn with_max_steps(mut self, max_steps: u32) -> Self {
        self.max_steps = Some(max_steps);
        self
    }

    /// Enable or disable trajectory recording.
    ///
    /// Note: trajectory recording is always enabled in the runtime.
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_sdk::RunOptions;
    ///
    /// let options = RunOptions::new()
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
    /// use sage_sdk::RunOptions;
    ///
    /// let options = RunOptions::new()
    ///     .with_trajectory_path("logs/execution.json");
    /// ```
    pub fn with_trajectory_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.trajectory_path = Some(path.into());
        self.enable_trajectory = true;
        self
    }

    /// Add custom metadata to the execution.
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_sdk::RunOptions;
    ///
    /// let options = RunOptions::new()
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
