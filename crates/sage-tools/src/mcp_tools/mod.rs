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
//! - McpServersTool for runtime server management
//!
//! # Example
//!
//! ```rust,ignore
//! use sage_tools::mcp_tools::{McpToolRegistry, create_mcp_registry, McpServersTool};
//! use sage_core::config::Config;
//!
//! // Create registry from config
//! let config = Config::default();
//! let registry = create_mcp_registry(&config).await?;
//!
//! // Initialize global registry for McpServersTool
//! init_global_mcp_registry(registry.clone()).await?;
//!
//! // Get all available tools
//! let tools = registry.all_tools().await;
//! ```

pub mod registry;
pub mod servers_tool;

pub use registry::{McpToolRegistry, SharedMcpToolRegistry, create_mcp_registry};
pub use sage_core::mcp::McpToolAdapter;
pub use servers_tool::{
    McpServersTool, get_global_mcp_registry, get_mcp_tools, init_global_mcp_registry,
};
