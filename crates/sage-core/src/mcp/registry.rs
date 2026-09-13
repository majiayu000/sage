//! MCP registry for managing multiple MCP servers
//!
//! Provides centralized management of MCP servers and their tools.

use super::client::McpClient;
use super::deferred_tools::McpDeferredToolIndex;
use super::error::McpError;
use super::registry_tool_errors::tool_unavailable_error;
use super::runtime_status::{McpServerRuntimeStatus, McpToolDiscoveryState};
use super::source::MergedMcpServerSource;
use super::transport::{HttpTransport, HttpTransportConfig, StdioTransport, TransportConfig};
use super::types::{McpPrompt, McpResource, McpServerInfo, McpTool};
use crate::tools::base::Tool;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

pub use super::registry_adapter::McpToolAdapter;

#[derive(Debug, Clone)]
pub(crate) struct ToolRoute {
    pub(crate) server_name: String,
    pub(crate) remote_name: String,
}

/// Process-wide lock for the shared `mcp_tool_trust.json` load/check/save
/// transaction. Separate `McpRegistry` instances must share this lock so
/// concurrent first baselines cannot overwrite each other. Cross-process
/// coordination uses `ToolTrustFileLock` inside the trust-store load path.
pub(crate) fn global_tool_trust_lock() -> Arc<tokio::sync::Mutex<()>> {
    static LOCK: OnceLock<Arc<tokio::sync::Mutex<()>>> = OnceLock::new();
    LOCK.get_or_init(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

/// Registry for managing MCP servers and their capabilities
pub struct McpRegistry {
    /// Configured MCP sources by server name
    pub(crate) sources: DashMap<String, MergedMcpServerSource>,
    /// Runtime status by server name
    pub(crate) statuses: DashMap<String, McpServerRuntimeStatus>,
    /// Connected MCP clients by name
    pub(crate) clients: DashMap<String, Arc<McpClient>>,
    /// Namespaced tool to route mapping
    pub(crate) tool_mapping: DashMap<String, ToolRoute>,
    /// Resource to client mapping
    pub(crate) resource_mapping: DashMap<String, String>,
    /// Prompt to client mapping
    pub(crate) prompt_mapping: DashMap<String, String>,
    /// Deferred searchable tool metadata
    pub(crate) deferred_tools: RwLock<McpDeferredToolIndex>,
    /// When true, trust baseline drift only warns and still registers tools.
    pub(crate) warn_on_tool_trust_drift: AtomicBool,
    /// Per-server locks that serialize capability refreshes and same-name registration.
    pub(crate) capability_refresh_locks: DashMap<String, Arc<tokio::sync::Mutex<()>>>,
    /// Shared process-wide lock for the mcp_tool_trust.json load/check/save transaction.
    pub(crate) tool_trust_lock: Arc<tokio::sync::Mutex<()>>,
}

impl McpRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            sources: DashMap::new(),
            statuses: DashMap::new(),
            clients: DashMap::new(),
            tool_mapping: DashMap::new(),
            resource_mapping: DashMap::new(),
            prompt_mapping: DashMap::new(),
            deferred_tools: RwLock::new(McpDeferredToolIndex::new()),
            warn_on_tool_trust_drift: AtomicBool::new(false),
            capability_refresh_locks: DashMap::new(),
            tool_trust_lock: global_tool_trust_lock(),
        }
    }

    /// Configure whether MCP tool trust baseline drift should warn instead of reject.
    ///
    /// When connected clients already exist and the policy changes, revalidates
    /// routes/allowlists immediately so warn→fail-closed cannot leave drifted
    /// tools callable until a later `all_tools()` call.
    pub async fn set_warn_on_tool_trust_drift(&self, enabled: bool) {
        let previous = self.warn_on_tool_trust_drift.load(Ordering::Relaxed);
        self.warn_on_tool_trust_drift
            .store(enabled, Ordering::Relaxed);
        if previous != enabled && !self.clients.is_empty() {
            let _ = self.all_tools().await;
        }
    }

    /// Return whether MCP tool trust baseline drift only warns (legacy behavior).
    pub fn warn_on_tool_trust_drift(&self) -> bool {
        self.warn_on_tool_trust_drift.load(Ordering::Relaxed)
    }

    pub(crate) fn capability_refresh_lock(&self, name: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.capability_refresh_locks
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    /// Register and connect to an MCP server
    pub async fn register_server(
        &self,
        name: impl Into<String>,
        config: TransportConfig,
    ) -> Result<McpServerInfo, McpError> {
        let name = name.into();

        // Create transport based on config
        let transport: Box<dyn super::transport::McpTransport> = match config {
            TransportConfig::Stdio { command, args, env } => {
                let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                Box::new(StdioTransport::spawn_with_env(&command, &args_refs, &env).await?)
            }
            TransportConfig::Http { base_url, headers } => {
                let http_config = HttpTransportConfig::new(&base_url);
                let http_config = headers
                    .into_iter()
                    .fold(http_config, |cfg, (k, v)| cfg.with_header(k, v));
                let mut transport = HttpTransport::new(http_config)?;
                transport.connect().await?;
                Box::new(transport)
            }
            TransportConfig::WebSocket { .. } => {
                return Err(McpError::transport(
                    "WebSocket transport not yet implemented",
                ));
            }
        };

        // Create and initialize client
        let client = Arc::new(McpClient::new(transport));
        let server_info = client.initialize().await?;

        // Activate an empty trusted-tool allowlist before publishing the client
        // so concurrent get_client()/call_tool cannot bypass trust filtering
        // while capability refresh is still in flight. Direct/standalone
        // clients leave the allowlist inactive.
        client.replace_trusted_tools(Vec::new()).await;

        // Serialize same-name revoke/insert/refresh so a failed older refresh
        // cannot remove a newer client or wipe its routes.
        let refresh_lock = self.capability_refresh_lock(&name);
        let _refresh_guard = refresh_lock.lock().await;

        // Same-name re-registration must revoke the displaced client so
        // previously issued adapters cannot keep calling a drifted schema.
        self.revoke_displaced_server_client(&name).await;

        // Store client
        self.clients.insert(name.clone(), client.clone());

        // Discover tools, resources, and prompts (lock already held).
        if let Err(error) = self
            .refresh_server_capabilities_locked(&name, &client)
            .await
        {
            // Only tear down registry state if we still own this slot.
            if self.remove_client_if_current(&name, &client) {
                self.tool_mapping
                    .retain(|_, route| route.server_name != name);
                self.resource_mapping.retain(|_, v| v != &name);
                self.prompt_mapping.retain(|_, v| v != &name);
                self.deferred_tools
                    .write()
                    .mark_server(name.clone(), McpToolDiscoveryState::SchemaError);
            }
            if let Err(close_error) = client.close().await {
                tracing::debug!(
                    "Failed to close MCP client '{}' after capability error: {}",
                    name,
                    close_error
                );
            }
            return Err(error);
        }

        Ok(server_info)
    }

    /// Remove `client` from `clients` only when it is still the live mapping.
    pub(crate) fn remove_client_if_current(&self, name: &str, client: &Arc<McpClient>) -> bool {
        let Some(entry) = self.clients.get(name) else {
            return false;
        };
        if !Arc::ptr_eq(entry.value(), client) {
            return false;
        }
        drop(entry);
        self.clients.remove(name).is_some()
    }

    async fn revoke_displaced_server_client(&self, name: &str) {
        let Some((_, old_client)) = self.clients.remove(name) else {
            return;
        };
        old_client.replace_trusted_tools(Vec::new()).await;
        self.tool_mapping
            .retain(|_, route| route.server_name != name);
        self.resource_mapping.retain(|_, v| v != name);
        self.prompt_mapping.retain(|_, v| v != name);
        if let Err(close_error) = old_client.close().await {
            tracing::debug!(
                "Failed to close displaced MCP client '{}': {}",
                name,
                close_error
            );
        }
    }

    /// Unregister and disconnect from an MCP server.
    ///
    /// Holds `capability_refresh_lock` so unregister cannot race a registration
    /// or `all_tools()` refresh that has already passed `is_current_client` and
    /// is about to publish routes for a client we are closing.
    pub async fn unregister_server(&self, name: &str) -> Result<(), McpError> {
        let refresh_lock = self.capability_refresh_lock(name);
        let _refresh_guard = refresh_lock.lock().await;

        if let Some((_, client)) = self.clients.remove(name) {
            // Remove mappings for this server
            self.tool_mapping
                .retain(|_, route| route.server_name != name);
            self.resource_mapping.retain(|_, v| v != name);
            self.prompt_mapping.retain(|_, v| v != name);

            // Close the client
            client.close().await?;
        }
        self.deferred_tools.write().mark_server_stale(name);
        Ok(())
    }

    /// Get a client by name
    pub fn get_client(&self, name: &str) -> Option<Arc<McpClient>> {
        self.clients.get(name).map(|c| c.clone())
    }

    /// Get all server names
    pub fn server_names(&self) -> Vec<String> {
        self.clients.iter().map(|e| e.key().clone()).collect()
    }

    /// Get all available tools across all servers
    pub async fn all_tools(&self) -> Vec<McpTool> {
        let mut tools = Vec::new();
        let clients = self
            .clients
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect::<Vec<_>>();
        for (server_name, client) in clients {
            if let Err(error) = self
                .refresh_server_capabilities(&server_name, &client)
                .await
            {
                tracing::warn!(
                    server = server_name.as_str(),
                    error = %error,
                    "Failed to refresh trusted MCP tools before listing; invalidated routes and trusted cache"
                );
                continue;
            }
            tools.extend(client.cached_tools().await);
        }
        tools
    }

    /// Get all available resources across all servers
    pub async fn all_resources(&self) -> Vec<McpResource> {
        let mut resources = Vec::new();
        for entry in self.clients.iter() {
            if let Ok(r) = entry.value().list_resources().await {
                resources.extend(r);
            }
        }
        resources
    }

    /// Get all available prompts across all servers
    pub async fn all_prompts(&self) -> Vec<McpPrompt> {
        let mut prompts = Vec::new();
        for entry in self.clients.iter() {
            if let Ok(p) = entry.value().list_prompts().await {
                prompts.extend(p);
            }
        }
        prompts
    }

    /// Call a tool by name
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<String, McpError> {
        let route = match self
            .tool_mapping
            .get(name)
            .map(|entry| entry.value().clone())
        {
            Some(route) => route,
            None => {
                if let Some(status) = self.status_for_tool_name(name) {
                    if let Some(error) = tool_unavailable_error(status) {
                        return Err(error);
                    }
                }
                return Err(McpError::tool_not_found(name.to_string()));
            }
        };

        let client = self
            .clients
            .get(&route.server_name)
            .map(|e| e.clone())
            .ok_or_else(|| {
                McpError::connection(format!("Server {} not found", route.server_name))
            })?;

        let result = client.call_tool(&route.remote_name, arguments).await?;

        // Convert result to string
        let text = result
            .content
            .iter()
            .filter_map(|c| match c {
                super::types::McpContent::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        if result.is_error {
            Err(McpError::server(-1, text))
        } else {
            Ok(text)
        }
    }

    /// Read a resource by URI
    pub async fn read_resource(&self, uri: &str) -> Result<String, McpError> {
        let server_name = self
            .resource_mapping
            .get(uri)
            .map(|e| e.value().clone())
            .ok_or_else(|| McpError::resource_not_found(uri.to_string()))?;

        let client = self
            .clients
            .get(&server_name)
            .map(|e| e.clone())
            .ok_or_else(|| McpError::connection(format!("Server {} not found", server_name)))?;

        let content = client.read_resource(uri).await?;

        content
            .text
            .ok_or_else(|| McpError::resource_not_found(uri.to_string()))
    }

    /// Convert MCP tools to Sage tools
    pub async fn as_tools(&self) -> Vec<Arc<dyn Tool>> {
        let mut tools = Vec::new();

        for client_entry in self.clients.iter() {
            let server_name = client_entry.key().clone();
            let client = client_entry.value().clone();
            for mcp_tool in client.cached_tools().await {
                let adapter = McpToolAdapter::new(mcp_tool, client.clone(), server_name.clone());
                tools.push(Arc::new(adapter) as Arc<dyn Tool>);
            }
        }

        tools
    }

    /// Close all connections
    pub async fn close_all(&self) -> Result<(), McpError> {
        for entry in self.clients.iter() {
            entry.value().close().await?;
        }
        self.clients.clear();
        self.tool_mapping.clear();
        self.resource_mapping.clear();
        self.prompt_mapping.clear();
        *self.deferred_tools.write() = McpDeferredToolIndex::new();
        Ok(())
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod registry_tests;
