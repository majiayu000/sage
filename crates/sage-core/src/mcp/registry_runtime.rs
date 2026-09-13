//! Runtime source/status extensions for the MCP registry.

use super::auth_status::McpAuthorizationPrompt;
use super::client::McpClient;
use super::deferred_tools::{McpDeferredTool, McpDeferredToolIndex, namespaced_tool_prefix};
use super::discovery::utils::server_config_to_transport;
use super::error::McpError;
#[cfg(test)]
use super::registry::global_tool_trust_lock;
use super::registry::{McpRegistry, ToolRoute};
use super::registry_adapter::McpToolAdapter;
use super::registry_runtime_helpers::{
    ensure_supported_transport, log_mcp_tool_trust_decision, refresh_status_auth,
    trusted_mcp_tools_for_server,
};
use super::runtime_status::{
    McpRuntimeAction, McpRuntimeActionResult, McpServerRuntimeStatus, McpToolDiscoveryState,
};
use super::source::{McpSourceSet, MergedMcpServerSource};
use super::tool_trust::McpToolTrustStore;
use crate::config::McpAuthKind;
use std::sync::Arc;

#[cfg(test)]
use super::tool_trust::McpToolTrustDecision;
#[cfg(test)]
use super::types::McpTool;

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
        // Serialize per-server refreshes so a stale list_tools response cannot
        // overwrite a newer drift rejection / allowlist update.
        let refresh_lock = self.capability_refresh_lock(name);
        let _refresh_guard = refresh_lock.lock().await;
        self.refresh_server_capabilities_locked(name, client).await
    }

    /// Refresh capabilities while the caller already holds `capability_refresh_lock`.
    pub(crate) async fn refresh_server_capabilities_locked(
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
        // Preserve transport/timeout/connection variants; only trust-store and
        // schema validation failures become Schema errors below.
        // Capture listChanged generation before tools/list so a notification
        // during fetch or the subsequent blocking trust check cannot let this
        // refresh republish a stale allowlist.
        let list_generation = client.list_changed_generation();
        let tools = client.list_tools_uncached().await.map_err(|error| {
            error.with_context(format!("while discovering tools for MCP server '{name}'"))
        })?;
        if client.list_changed_generation() != list_generation {
            return Err(McpError::schema(format!(
                "MCP server '{name}' tools/listChanged during capability refresh; retry required"
            )));
        }

        // Serialize the process-wide trust baseline so concurrent first
        // baselines across servers *and* separate McpRegistry instances cannot
        // overwrite each other and later treat a lost entry as BaselineCreated.
        // The file lock additionally serializes concurrent Sage processes that
        // share the same home-directory trust file.
        let _trust_guard = self.tool_trust_lock.lock().await;
        // Run the full load / check / durable-save transaction on the blocking
        // pool so contended flocks, schema hashing, and sync_all cannot stall
        // the Tokio worker while process-wide and inter-process locks are held.
        let server_name = name.to_string();
        let warn_on_drift = self.warn_on_tool_trust_drift();
        let trusted_tools = tokio::task::spawn_blocking(move || {
            let (_file_lock, mut trust_store) = McpToolTrustStore::load_default_locked()?;
            let trusted =
                trusted_mcp_tools_for_server(&server_name, tools, &mut trust_store, warn_on_drift)?;
            trust_store.save_if_dirty()?;
            drop(_file_lock);
            Ok::<_, McpError>(trusted)
        })
        .await
        .map_err(|error| {
            McpError::schema(format!(
                "Failed to refresh MCP tool trust on blocking pool: {error}"
            ))
        })??;
        drop(_trust_guard);

        // A displaced same-name registration may have replaced this client while
        // we were awaiting list_tools; do not publish routes for a stale Arc.
        if !self.is_current_client(name, client) {
            client.replace_trusted_tools(Vec::new()).await;
            return Err(McpError::connection(format!(
                "MCP server '{name}' was replaced during capability refresh"
            )));
        }

        // listChanged after tools/list (e.g. during the blocking trust check)
        // cleared the allowlist; refuse to republish the now-stale response.
        if client.list_changed_generation() != list_generation {
            return Err(McpError::schema(format!(
                "MCP server '{name}' tools/listChanged during capability refresh; retry required"
            )));
        }

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
        self.clear_runtime_error_after_successful_refresh(name);

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

    fn is_current_client(&self, name: &str, client: &Arc<McpClient>) -> bool {
        self.clients
            .get(name)
            .is_some_and(|entry| Arc::ptr_eq(entry.value(), client))
    }

    fn clear_runtime_error_after_successful_refresh(&self, name: &str) {
        let Some(entry) = self.statuses.get(name) else {
            return;
        };
        let mut status = entry.value().clone();
        drop(entry);
        status.mark_connected();
        self.store_status(status);
    }

    async fn clear_server_trusted_tools(
        &self,
        name: &str,
        client: &Arc<McpClient>,
        error: &McpError,
    ) {
        // Always revoke this client's allowlist; only mutate shared routes/status
        // when this Arc is still (or was already) the live mapping for `name`.
        client.replace_trusted_tools(Vec::new()).await;
        if !self.is_current_client(name, client) && self.clients.contains_key(name) {
            return;
        }

        self.tool_mapping
            .retain(|_, route| route.server_name != name);

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

#[cfg(test)]
#[path = "registry_runtime_tests.rs"]
mod registry_runtime_tests;
