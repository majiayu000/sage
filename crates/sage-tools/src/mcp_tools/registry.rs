//! MCP registry wrappers backed by sage-core.

use sage_core::config::Config;
use sage_core::error::SageResult;
use sage_core::mcp::{McpRegistry, build_mcp_registry_from_config};
use std::sync::Arc;

/// Canonical MCP registry type from sage-core.
pub type McpToolRegistry = McpRegistry;

/// Shared MCP registry handle.
pub type SharedMcpToolRegistry = Arc<McpToolRegistry>;

/// Create a shared MCP registry from full agent configuration.
pub async fn create_mcp_registry(config: &Config) -> SageResult<SharedMcpToolRegistry> {
    let registry = build_mcp_registry_from_config(config).await?;
    Ok(Arc::new(registry))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_registry_when_mcp_disabled() {
        let mut config = Config::default();
        config.mcp.enabled = false;

        let registry = create_mcp_registry(&config).await.unwrap();
        assert!(registry.server_names().is_empty());
    }
}
