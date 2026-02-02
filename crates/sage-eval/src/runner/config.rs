//! Evaluation configuration
//!
//! Configuration options for running evaluations.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for evaluation runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalConfig {
    /// Path to sage configuration file
    pub config_file: PathBuf,

    /// Working directory for evaluation
    pub working_dir: Option<PathBuf>,

    /// Number of attempts per task (for Pass@K)
    #[serde(default = "default_attempts")]
    pub attempts: u32,

    /// Timeout per task in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Maximum steps per task
    #[serde(default = "default_max_steps")]
    pub max_steps: u32,

    /// Whether to continue on task failure
    #[serde(default = "default_continue_on_failure")]
    pub continue_on_failure: bool,

    /// Whether to save detailed results
    #[serde(default = "default_save_results")]
    pub save_results: bool,

    /// Output directory for results
    pub output_dir: Option<PathBuf>,

    /// Categories to run (empty = all)
    #[serde(default)]
    pub categories: Vec<String>,

    /// Specific task IDs to run (empty = all)
    #[serde(default)]
    pub task_ids: Vec<String>,

    /// Tags to filter by (empty = all)
    #[serde(default)]
    pub tags: Vec<String>,

    /// Whether to run in verbose mode
    #[serde(default)]
    pub verbose: bool,

    /// Whether to clean up sandbox after each task
    #[serde(default = "default_cleanup")]
    pub cleanup_sandbox: bool,
}

fn default_attempts() -> u32 {
    1
}

fn default_timeout() -> u64 {
    300
}

fn default_max_steps() -> u32 {
    50
}

fn default_continue_on_failure() -> bool {
    true
}

fn default_save_results() -> bool {
    true
}

fn default_cleanup() -> bool {
    true
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            config_file: PathBuf::from("sage_config.json"),
            working_dir: None,
            attempts: default_attempts(),
            timeout_secs: default_timeout(),
            max_steps: default_max_steps(),
            continue_on_failure: default_continue_on_failure(),
            save_results: default_save_results(),
            output_dir: None,
            categories: Vec::new(),
            task_ids: Vec::new(),
            tags: Vec::new(),
            verbose: false,
            cleanup_sandbox: default_cleanup(),
        }
    }
}

impl EvalConfig {
    /// Create a new config with the given sage config file
    pub fn new(config_file: impl Into<PathBuf>) -> Self {
        Self {
            config_file: config_file.into(),
            ..Default::default()
        }
    }

    /// Set number of attempts
    pub fn with_attempts(mut self, attempts: u32) -> Self {
        self.attempts = attempts;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Set max steps
    pub fn with_max_steps(mut self, steps: u32) -> Self {
        self.max_steps = steps;
        self
    }

    /// Set categories to run
    pub fn with_categories(mut self, categories: Vec<String>) -> Self {
        self.categories = categories;
        self
    }

    /// Set specific task IDs
    pub fn with_task_ids(mut self, ids: Vec<String>) -> Self {
        self.task_ids = ids;
        self
    }

    /// Set output directory
    pub fn with_output_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.output_dir = Some(dir.into());
        self
    }

    /// Enable verbose mode
    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    /// Get effective timeout for a task
    pub fn effective_timeout(&self, task_timeout: Option<u64>) -> u64 {
        task_timeout.unwrap_or(self.timeout_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EvalConfig::default();
        assert_eq!(config.attempts, 1);
        assert_eq!(config.timeout_secs, 300);
        assert_eq!(config.max_steps, 50);
        assert!(config.continue_on_failure);
    }

    #[test]
    fn test_config_builder() {
        let config = EvalConfig::new("test_config.json")
            .with_attempts(3)
            .with_timeout(600)
            .with_categories(vec!["code_generation".to_string()])
            .verbose();

        assert_eq!(config.attempts, 3);
        assert_eq!(config.timeout_secs, 600);
        assert_eq!(config.categories, vec!["code_generation"]);
        assert!(config.verbose);
    }
}
