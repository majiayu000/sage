//! MCP transport layer implementations
//!
//! Provides different transport mechanisms for MCP communication.
//!
//! ## Available Transports
//!
//! - **Stdio**: Standard I/O transport for subprocess-based MCP servers
//! - **HTTP**: HTTP transport with SSE support for HTTP-based MCP servers

pub mod http;
pub mod stdio;

pub use http::{HttpTransport, HttpTransportConfig};
pub use stdio::StdioTransport;

use super::error::McpError;
use super::protocol::McpMessage;
use async_trait::async_trait;

/// Transport trait for MCP communication
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// Send a message
    async fn send(&mut self, message: McpMessage) -> Result<(), McpError>;

    /// Receive a message
    async fn receive(&mut self) -> Result<McpMessage, McpError>;

    /// Close the transport
    async fn close(&mut self) -> Result<(), McpError>;

    /// Check if the transport is connected
    fn is_connected(&self) -> bool;
}

/// Transport configuration
#[derive(Debug, Clone)]
pub enum TransportConfig {
    /// Standard I/O transport
    Stdio {
        /// Command to spawn
        command: String,
        /// Command arguments
        args: Vec<String>,
        /// Environment variables
        env: std::collections::HashMap<String, String>,
    },
    /// HTTP transport (planned)
    Http {
        /// Base URL
        base_url: String,
        /// Headers
        headers: std::collections::HashMap<String, String>,
    },
    /// WebSocket transport (planned)
    WebSocket {
        /// WebSocket URL
        url: String,
    },
}

impl TransportConfig {
    /// Create a stdio transport config
    pub fn stdio(command: impl Into<String>, args: Vec<String>) -> Self {
        Self::Stdio {
            command: command.into(),
            args,
            env: std::collections::HashMap::new(),
        }
    }

    /// Create an HTTP transport config
    pub fn http(base_url: impl Into<String>) -> Self {
        Self::Http {
            base_url: base_url.into(),
            headers: std::collections::HashMap::new(),
        }
    }

    /// Create a WebSocket transport config
    pub fn websocket(url: impl Into<String>) -> Self {
        Self::WebSocket { url: url.into() }
    }
}
