//! Model Context Protocol (MCP) integration for Sage Agent
//!
//! This module provides MCP client functionality for connecting to
//! external tool servers and services.
//!
//! ## Features
//!
//! - Multiple transport layers (stdio, HTTP, WebSocket)
//! - Tool discovery and execution
//! - Resource reading
//! - Prompt management
//!
//! ## Example
//!
//! ```rust,ignore
//! use sage_core::mcp::{McpClient, StdioTransport};
//!
//! let transport = StdioTransport::spawn("mcp-server", &["--mode", "stdio"]).await?;
//! let mut client = McpClient::new(Box::new(transport)).await?;
//!
//! let tools = client.list_tools().await?;
//! let result = client.call_tool("read_file", json!({"path": "/tmp/test.txt"})).await?;
//! ```

pub mod client;
pub mod error;
pub mod protocol;
pub mod registry;
pub mod transport;
pub mod types;

pub use client::McpClient;
pub use error::McpError;
pub use protocol::{McpMessage, McpNotification, McpRequest, McpResponse};
pub use registry::McpRegistry;
pub use transport::{McpTransport, StdioTransport};
pub use types::{
    McpCapabilities, McpPrompt, McpResource, McpResourceContent, McpServerInfo, McpTool,
    McpToolResult,
};
