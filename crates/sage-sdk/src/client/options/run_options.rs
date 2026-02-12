//! Basic run options

use std::collections::HashMap;
use std::path::PathBuf;

/// Options for running tasks.
///
/// Provides fine-grained control over task execution behavior including
/// working directory, step limits, and metadata.
///
/// # Examples
///
/// ```no_run
/// use sage_sdk::RunOptions;
///
/// let options = RunOptions::new()
///     .with_working_directory("/path/to/project")
///     .with_max_steps(50);
/// ```
#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    /// Working directory for the task
    pub working_directory: Option<PathBuf>,
    /// Maximum number of steps
    pub max_steps: Option<u32>,
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
