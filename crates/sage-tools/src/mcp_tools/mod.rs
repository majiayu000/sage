//! MCP (Model Context Protocol) Tools Integration
//!
//! This module provides integration between MCP servers and the Sage Agent tool system.
//! It allows MCP tools to be used as native Sage tools.
//!
//! # Features
//!
//! - Automatic tool discovery from MCP servers
//! - Schema conversion from MCP JSON Schema to Sage ToolSchema
//! - Transparent tool execution via MCP protocol
//! - Multiple server support
//!
//! # Example
//!
//! ```rust,ignore
//! use sage_tools::mcp_tools::{McpToolRegistry, create_mcp_registry};
//! use sage_core::config::McpConfig;
//!
//! // Create registry from config
//! let config = McpConfig::default();
//! let registry = create_mcp_registry(&config).await?;
//!
//! // Get all available tools
//! let tools = registry.all_tools().await;
//!
//! // Get tools from a specific server
//! let server_tools = registry.tools_from_server("my-server").await;
//! ```

pub mod adapter;
pub mod registry;

pub use adapter::{create_adapters_from_client, McpToolAdapter};
pub use registry::{
    create_mcp_registry, McpToolRegistry, ServerConnectionStatus, SharedMcpToolRegistry,
};
