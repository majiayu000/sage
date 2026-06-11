//! MCP registry builder from configuration

use super::discovery::utils::server_config_to_transport;
use super::registry::McpRegistry;
use super::transport::TransportConfig;
use crate::config::Config;
use crate::error::SageResult;

/// Build MCP registry from configuration
pub async fn build_mcp_registry_from_config(config: &Config) -> SageResult<McpRegistry> {
    let registry = McpRegistry::new();

    for (name, server_config) in config.mcp.enabled_servers() {
        let transport_config = server_config_to_transport(server_config)?;
        if matches!(transport_config, TransportConfig::WebSocket { .. }) {
            tracing::warn!(
                "Skipping MCP server '{}': WebSocket transport is not yet implemented",
                name
            );
            continue;
        }

        let server_info = registry.register_server(name, transport_config).await?;
        tracing::info!(
            "Connected to MCP server '{}': {} v{}",
            name,
            server_info.name,
            server_info.version
        );
    }

    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::McpServerConfig;

    fn config_with_server(name: &str, server: McpServerConfig) -> Config {
        let mut config = Config::default();
        config.mcp.enabled = true;
        config.mcp.servers.insert(name.to_string(), server);
        config
    }

    #[tokio::test]
    async fn test_enabled_stdio_server_without_command_fails() {
        let mut server = McpServerConfig::stdio("ignored", Vec::new());
        server.command = None;
        let config = config_with_server("broken", server);

        let result = build_mcp_registry_from_config(&config).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_enabled_http_server_without_url_fails() {
        let mut server = McpServerConfig::http("http://localhost:9999");
        server.url = None;
        let config = config_with_server("broken", server);

        let result = build_mcp_registry_from_config(&config).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_enabled_sse_server_without_url_fails() {
        let mut server = McpServerConfig::http("http://localhost:9999");
        server.transport = "sse".to_string();
        server.url = None;
        let config = config_with_server("broken", server);

        let result = build_mcp_registry_from_config(&config).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_enabled_unknown_transport_fails() {
        let mut server = McpServerConfig::http("http://localhost:9999");
        server.transport = "invalid".to_string();
        let config = config_with_server("broken", server);

        let result = build_mcp_registry_from_config(&config).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_disabled_invalid_server_is_ignored() -> SageResult<()> {
        let mut server = McpServerConfig::http("http://localhost:9999");
        server.transport = "invalid".to_string();
        server.enabled = false;
        let config = config_with_server("disabled", server);

        let registry = build_mcp_registry_from_config(&config).await?;

        assert!(registry.server_names().is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_enabled_websocket_server_is_skipped_until_supported() -> SageResult<()> {
        let config =
            config_with_server("future", McpServerConfig::websocket("ws://localhost:9000"));

        let registry = build_mcp_registry_from_config(&config).await?;

        assert!(registry.server_names().is_empty());
        Ok(())
    }
}
