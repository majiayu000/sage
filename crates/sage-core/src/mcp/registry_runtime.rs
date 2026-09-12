//! Runtime source/status extensions for the MCP registry.

use super::auth_status::{McpAuthStatus, McpAuthorizationPrompt};
use super::client::McpClient;
use super::deferred_tools::{McpDeferredTool, McpDeferredToolIndex, namespaced_tool_prefix};
use super::discovery::utils::server_config_to_transport;
use super::error::McpError;
use super::registry::{McpRegistry, McpToolAdapter, ToolRoute};
use super::runtime_status::{
    McpRuntimeAction, McpRuntimeActionResult, McpServerRuntimeStatus, McpToolDiscoveryState,
};
use super::source::{McpSourceSet, MergedMcpServerSource};
use super::tool_trust::{McpToolTrustDecision, McpToolTrustStore, validate_tool_description_trust};
use super::types::McpTool;
use crate::config::{McpAuthKind, McpServerConfig};
use std::sync::Arc;

impl McpRegistry {
    /// Replace configured MCP sources and initialize runtime status without connecting.
    pub fn apply_source_set(&self, source_set: McpSourceSet) {
        self.clients.clear();
        self.tool_mapping.clear();
        self.resource_mapping.clear();
        self.prompt_mapping.clear();
        self.sources.clear();
        self.statuses.clear();
        *self.deferred_tools.write() = McpDeferredToolIndex::new();

        for (server_id, source) in source_set.servers {
            let status = McpServerRuntimeStatus::from_source(&source);
            self.deferred_tools
                .write()
                .mark_server(&server_id, status.tool_discovery_state.clone());
            self.sources.insert(server_id.clone(), source);
            self.statuses.insert(server_id, status);
        }
    }

    /// Return all configured source ids.
    pub fn configured_server_names(&self) -> Vec<String> {
        let mut names = self
            .sources
            .iter()
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    /// Return structured runtime status for a server.
    pub fn server_runtime_status(&self, server_name: &str) -> Option<McpServerRuntimeStatus> {
        self.statuses
            .get(server_name)
            .map(|entry| entry.value().clone())
    }

    /// Return all structured runtime statuses.
    pub fn runtime_statuses(&self) -> Vec<McpServerRuntimeStatus> {
        let mut statuses = self
            .statuses
            .iter()
            .map(|entry| entry.value().clone())
            .collect::<Vec<_>>();
        statuses.sort_by(|a, b| a.server_id.cmp(&b.server_id));
        statuses
    }

    /// List deferred MCP tools without connecting additional servers.
    pub fn deferred_tools(&self) -> Vec<McpDeferredTool> {
        self.deferred_tools.read().list()
    }

    /// Search deferred MCP tools without connecting additional servers.
    pub fn search_deferred_tools(&self, query: &str) -> Vec<McpDeferredTool> {
        self.deferred_tools.read().search(query)
    }

    /// Connect a configured MCP source.
    pub async fn connect_configured_server(
        &self,
        name: &str,
    ) -> Result<McpRuntimeActionResult, McpError> {
        let source = self.configured_source(name)?;
        let mut status = self.current_or_initial_status(&source);
        refresh_status_auth(&source, &mut status);

        if !status.enabled {
            let error = McpError::disabled(name);
            status.mark_error(&error);
            self.store_status(status);
            return Err(error);
        }

        if status.auth_blocks_tools() {
            let prompt = status
                .auth
                .prompt
                .clone()
                .unwrap_or_else(|| McpAuthorizationPrompt {
                    server_id: name.to_string(),
                    kind: McpAuthKind::None,
                    message: "Authorize this MCP server before running tools".to_string(),
                    authorization_url: None,
                    token_env: None,
                    scopes: Vec::new(),
                });
            let error = McpError::auth_required(name, prompt);
            status.mark_error(&error);
            self.store_status(status);
            return Err(error);
        }

        if let Err(error) = ensure_supported_transport(&source.selected.config) {
            status.mark_error(&error);
            self.store_status(status);
            return Err(error);
        }

        status.mark_connecting();
        self.store_status(status.clone());

        let transport_config = match server_config_to_transport(&source.selected.config) {
            Ok(config) => config,
            Err(error) => {
                status.mark_error(&error);
                self.store_status(status);
                return Err(error);
            }
        };

        match self.register_server(name, transport_config).await {
            Ok(_) => {
                status.mark_connected();
                self.store_status(status.clone());
                Ok(McpRuntimeActionResult {
                    action: McpRuntimeAction::Connect,
                    status,
                })
            }
            Err(error) => {
                status.mark_error(&error);
                self.store_status(status);
                Err(error)
            }
        }
    }

    /// Disconnect a configured MCP source.
    pub async fn disconnect_configured_server(
        &self,
        name: &str,
    ) -> Result<McpRuntimeActionResult, McpError> {
        let source = self.configured_source(name)?;
        self.unregister_server(name).await?;

        let mut status = self.current_or_initial_status(&source);
        status.mark_disconnected();
        self.deferred_tools.write().mark_server_stale(name);
        self.store_status(status.clone());

        Ok(McpRuntimeActionResult {
            action: McpRuntimeAction::Disconnect,
            status,
        })
    }

    /// Retry a configured MCP source.
    pub async fn retry_configured_server(
        &self,
        name: &str,
    ) -> Result<McpRuntimeActionResult, McpError> {
        if let Err(error) = self.unregister_server(name).await {
            tracing::debug!(
                "Ignoring disconnect failure before MCP retry for '{}': {}",
                name,
                error
            );
        }
        self.deferred_tools.write().mark_server_stale(name);
        self.connect_configured_server(name)
            .await
            .map(|mut result| {
                result.action = McpRuntimeAction::Retry;
                result
            })
    }

    pub(super) fn status_for_tool_name(&self, tool_name: &str) -> Option<McpServerRuntimeStatus> {
        self.statuses.iter().find_map(|entry| {
            let status = entry.value();
            if tool_name.starts_with(&namespaced_tool_prefix(&status.server_id)) {
                Some(status.clone())
            } else {
                None
            }
        })
    }

    pub(crate) async fn refresh_server_capabilities(
        &self,
        name: &str,
        client: &Arc<McpClient>,
    ) -> Result<(), McpError> {
        let refresh_result = self.refresh_server_capabilities_inner(name, client).await;
        if let Err(error) = &refresh_result {
            // Fail closed: do not keep previously trusted routes/cache when a
            // refresh cannot complete (e.g. drifted tools mixed with schema errors).
            self.clear_server_trusted_tools(name, client, error).await;
        }
        refresh_result
    }

    async fn refresh_server_capabilities_inner(
        &self,
        name: &str,
        client: &Arc<McpClient>,
    ) -> Result<(), McpError> {
        let tools = client.list_tools_uncached().await.map_err(|error| {
            McpError::schema(format!(
                "Failed to discover tools for MCP server '{name}': {error}"
            ))
        })?;

        let mut trust_store = McpToolTrustStore::load_default()?;
        let warn_on_drift = self.warn_on_tool_trust_drift();
        let trusted_tools =
            trusted_mcp_tools_for_server(name, tools, &mut trust_store, warn_on_drift)?;
        trust_store.save_if_dirty()?;

        self.tool_mapping
            .retain(|_, route| route.server_name != name);
        let mut routed_tools = Vec::with_capacity(trusted_tools.len());
        for (tool, trust_decision) in &trusted_tools {
            log_mcp_tool_trust_decision(name, &tool.name, trust_decision.clone());
            self.warn_remote_tool_name_collision(name, &tool.name);
            let namespaced_name = McpToolAdapter::namespaced_tool_name(name, &tool.name);
            if self.warn_namespaced_tool_route_collision(name, &tool.name, &namespaced_name) {
                continue;
            }
            self.tool_mapping.insert(
                namespaced_name,
                ToolRoute {
                    server_name: name.to_string(),
                    remote_name: tool.name.clone(),
                },
            );
            routed_tools.push(tool.clone());
        }
        client.replace_trusted_tools(routed_tools.clone()).await;
        self.deferred_tools
            .write()
            .replace_server_tools(name.to_string(), routed_tools);

        if let Ok(resources) = client.list_resources().await {
            for resource in resources {
                self.resource_mapping
                    .insert(resource.uri.clone(), name.to_string());
            }
        }

        if let Ok(prompts) = client.list_prompts().await {
            for prompt in prompts {
                self.prompt_mapping
                    .insert(prompt.name.clone(), name.to_string());
            }
        }

        Ok(())
    }

    async fn clear_server_trusted_tools(
        &self,
        name: &str,
        client: &Arc<McpClient>,
        error: &McpError,
    ) {
        self.tool_mapping
            .retain(|_, route| route.server_name != name);
        client.replace_trusted_tools(Vec::new()).await;

        let discovery_state = match error {
            McpError::Schema { .. } => McpToolDiscoveryState::SchemaError,
            _ => McpToolDiscoveryState::Stale,
        };
        self.deferred_tools
            .write()
            .clear_server_tools(name, discovery_state.clone());

        if let Some(entry) = self.statuses.get(name) {
            let mut status = entry.value().clone();
            drop(entry);
            status.mark_error(error);
            // Keep the discovery state derived above when mark_error would
            // otherwise leave a non-schema failure looking merely Deferred.
            status.tool_discovery_state = discovery_state;
            self.store_status(status);
        }
    }

    fn configured_source(&self, name: &str) -> Result<MergedMcpServerSource, McpError> {
        self.sources
            .get(name)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| McpError::connection(format!("MCP server '{name}' is not configured")))
    }
    fn current_or_initial_status(&self, source: &MergedMcpServerSource) -> McpServerRuntimeStatus {
        self.statuses
            .get(&source.selected.server_id)
            .map(|entry| entry.value().clone())
            .unwrap_or_else(|| McpServerRuntimeStatus::from_source(source))
    }
    fn store_status(&self, status: McpServerRuntimeStatus) {
        self.deferred_tools
            .write()
            .mark_server(&status.server_id, status.tool_discovery_state.clone());
        self.statuses.insert(status.server_id.clone(), status);
    }
    fn warn_remote_tool_name_collision(&self, server_name: &str, remote_name: &str) -> usize {
        let mut collision_count = 0;
        for entry in self.tool_mapping.iter() {
            let route = entry.value();
            if route.server_name != server_name && route.remote_name == remote_name {
                collision_count += 1;
                tracing::warn!(
                    server = server_name,
                    existing_server = route.server_name.as_str(),
                    tool = remote_name,
                    namespaced_tool = entry.key().as_str(),
                    "MCP tool name collision detected; Sage keeps server-qualified tool routes to avoid shadowing"
                );
            }
        }
        collision_count
    }
    fn warn_namespaced_tool_route_collision(
        &self,
        server_name: &str,
        remote_name: &str,
        namespaced_name: &str,
    ) -> bool {
        let Some(existing) = self.tool_mapping.get(namespaced_name) else {
            return false;
        };
        if existing.server_name == server_name && existing.remote_name == remote_name {
            return false;
        }
        tracing::warn!(
            server = server_name,
            existing_server = existing.server_name.as_str(),
            tool = remote_name,
            existing_tool = existing.remote_name.as_str(),
            namespaced_tool = namespaced_name,
            "MCP tool namespaced route collision detected; skipping the later route to avoid silent shadowing"
        );
        true
    }
}
fn trusted_mcp_tools_for_server(
    server_name: &str,
    tools: Vec<McpTool>,
    trust_store: &mut McpToolTrustStore,
    warn_on_drift: bool,
) -> Result<Vec<(McpTool, McpToolTrustDecision)>, McpError> {
    let mut trusted_tools = Vec::with_capacity(tools.len());
    for tool in tools {
        validate_mcp_tool_schema(server_name, &tool)?;
        if let Err(error) = validate_tool_description_trust(server_name, &tool) {
            tracing::warn!(
                server = server_name,
                tool = tool.name.as_str(),
                error = %error,
                "Skipping untrusted MCP tool while keeping the server available"
            );
            continue;
        }
        let trust_decision = trust_store.check_tool(server_name, &tool);
        if let McpToolTrustDecision::Drift { previous, current } = &trust_decision {
            if !warn_on_drift {
                tracing::warn!(
                    server = server_name,
                    tool = tool.name.as_str(),
                    previous = previous.as_str(),
                    current = current.as_str(),
                    "MCP tool trust baseline drift detected; skipping tool (fail closed)"
                );
                continue;
            }
        }
        trusted_tools.push((tool, trust_decision));
    }
    Ok(trusted_tools)
}
fn log_mcp_tool_trust_decision(server_name: &str, tool_name: &str, decision: McpToolTrustDecision) {
    match decision {
        McpToolTrustDecision::BaselineCreated { hash } => {
            tracing::info!(
                server = server_name,
                tool = tool_name,
                hash = hash.as_str(),
                "Created MCP tool trust baseline"
            );
        }
        McpToolTrustDecision::Unchanged => {}
        McpToolTrustDecision::Drift { previous, current } => {
            tracing::warn!(
                server = server_name,
                tool = tool_name,
                previous = previous.as_str(),
                current = current.as_str(),
                "MCP tool description/schema trust baseline drift detected"
            );
        }
    }
}

fn ensure_supported_transport(config: &McpServerConfig) -> Result<(), McpError> {
    match config.transport.as_str() {
        "websocket" => Err(McpError::unsupported_transport(
            "websocket",
            "WebSocket MCP transport is not controlled by this runtime and fails closed",
        )),
        "stdio" => {
            let command = config.command.as_deref().unwrap_or_default();
            if matches!(command, "ssh" | "plink" | "nc" | "ncat") {
                return Err(McpError::unsupported_transport(
                    "stdio",
                    "Remote stdio MCP transport is not controlled by this runtime and fails closed",
                ));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_mcp_tool_schema(server_name: &str, tool: &McpTool) -> Result<(), McpError> {
    if tool.input_schema.is_null() {
        return Ok(());
    }
    let Some(schema) = tool.input_schema.as_object() else {
        return Err(McpError::schema(format!(
            "MCP server '{server_name}' returned non-object schema for tool '{}'",
            tool.name
        )));
    };
    if schema
        .get("properties")
        .is_some_and(|properties| !properties.is_object())
    {
        return Err(McpError::schema(format!(
            "MCP server '{server_name}' returned invalid properties schema for tool '{}'",
            tool.name
        )));
    }
    if schema
        .get("required")
        .is_some_and(|required| !required.is_array())
    {
        return Err(McpError::schema(format!(
            "MCP server '{server_name}' returned invalid required schema for tool '{}'",
            tool.name
        )));
    }
    Ok(())
}

fn refresh_status_auth(source: &MergedMcpServerSource, status: &mut McpServerRuntimeStatus) {
    status.auth =
        McpAuthStatus::from_server_config(&source.selected.server_id, &source.selected.config);
    if status.enabled
        && status.auth_blocks_tools()
        && !matches!(
            status.state,
            super::runtime_status::McpRuntimeState::AuthRequired
        )
    {
        status.state = super::runtime_status::McpRuntimeState::AuthRequired;
    } else if status.enabled
        && !status.auth_blocks_tools()
        && matches!(
            status.state,
            super::runtime_status::McpRuntimeState::AuthRequired
        )
    {
        status.state = super::runtime_status::McpRuntimeState::Disconnected;
    }
}

#[cfg(test)]
#[path = "registry_runtime_tests.rs"]
mod registry_runtime_tests;
