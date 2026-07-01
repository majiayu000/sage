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

pub mod auth_status;
pub mod cache;
pub mod client;
pub mod config_builder;
pub mod deferred_tools;
pub mod discovery;
pub mod error;
pub mod notifications;
pub mod protocol;
pub mod registry;
mod registry_runtime;
pub mod runtime_registry;
pub mod runtime_status;
pub mod schema_translator;
pub mod source;
pub mod transport;
pub mod types;

#[cfg(test)]
mod runtime_tests;

pub use auth_status::{McpAuthState, McpAuthStatus, McpAuthorizationPrompt};
pub use cache::{McpCache, McpCacheConfig, McpCacheStats};
pub use client::McpClient;
pub use config_builder::{
    build_mcp_registry_from_config, build_mcp_registry_from_config_and_packages,
};
pub use deferred_tools::{
    McpDeferredTool, McpDeferredToolIndex, McpDeferredToolSearchResult, McpToolFreshness,
};
pub use discovery::{
    DiscoverySource, McpServerManager, McpServerManagerBuilder, ServerHealth, ServerStatus,
};
pub use error::McpError;
pub use notifications::{
    NotificationDispatcher, NotificationDispatcherBuilder, NotificationEvent, NotificationHandler,
};
pub use protocol::{McpMessage, McpNotification, McpRequest, McpResponse};
pub use registry::{McpRegistry, McpToolAdapter};
pub use runtime_registry::{
    clear_active_mcp_registry, get_active_mcp_registry, set_active_mcp_registry,
};
pub use runtime_status::{
    McpFailureKind, McpRuntimeAction, McpRuntimeActionResult, McpRuntimeState,
    McpServerRuntimeStatus, McpStructuredFailure, McpToolDiscoveryState,
};
pub use schema_translator::SchemaTranslator;
pub use source::{
    McpServerSource, McpSourceKind, McpSourceMergeError, McpSourceMetadata, McpSourceSet,
    direct_config_sources, merge_mcp_sources, package_sources,
};
pub use transport::{
    HttpTransport, HttpTransportConfig, McpTransport, StdioTransport, TransportConfig,
};
pub use types::{
    McpCapabilities, McpContent, McpPrompt, McpResource, McpResourceContent, McpServerInfo,
    McpTool, McpToolResult,
};
