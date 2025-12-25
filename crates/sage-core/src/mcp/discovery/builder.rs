//! Builder for creating McpServerManager with custom settings

use super::manager::McpServerManager;
use std::time::Duration;

/// Builder for creating McpServerManager with custom settings
pub struct McpServerManagerBuilder {
    timeout: Duration,
    auto_reconnect: bool,
    health_check_interval: Option<Duration>,
}

impl McpServerManagerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(300),
            auto_reconnect: false,
            health_check_interval: None,
        }
    }

    /// Set request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable auto-reconnect on failure
    pub fn with_auto_reconnect(mut self, enabled: bool) -> Self {
        self.auto_reconnect = enabled;
        self
    }

    /// Set health check interval
    pub fn with_health_check_interval(mut self, interval: Duration) -> Self {
        self.health_check_interval = Some(interval);
        self
    }

    /// Build the manager
    pub fn build(self) -> McpServerManager {
        McpServerManager::with_timeout(self.timeout)
    }
}

impl Default for McpServerManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
