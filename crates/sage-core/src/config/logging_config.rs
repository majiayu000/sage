//! Logging configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Whether to log to file
    pub log_to_file: bool,
    /// Log file path
    pub log_file: Option<PathBuf>,
    /// Whether to log to console
    pub log_to_console: bool,
    /// Log format (json, pretty, compact)
    pub format: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            log_to_file: false,
            log_file: None,
            log_to_console: true,
            format: "pretty".to_string(),
        }
    }
}

impl LoggingConfig {
    /// Merge with another logging config
    pub fn merge(&mut self, other: LoggingConfig) {
        if !other.level.is_empty() {
            self.level = other.level;
        }

        self.log_to_file = other.log_to_file;

        if other.log_file.is_some() {
            self.log_file = other.log_file;
        }

        self.log_to_console = other.log_to_console;

        if !other.format.is_empty() {
            self.format = other.format;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, "info");
        assert!(!config.log_to_file);
        assert!(config.log_to_console);
        assert_eq!(config.format, "pretty");
    }

    #[test]
    fn test_logging_config_merge() {
        let mut config1 = LoggingConfig::default();
        let config2 = LoggingConfig {
            level: "debug".to_string(),
            log_to_file: true,
            log_file: Some(PathBuf::from("/tmp/test.log")),
            log_to_console: false,
            format: "json".to_string(),
        };

        config1.merge(config2);
        assert_eq!(config1.level, "debug");
        assert!(config1.log_to_file);
        assert_eq!(config1.log_file, Some(PathBuf::from("/tmp/test.log")));
        assert!(!config1.log_to_console);
        assert_eq!(config1.format, "json");
    }

    #[test]
    fn test_logging_config_merge_empty_level() {
        let mut config1 = LoggingConfig::default();
        let config2 = LoggingConfig {
            level: "".to_string(),
            log_to_file: true,
            log_file: None,
            log_to_console: false,
            format: "".to_string(),
        };

        config1.merge(config2);
        // Empty strings should not override
        assert_eq!(config1.level, "info");
        assert_eq!(config1.format, "pretty");
        // But booleans should update
        assert!(config1.log_to_file);
        assert!(!config1.log_to_console);
    }
}
