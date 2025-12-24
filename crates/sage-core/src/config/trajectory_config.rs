//! Trajectory configuration
//!
//! Session recordings are stored in ~/.sage/projects/{escaped-cwd}/ as JSONL files.

use serde::{Deserialize, Serialize};

/// Trajectory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryConfig {
    /// Whether session recording is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl Default for TrajectoryConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl TrajectoryConfig {
    /// Check if session recording is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trajectory_config_default() {
        let config = TrajectoryConfig::default();
        assert!(config.enabled);
        assert!(config.is_enabled());
    }

    #[test]
    fn test_trajectory_config_disabled() {
        let config = TrajectoryConfig { enabled: false };
        assert!(!config.is_enabled());
    }
}
