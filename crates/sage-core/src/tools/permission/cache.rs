//! Permission cache for "always allow" / "always deny" decisions

use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::tools::types::ToolCall;

/// Permission cache for "always allow" / "always deny" decisions
#[derive(Debug, Default)]
pub struct PermissionCache {
    allowed: RwLock<HashMap<String, bool>>,
}

impl PermissionCache {
    /// Create a new permission cache
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate cache key for a tool call
    pub fn cache_key(tool_name: &str, call: &ToolCall) -> String {
        // Simple key based on tool name and argument keys
        let arg_keys: Vec<_> = call.arguments.keys().collect();
        format!("{}:{:?}", tool_name, arg_keys)
    }

    /// Check if there's a cached decision
    pub async fn get(&self, key: &str) -> Option<bool> {
        self.allowed.read().await.get(key).copied()
    }

    /// Cache a decision
    pub async fn set(&self, key: String, allowed: bool) {
        self.allowed.write().await.insert(key, allowed);
    }

    /// Clear the cache
    pub async fn clear(&self) {
        self.allowed.write().await.clear();
    }
}
