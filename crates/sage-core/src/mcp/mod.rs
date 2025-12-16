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
//! - Server discovery from config, environment, and standard paths
//!
//! ## Example
//!
//! ```rust,ignore
//! use sage_core::mcp::{McpClient, StdioTransport, McpServerManager};
//!
//! // Option 1: Direct client usage
//! let transport = StdioTransport::spawn("mcp-server", &["--mode", "stdio"]).await?;
//! let client = McpClient::new(Box::new(transport));
//! client.initialize().await?;
//!
//! let tools = client.list_tools().await?;
//! let result = client.call_tool("read_file", json!({"path": "/tmp/test.txt"})).await?;
//!
//! // Option 2: Using server manager with auto-discovery
//! let manager = McpServerManager::new();
//! manager.discover(vec![DiscoverySource::Standard]).await?;
//!
//! let registry = manager.registry();
//! let tools = registry.all_tools().await;
//! ```

pub mod cache;
pub mod client;
pub mod discovery;
pub mod error;
pub mod notifications;
pub mod protocol;
pub mod registry;
pub mod schema_translator;
pub mod transport;
pub mod types;

pub use cache::{CacheConfig, CacheStats, McpCache};
pub use client::McpClient;
pub use discovery::{
    DiscoverySource, McpServerManager, McpServerManagerBuilder, ServerHealth, ServerStatus,
};
pub use error::McpError;
pub use notifications::{
    NotificationDispatcher, NotificationDispatcherBuilder, NotificationEvent, NotificationHandler,
};
pub use protocol::{McpMessage, McpNotification, McpRequest, McpResponse};
pub use registry::McpRegistry;
pub use schema_translator::SchemaTranslator;
pub use transport::{
    HttpTransport, HttpTransportConfig, McpTransport, StdioTransport, TransportConfig,
};
pub use types::{
    McpCapabilities, McpPrompt, McpResource, McpResourceContent, McpServerInfo, McpTool,
    McpToolResult,
};
