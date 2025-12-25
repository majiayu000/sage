//! Configuration and statistics for learning

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for learning mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    /// Whether learning mode is enabled
    pub enabled: bool,
    /// Minimum confidence threshold to apply a pattern
    pub apply_threshold: f32,
    /// Maximum number of patterns to store
    pub max_patterns: usize,
    /// Days before patterns start decaying
    pub decay_after_days: u32,
    /// Whether to learn from corrections automatically
    pub auto_learn_corrections: bool,
    /// Whether to learn from tool usage patterns
    pub auto_learn_tool_usage: bool,
    /// Whether to learn from code style
    pub auto_learn_code_style: bool,
    /// Store learning data persistently
    pub persistent: bool,
    /// Storage path for persistent learning
    pub storage_path: Option<std::path::PathBuf>,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            apply_threshold: 0.6,
            max_patterns: 500,
            decay_after_days: 30,
            auto_learn_corrections: true,
            auto_learn_tool_usage: true,
            auto_learn_code_style: true,
            persistent: true,
            storage_path: None,
        }
    }
}

impl LearningConfig {
    /// Create a disabled configuration
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Create with persistent storage
    pub fn with_storage(storage_path: std::path::PathBuf) -> Self {
        Self {
            storage_path: Some(storage_path),
            ..Default::default()
        }
    }
}

/// Statistics about learning
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearningStats {
    /// Total patterns stored
    pub total_patterns: usize,
    /// Patterns by type
    pub patterns_by_type: HashMap<String, usize>,
    /// Average confidence across patterns
    pub avg_confidence: f32,
    /// Number of high-confidence patterns
    pub high_confidence_count: usize,
    /// Patterns applied in current session
    pub patterns_applied: usize,
    /// Learning events in current session
    pub events_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_learning_config() {
        let config = LearningConfig::default();
        assert!(config.enabled);
        assert!(config.auto_learn_corrections);

        let disabled = LearningConfig::disabled();
        assert!(!disabled.enabled);
    }
}
