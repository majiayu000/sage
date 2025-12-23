//! Centralized timeout configuration
//!
//! This module provides default timeout values for various operations.
//! All timeout values can be overridden via configuration.

use std::time::Duration;

/// Default timeout values for network operations
pub mod network {
    use super::*;

    /// Default timeout for web fetch operations (20 seconds)
    pub const WEB_FETCH_SECS: u64 = 20;

    /// Default timeout for web search operations (15 seconds)
    pub const WEB_SEARCH_SECS: u64 = 15;

    /// Default timeout for generic HTTP requests (30 seconds)
    pub const HTTP_REQUEST_SECS: u64 = 30;

    /// Get web fetch timeout as Duration
    pub fn web_fetch_timeout() -> Duration {
        Duration::from_secs(WEB_FETCH_SECS)
    }

    /// Get web search timeout as Duration
    pub fn web_search_timeout() -> Duration {
        Duration::from_secs(WEB_SEARCH_SECS)
    }

    /// Get HTTP request timeout as Duration
    pub fn http_request_timeout() -> Duration {
        Duration::from_secs(HTTP_REQUEST_SECS)
    }
}

/// Default timeout values for LLM operations
pub mod llm {
    use super::*;

    /// Default connection timeout for LLM APIs (30 seconds)
    pub const CONNECTION_SECS: u64 = 30;

    /// Default request timeout for LLM APIs (60 seconds)
    pub const REQUEST_SECS: u64 = 60;

    /// Get connection timeout as Duration
    pub fn connection_timeout() -> Duration {
        Duration::from_secs(CONNECTION_SECS)
    }

    /// Get request timeout as Duration
    pub fn request_timeout() -> Duration {
        Duration::from_secs(REQUEST_SECS)
    }
}

/// Default timeout values for task operations
pub mod task {
    use super::*;

    /// Default timeout for task output polling (30 seconds)
    pub const OUTPUT_POLL_SECS: u64 = 30;

    /// Maximum timeout for task output (10 minutes)
    pub const OUTPUT_MAX_SECS: u64 = 600;

    /// Default timeout for background tasks (5 minutes)
    pub const BACKGROUND_SECS: u64 = 300;

    /// Get output poll timeout as Duration
    pub fn output_poll_timeout() -> Duration {
        Duration::from_secs(OUTPUT_POLL_SECS)
    }

    /// Get output max timeout as Duration
    pub fn output_max_timeout() -> Duration {
        Duration::from_secs(OUTPUT_MAX_SECS)
    }

    /// Get background task timeout as Duration
    pub fn background_timeout() -> Duration {
        Duration::from_secs(BACKGROUND_SECS)
    }
}

/// Default timeout values for MCP operations
pub mod mcp {
    use super::*;

    /// Default timeout for MCP requests (5 minutes)
    pub const REQUEST_SECS: u64 = 300;

    /// Default timeout for MCP tool execution (5 minutes)
    pub const TOOL_EXECUTION_SECS: u64 = 300;

    /// Get MCP request timeout as Duration
    pub fn request_timeout() -> Duration {
        Duration::from_secs(REQUEST_SECS)
    }

    /// Get tool execution timeout as Duration
    pub fn tool_execution_timeout() -> Duration {
        Duration::from_secs(TOOL_EXECUTION_SECS)
    }
}

/// Default timeout values for bash/shell operations
pub mod bash {
    use super::*;

    /// Default timeout for bash commands (2 minutes)
    pub const COMMAND_SECS: u64 = 120;

    /// Maximum timeout for bash commands (10 minutes)
    pub const COMMAND_MAX_SECS: u64 = 600;

    /// Get command timeout as Duration
    pub fn command_timeout() -> Duration {
        Duration::from_secs(COMMAND_SECS)
    }

    /// Get max command timeout as Duration
    pub fn command_max_timeout() -> Duration {
        Duration::from_secs(COMMAND_MAX_SECS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_timeouts() {
        assert_eq!(network::WEB_FETCH_SECS, 20);
        assert_eq!(network::WEB_SEARCH_SECS, 15);
        assert_eq!(network::HTTP_REQUEST_SECS, 30);
    }

    #[test]
    fn test_llm_timeouts() {
        assert_eq!(llm::CONNECTION_SECS, 30);
        assert_eq!(llm::REQUEST_SECS, 60);
    }

    #[test]
    fn test_task_timeouts() {
        assert_eq!(task::OUTPUT_POLL_SECS, 30);
        assert_eq!(task::OUTPUT_MAX_SECS, 600);
    }

    #[test]
    fn test_duration_helpers() {
        assert_eq!(network::web_fetch_timeout(), Duration::from_secs(20));
        assert_eq!(llm::connection_timeout(), Duration::from_secs(30));
    }
}
