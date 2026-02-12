//! Tool settings and configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolSettings {
    /// Enabled tools (if empty, all are enabled)
    #[serde(default)]
    pub enabled: Vec<String>,

    /// Disabled tools
    #[serde(default)]
    pub disabled: Vec<String>,

    /// Tool-specific configuration
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,

    /// Tool-specific timeouts (in milliseconds)
    #[serde(default)]
    pub timeouts: HashMap<String, u64>,
}

impl ToolSettings {
    /// Merge another tool settings
    pub fn merge(&mut self, other: ToolSettings) {
        self.enabled.extend(other.enabled);
        self.disabled.extend(other.disabled);
        self.config.extend(other.config);
        self.timeouts.extend(other.timeouts);
    }
}
