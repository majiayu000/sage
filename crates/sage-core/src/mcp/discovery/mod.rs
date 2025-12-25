//! MCP server discovery and management
//!
//! Provides mechanisms for discovering and connecting to MCP servers from:
//! - Configuration files
//! - Environment variables
//! - Standard paths
//! - Dynamic registration

mod builder;
mod connection;
mod health;
mod manager;
mod scanner;
#[cfg(test)]
mod tests;
mod types;
mod utils;

// Re-export public types
pub use builder::McpServerManagerBuilder;
pub use manager::McpServerManager;
pub use scanner::get_standard_mcp_paths;
pub use types::{DiscoverySource, ServerHealth, ServerStatus};
