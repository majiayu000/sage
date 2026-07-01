//! MCP registry builder from configuration

use super::registry::McpRegistry;
use super::source::{direct_config_sources, merge_mcp_sources, package_sources};
use crate::config::Config;
use crate::error::{SageError, SageResult};
use crate::plugins::{ExtensionPackageManager, PackageMcpServerRegistration};

/// Build MCP registry from configuration
pub async fn build_mcp_registry_from_config(config: &Config) -> SageResult<McpRegistry> {
    let package_registrations = if config.mcp.enabled {
        default_enabled_package_mcp_servers()?
    } else {
        Vec::new()
    };
    build_mcp_registry_from_config_and_packages(config, package_registrations.iter()).await
}

/// Build MCP registry from direct config and package-sourced MCP declarations.
pub async fn build_mcp_registry_from_config_and_packages<'a>(
    config: &Config,
    package_registrations: impl IntoIterator<Item = &'a PackageMcpServerRegistration>,
) -> SageResult<McpRegistry> {
    let registry = McpRegistry::new();

    let mut sources = direct_config_sources(&config.mcp);
    if config.mcp.enabled {
        sources.extend(package_sources(package_registrations));
    }
    let source_set = merge_mcp_sources(sources)
        .map_err(|err| SageError::config(format!("Failed to merge MCP server sources: {err}")))?;
    registry.apply_source_set(source_set);

    if !config.mcp.enabled || !config.mcp.auto_connect {
        return Ok(registry);
    }

    for status in registry.runtime_statuses() {
        if !status.enabled {
            continue;
        }
        match registry.connect_configured_server(&status.server_id).await {
            Ok(result) => {
                tracing::info!(
                    "Connected to MCP server '{}': {:?}",
                    result.status.server_id,
                    result.status.state
                );
            }
            Err(err) => {
                tracing::warn!("MCP server '{}' is unavailable: {}", status.server_id, err);
            }
        }
    }

    Ok(registry)
}

fn default_enabled_package_mcp_servers() -> SageResult<Vec<PackageMcpServerRegistration>> {
    let manager = ExtensionPackageManager::default_user_manager().map_err(|err| {
        SageError::config(format!("Failed to load extension package store: {err}"))
    })?;
    manager.enabled_mcp_servers().map_err(|err| {
        SageError::config(format!(
            "Failed to load enabled extension package MCP servers: {err}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::McpServerConfig;
    use crate::mcp::{McpFailureKind, McpRuntimeState, McpSourceKind};
    use std::path::PathBuf;

    fn config_with_server(name: &str, server: McpServerConfig) -> Config {
        let mut config = Config::default();
        config.mcp.enabled = true;
        config.mcp.auto_connect = true;
        config.mcp.servers.insert(name.to_string(), server);
        config
    }

    #[tokio::test]
    async fn test_enabled_stdio_server_without_command_records_status() -> SageResult<()> {
        let mut server = McpServerConfig::stdio("ignored", Vec::new());
        server.command = None;
        let config = config_with_server("broken", server);

        let registry = build_mcp_registry_from_config(&config).await?;

        assert!(registry.server_names().is_empty());
        let status = registry.server_runtime_status("broken").expect("status");
        assert_eq!(status.state, McpRuntimeState::ConnectionError);
        assert_eq!(
            status.last_error.expect("error").kind,
            McpFailureKind::Config
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_enabled_http_server_without_url_records_status() -> SageResult<()> {
        let mut server = McpServerConfig::http("http://localhost:9999");
        server.url = None;
        let config = config_with_server("broken", server);

        let registry = build_mcp_registry_from_config(&config).await?;

        assert!(registry.server_names().is_empty());
        let status = registry.server_runtime_status("broken").expect("status");
        assert_eq!(status.state, McpRuntimeState::ConnectionError);
        assert_eq!(
            status.last_error.expect("error").kind,
            McpFailureKind::Config
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_enabled_sse_server_without_url_records_status() -> SageResult<()> {
        let mut server = McpServerConfig::http("http://localhost:9999");
        server.transport = "sse".to_string();
        server.url = None;
        let config = config_with_server("broken", server);

        let registry = build_mcp_registry_from_config(&config).await?;

        assert!(registry.server_names().is_empty());
        let status = registry.server_runtime_status("broken").expect("status");
        assert_eq!(status.state, McpRuntimeState::ConnectionError);
        assert_eq!(
            status.last_error.expect("error").kind,
            McpFailureKind::Config
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_enabled_unknown_transport_records_status() -> SageResult<()> {
        let mut server = McpServerConfig::http("http://localhost:9999");
        server.transport = "invalid".to_string();
        let config = config_with_server("broken", server);

        let registry = build_mcp_registry_from_config(&config).await?;

        assert!(registry.server_names().is_empty());
        let status = registry.server_runtime_status("broken").expect("status");
        assert_eq!(status.state, McpRuntimeState::ConnectionError);
        assert_eq!(
            status.last_error.expect("error").kind,
            McpFailureKind::Config
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_disabled_invalid_server_is_ignored() -> SageResult<()> {
        let mut server = McpServerConfig::http("http://localhost:9999");
        server.transport = "invalid".to_string();
        server.enabled = false;
        let config = config_with_server("disabled", server);

        let registry = build_mcp_registry_from_config(&config).await?;

        assert!(registry.server_names().is_empty());
        let status = registry.server_runtime_status("disabled").expect("status");
        assert_eq!(status.state, McpRuntimeState::Disabled);
        assert!(status.last_error.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_enabled_websocket_server_fails_closed_with_status() -> SageResult<()> {
        let config =
            config_with_server("future", McpServerConfig::websocket("ws://localhost:9000"));

        let registry = build_mcp_registry_from_config(&config).await?;

        assert!(registry.server_names().is_empty());
        let status = registry.server_runtime_status("future").expect("status");
        assert_eq!(status.state, McpRuntimeState::UnsupportedTransport);
        assert_eq!(
            status.last_error.expect("error").kind,
            McpFailureKind::UnsupportedTransport
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_enabled_runtime_connection_failure_records_status() -> SageResult<()> {
        let config = config_with_server(
            "offline",
            McpServerConfig::stdio("__sage_missing_mcp_binary__", Vec::new()),
        );

        let registry = build_mcp_registry_from_config(&config).await?;

        assert!(registry.server_names().is_empty());
        let status = registry.server_runtime_status("offline").expect("status");
        assert_eq!(status.state, McpRuntimeState::ConnectionError);
        assert_eq!(
            status.last_error.expect("error").kind,
            McpFailureKind::Connection
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_auto_connect_false_skips_runtime_connection() -> SageResult<()> {
        let mut config = config_with_server(
            "offline",
            McpServerConfig::stdio("__sage_missing_mcp_binary__", Vec::new()),
        );
        config.mcp.auto_connect = false;

        let registry = build_mcp_registry_from_config(&config).await?;

        assert!(registry.server_names().is_empty());
        let status = registry.server_runtime_status("offline").expect("status");
        assert_eq!(status.state, McpRuntimeState::Disconnected);
        Ok(())
    }

    #[tokio::test]
    async fn test_package_sources_feed_runtime_status_without_eager_connection() -> SageResult<()> {
        let mut config = Config::default();
        config.mcp.enabled = true;
        config.mcp.auto_connect = false;
        let registration = PackageMcpServerRegistration {
            package_id: "pkg.docs".to_string(),
            asset_id: "docs".to_string(),
            package_root: PathBuf::from("/tmp/pkg.docs"),
            config: McpServerConfig::stdio("docs-server", Vec::new()),
        };

        let registry =
            build_mcp_registry_from_config_and_packages(&config, [&registration]).await?;

        let status = registry
            .server_runtime_status("docs")
            .ok_or_else(|| SageError::config("missing docs runtime status"))?;
        assert_eq!(status.source.kind, McpSourceKind::Package);
        assert_eq!(status.state, McpRuntimeState::Disconnected);
        assert!(registry.deferred_tools().is_empty());
        Ok(())
    }
}
