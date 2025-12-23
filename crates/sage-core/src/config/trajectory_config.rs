//! Trajectory configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Helper function for default true value
pub(crate) fn default_true() -> bool {
    true
}

/// Trajectory configuration
/// Note: Trajectory recording is always enabled and cannot be disabled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryConfig {
    /// Directory to store trajectory files
    pub directory: PathBuf,
    /// Whether to auto-save trajectories during execution
    pub auto_save: bool,
    /// Number of steps between auto-saves
    pub save_interval_steps: usize,
    /// Whether to compress trajectory files with gzip
    #[serde(default = "default_true")]
    pub enable_compression: bool,
}

impl Default for TrajectoryConfig {
    fn default() -> Self {
        Self {
            // Note: trajectory is always enabled, no enabled field
            directory: PathBuf::from("trajectories"),
            auto_save: true,
            save_interval_steps: 5,
            enable_compression: true, // Enable compression by default
        }
    }
}

impl TrajectoryConfig {
    /// Trajectory is always enabled - this is a required feature
    pub fn is_enabled(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trajectory_config_default() {
        let config = TrajectoryConfig::default();
        assert_eq!(config.directory, PathBuf::from("trajectories"));
        assert!(config.auto_save);
        assert_eq!(config.save_interval_steps, 5);
        assert!(config.enable_compression);
    }

    #[test]
    fn test_trajectory_config_is_enabled() {
        let config = TrajectoryConfig::default();
        assert!(config.is_enabled()); // Always returns true
    }

    #[test]
    fn test_default_true() {
        assert_eq!(default_true(), true);
    }
}
