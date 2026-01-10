//! Network configuration

use crate::llm::provider_types::TimeoutConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Network configuration for API communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// API endpoint base URL (overrides provider default)
    pub base_url: Option<String>,
    /// Custom HTTP headers to include in requests
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Timeout configuration for connection and request
    #[serde(default)]
    pub timeouts: TimeoutConfig,
    /// Legacy timeout field (deprecated, use `timeouts` instead)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            base_url: None,
            headers: HashMap::new(),
            timeouts: TimeoutConfig::default(),
            timeout: None,
        }
    }
}

impl NetworkConfig {
    /// Create a new network config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set base URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Add a custom header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set timeout configuration
    pub fn with_timeouts(mut self, timeouts: TimeoutConfig) -> Self {
        self.timeouts = timeouts;
        self
    }

    /// Get the effective timeout configuration
    pub fn get_effective_timeouts(&self) -> TimeoutConfig {
        let mut timeouts = self.timeouts;
        if let Some(legacy_timeout) = self.timeout {
            timeouts.request_timeout_secs = legacy_timeout;
        }
        timeouts
    }
}
