//! MCP server discovery and management
//!
//! Provides mechanisms for discovering and connecting to MCP servers from:
//! - Configuration files
//! - Environment variables
//! - Standard paths
//! - Dynamic registration

mod manager;
mod scanner;
#[cfg(test)]
mod tests;
mod types;

// Re-export public types
pub use manager::{McpServerManager, McpServerManagerBuilder};
pub use scanner::get_standard_mcp_paths;
pub use types::{DiscoverySource, ServerHealth, ServerStatus};
