//! Runtime MCP registry holder shared across execution entry points.

use super::registry::McpRegistry;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::sync::Arc;

static ACTIVE_MCP_REGISTRY: Lazy<RwLock<Option<Arc<McpRegistry>>>> =
    Lazy::new(|| RwLock::new(None));

/// Set the currently active MCP registry for runtime tools.
pub fn set_active_mcp_registry(registry: Arc<McpRegistry>) {
    *ACTIVE_MCP_REGISTRY.write() = Some(registry);
}

/// Get the currently active MCP registry for runtime tools.
pub fn get_active_mcp_registry() -> Option<Arc<McpRegistry>> {
    ACTIVE_MCP_REGISTRY.read().clone()
}

/// Clear the currently active MCP registry.
pub fn clear_active_mcp_registry() {
    *ACTIVE_MCP_REGISTRY.write() = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_get_and_clear_active_registry() {
        clear_active_mcp_registry();
        assert!(get_active_mcp_registry().is_none());

        let registry = Arc::new(McpRegistry::new());
        set_active_mcp_registry(Arc::clone(&registry));

        let active = get_active_mcp_registry().expect("active registry should exist");
        assert_eq!(active.server_names().len(), 0);

        clear_active_mcp_registry();
        assert!(get_active_mcp_registry().is_none());
    }
}
