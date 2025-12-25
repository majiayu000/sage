//! Server health checking and monitoring

use super::types::{ServerHealth, ServerStatus};
use crate::mcp::error::McpError;
use crate::mcp::registry::McpRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Health tracker for MCP servers
pub(super) struct HealthTracker {
    health: RwLock<HashMap<String, ServerHealth>>,
}

impl HealthTracker {
    /// Create a new health tracker
    pub fn new() -> Self {
        Self {
            health: RwLock::new(HashMap::new()),
        }
    }

    /// Get health status for all servers
    pub async fn health_status(&self) -> Vec<ServerHealth> {
        self.health.read().await.values().cloned().collect()
    }

    /// Get health status for a specific server
    pub async fn server_health(&self, name: &str) -> Option<ServerHealth> {
        self.health.read().await.get(name).cloned()
    }

    /// Update health status for a server
    pub async fn update_health(&self, name: &str, status: ServerStatus) {
        let mut health = self.health.write().await;
        health
            .entry(name.to_string())
            .and_modify(|h| h.status = status.clone())
            .or_insert_with(|| ServerHealth {
                name: name.to_string(),
                status,
                last_ping: None,
                consecutive_failures: 0,
                successful_requests: 0,
                failed_requests: 0,
            });
    }

    /// Check health of a server by pinging it
    pub async fn check_health(
        &self,
        name: &str,
        registry: &Arc<McpRegistry>,
    ) -> Result<bool, McpError> {
        if let Some(client) = registry.get_client(name) {
            match client.ping().await {
                Ok(_) => {
                    let mut health = self.health.write().await;
                    if let Some(h) = health.get_mut(name) {
                        h.last_ping = Some(std::time::Instant::now());
                        h.consecutive_failures = 0;
                        h.successful_requests += 1;
                    }
                    Ok(true)
                }
                Err(e) => {
                    let mut health = self.health.write().await;
                    if let Some(h) = health.get_mut(name) {
                        h.consecutive_failures += 1;
                        h.failed_requests += 1;
                    }
                    Err(e)
                }
            }
        } else {
            Ok(false)
        }
    }

    /// Check health of all servers
    pub async fn check_all_health(
        &self,
        server_names: &[String],
        registry: &Arc<McpRegistry>,
    ) -> HashMap<String, bool> {
        let mut results = HashMap::new();

        for name in server_names {
            let healthy = self.check_health(name, registry).await.unwrap_or(false);
            results.insert(name.clone(), healthy);
        }

        results
    }

    /// Update all health statuses to disconnected
    pub async fn mark_all_disconnected(&self) {
        let mut health = self.health.write().await;
        for (_, h) in health.iter_mut() {
            h.status = ServerStatus::Disconnected;
        }
    }
}
