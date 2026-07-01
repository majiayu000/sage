//! Deferred MCP tool index.

use super::runtime_status::McpToolDiscoveryState;
use super::types::McpTool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Freshness of a deferred MCP tool entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpToolFreshness {
    /// Entry was discovered during the latest successful refresh.
    Fresh,
    /// Entry may be stale because the server disconnected or refresh failed.
    Stale,
}

/// Searchable deferred MCP tool metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpDeferredTool {
    /// Namespaced Sage tool name.
    pub name: String,
    /// Remote MCP tool name.
    pub remote_name: String,
    /// Owning server id.
    pub server_id: String,
    /// Tool description.
    pub description: Option<String>,
    /// Original MCP input schema.
    pub input_schema: Value,
    /// Discovery timestamp.
    pub discovered_at: DateTime<Utc>,
    /// Freshness marker.
    pub freshness: McpToolFreshness,
}

/// Search result including server identity and freshness.
pub type McpDeferredToolSearchResult = McpDeferredTool;

/// Deferred tool list/search index.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpDeferredToolIndex {
    tools: BTreeMap<String, McpDeferredTool>,
    server_states: BTreeMap<String, McpToolDiscoveryState>,
}

impl McpDeferredToolIndex {
    /// Create an empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a server's discovery state.
    pub fn mark_server(&mut self, server_id: impl Into<String>, state: McpToolDiscoveryState) {
        self.server_states.insert(server_id.into(), state);
    }

    /// Remove all cached tools for a server.
    pub fn remove_server(&mut self, server_id: &str) {
        self.tools.retain(|_, tool| tool.server_id != server_id);
        self.server_states.remove(server_id);
    }

    /// Mark a server's cached tools as stale.
    pub fn mark_server_stale(&mut self, server_id: &str) {
        for tool in self.tools.values_mut() {
            if tool.server_id == server_id {
                tool.freshness = McpToolFreshness::Stale;
            }
        }
        self.server_states
            .insert(server_id.to_string(), McpToolDiscoveryState::Stale);
    }

    /// Replace cached tools for one server.
    pub fn replace_server_tools(
        &mut self,
        server_id: impl Into<String>,
        tools: impl IntoIterator<Item = McpTool>,
    ) {
        let server_id = server_id.into();
        self.tools.retain(|_, tool| tool.server_id != server_id);

        let discovered_at = Utc::now();
        for tool in tools {
            let name = namespaced_tool_name(&server_id, &tool.name);
            self.tools.insert(
                name.clone(),
                McpDeferredTool {
                    name,
                    remote_name: tool.name,
                    server_id: server_id.clone(),
                    description: tool.description,
                    input_schema: tool.input_schema,
                    discovered_at,
                    freshness: McpToolFreshness::Fresh,
                },
            );
        }

        self.server_states
            .insert(server_id, McpToolDiscoveryState::Fresh);
    }

    /// Return all deferred tools in deterministic order.
    pub fn list(&self) -> Vec<McpDeferredTool> {
        self.tools.values().cloned().collect()
    }

    /// Search by namespaced name, remote name, description, or server id.
    pub fn search(&self, query: &str) -> Vec<McpDeferredToolSearchResult> {
        let query = query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return self.list();
        }

        self.tools
            .values()
            .filter(|tool| {
                tool.name.to_ascii_lowercase().contains(&query)
                    || tool.remote_name.to_ascii_lowercase().contains(&query)
                    || tool.server_id.to_ascii_lowercase().contains(&query)
                    || tool
                        .description
                        .as_deref()
                        .unwrap_or_default()
                        .to_ascii_lowercase()
                        .contains(&query)
            })
            .cloned()
            .collect()
    }

    /// Return one server discovery state.
    pub fn server_state(&self, server_id: &str) -> Option<McpToolDiscoveryState> {
        self.server_states.get(server_id).cloned()
    }
}

/// Build the exposed Sage tool name for an MCP tool.
pub fn namespaced_tool_name(server_id: &str, remote_tool_name: &str) -> String {
    format!(
        "{}{}",
        namespaced_tool_prefix(server_id),
        normalize_component(remote_tool_name)
    )
}

/// Build the exposed Sage tool name prefix for an MCP server.
pub fn namespaced_tool_prefix(server_id: &str) -> String {
    format!("mcp__{}__", normalize_component(server_id))
}

fn normalize_component(input: &str) -> String {
    let mut normalized = String::new();
    let mut previous_was_underscore = false;

    for ch in input.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '_'
        };

        if mapped == '_' {
            if previous_was_underscore {
                continue;
            }
            previous_was_underscore = true;
        } else {
            previous_was_underscore = false;
        }

        normalized.push(mapped);
    }

    let normalized = normalized.trim_matches('_');
    if normalized.is_empty() {
        "unnamed".to_string()
    } else {
        normalized.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mcp_deferred_tools_search_returns_server_identity_and_freshness() {
        let mut index = McpDeferredToolIndex::new();
        index.replace_server_tools(
            "docs",
            [McpTool::new("Read File")
                .with_description("Read a documentation file")
                .with_input_schema(json!({"type":"object"}))],
        );

        let results = index.search("documentation");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "mcp__docs__read_file");
        assert_eq!(results[0].server_id, "docs");
        assert_eq!(results[0].freshness, McpToolFreshness::Fresh);
        assert_eq!(
            index.server_state("docs"),
            Some(McpToolDiscoveryState::Fresh)
        );
    }

    #[test]
    fn mcp_deferred_tools_marks_server_tools_stale() {
        let mut index = McpDeferredToolIndex::new();
        index.replace_server_tools("docs", [McpTool::new("read")]);
        index.mark_server_stale("docs");

        assert_eq!(index.list()[0].freshness, McpToolFreshness::Stale);
        assert_eq!(
            index.server_state("docs"),
            Some(McpToolDiscoveryState::Stale)
        );
    }
}
