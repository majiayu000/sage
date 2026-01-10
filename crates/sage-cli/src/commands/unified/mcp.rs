//! MCP registry building for the unified command

use sage_core::config::Config;
use sage_core::error::SageResult;
use sage_core::mcp::registry::McpRegistry;
use sage_core::mcp::transport::TransportConfig;

/// Build MCP registry from configuration
pub async fn build_mcp_registry_from_config(config: &Config) -> SageResult<McpRegistry> {
    let registry = McpRegistry::new();

    for (name, server_config) in &config.mcp.servers {
        if !server_config.enabled {
            continue;
        }

        let transport_config = match server_config.transport.as_str() {
            "stdio" => {
                let command = server_config.command.clone().unwrap_or_default();
                let args = server_config.args.clone();
                let env = server_config.env.clone();
                TransportConfig::Stdio { command, args, env }
            }
            "http" | "https" | "sse" => {
                let base_url = server_config.url.clone().unwrap_or_default();
                let headers = server_config.headers.clone();
                TransportConfig::Http { base_url, headers }
            }
            _ => {
                tracing::warn!("Unsupported MCP transport type: {}", server_config.transport);
                continue;
            }
        };

        match registry.register_server(name, transport_config).await {
            Ok(server_info) => {
                tracing::info!(
                    "Connected to MCP server '{}': {} v{}",
                    name,
                    server_info.name,
                    server_info.version
                );
            }
            Err(e) => {
                tracing::error!("Failed to connect to MCP server '{}': {}", name, e);
            }
        }
    }

    Ok(registry)
}
