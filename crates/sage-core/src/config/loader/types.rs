//! Configuration source types

use std::collections::HashMap;
use std::path::PathBuf;

/// Source of configuration data
#[derive(Debug, Clone)]
pub enum ConfigSource {
    /// Configuration from a file
    File(PathBuf),
    /// Configuration from environment variables
    Environment,
    /// Configuration from command line arguments
    CommandLine(HashMap<String, String>),
    /// Default configuration
    Default,
}
