//! Tests for MCP server discovery

#[cfg(test)]
mod tests {
    use crate::config::{McpConfig, McpServerConfig};
    use crate::mcp::discovery::manager::McpServerManager;
    use crate::mcp::discovery::scanner::get_standard_mcp_paths;
    use crate::mcp::discovery::types::DiscoverySource;
    use crate::mcp::transport::TransportConfig;

    fn server_config_to_transport(
        config: &McpServerConfig,
    ) -> Result<TransportConfig, crate::mcp::error::McpError> {
        use crate::mcp::error::McpError;

        match config.transport.as_str() {
            "stdio" => {
                let command = config
                    .command
                    .as_ref()
                    .ok_or_else(|| McpError::invalid_request("Stdio transport requires command"))?;

                Ok(TransportConfig::Stdio {
                    command: command.clone(),
                    args: config.args.clone(),
                    env: config.env.clone(),
                })
            }
            "http" => {
                let url = config
                    .url
                    .as_ref()
                    .ok_or_else(|| McpError::invalid_request("HTTP transport requires url"))?;

                Ok(TransportConfig::Http {
                    base_url: url.clone(),
                    headers: config.headers.clone(),
                })
            }
            "websocket" => {
                let url = config
                    .url
                    .as_ref()
                    .ok_or_else(|| McpError::invalid_request("WebSocket transport requires url"))?;

                Ok(TransportConfig::WebSocket { url: url.clone() })
            }
            other => Err(McpError::invalid_request(format!(
                "Unknown transport type: {}",
                other
            ))),
        }
    }

    #[test]
    fn test_server_config_to_transport_stdio() {
        let config = McpServerConfig::stdio("echo", vec!["hello".to_string()]);
        let transport = server_config_to_transport(&config).unwrap();

        assert!(matches!(transport, TransportConfig::Stdio { .. }));
    }

    #[test]
    fn test_server_config_to_transport_http() {
        let config = McpServerConfig::http("http://localhost:8080");
        let transport = server_config_to_transport(&config).unwrap();

        assert!(matches!(transport, TransportConfig::Http { .. }));
    }

    #[test]
    fn test_server_config_to_transport_websocket() {
        let config = McpServerConfig::websocket("ws://localhost:8080");
        let transport = server_config_to_transport(&config).unwrap();

        assert!(matches!(transport, TransportConfig::WebSocket { .. }));
    }

    #[test]
    fn test_server_config_to_transport_invalid() {
        let mut config = McpServerConfig::http("http://localhost:8080");
        config.transport = "invalid".to_string();

        let result = server_config_to_transport(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_standard_paths_not_empty() {
        let paths = get_standard_mcp_paths();
        assert!(!paths.is_empty());
    }

    #[test]
    fn test_manager_creation() {
        let manager = McpServerManager::new();
        assert!(manager.connected_servers().is_empty());
    }

    fn config_with_server(name: &str, server: McpServerConfig) -> McpConfig {
        let mut config = McpConfig::default();
        config.enabled = true;
        config.auto_connect = true;
        config.servers.insert(name.to_string(), server);
        config
    }

    #[tokio::test]
    async fn test_manager_discover_from_config_fails_on_enabled_invalid_server() {
        let manager = McpServerManager::new();
        let mut server = McpServerConfig::stdio("ignored", Vec::new());
        server.command = None;

        let result = manager
            .discover_from_config(config_with_server("broken", server))
            .await;

        assert!(result.is_err());
        assert!(manager.connected_servers().is_empty());
    }

    #[tokio::test]
    async fn test_manager_discover_from_config_respects_auto_connect_false()
    -> Result<(), crate::mcp::error::McpError> {
        let manager = McpServerManager::new();
        let mut config = config_with_server(
            "offline",
            McpServerConfig::stdio("__sage_missing_mcp_binary__", Vec::new()),
        );
        config.auto_connect = false;

        let connected = manager.discover_from_config(config).await?;

        assert!(connected.is_empty());
        assert!(manager.connected_servers().is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_manager_discover_config_source_fails_on_enabled_invalid_server() {
        let manager = McpServerManager::new();
        let mut server = McpServerConfig::http("http://localhost:9999");
        server.url = None;
        let source = DiscoverySource::Config(config_with_server("broken", server));

        let result = manager.discover(vec![source]).await;

        assert!(result.is_err());
        assert!(manager.connected_servers().is_empty());
    }

    #[tokio::test]
    async fn test_manager_discover_applies_warn_on_tool_trust_drift_from_config_source() {
        let manager = McpServerManager::new();
        let mut config = config_with_server(
            "offline",
            McpServerConfig::stdio("__sage_missing_mcp_binary__", Vec::new()),
        );
        config.auto_connect = false;
        config.warn_on_tool_trust_drift = true;
        config.warn_on_tool_trust_drift_set = true;

        let connected = manager
            .discover(vec![DiscoverySource::Config(config)])
            .await
            .expect("discover should succeed when auto_connect is false");

        assert!(connected.is_empty());
        assert!(manager.registry().warn_on_tool_trust_drift());
    }

    #[tokio::test]
    async fn test_manager_discover_merges_trust_policy_before_connecting() {
        let manager = McpServerManager::new();

        let mut warn_config = config_with_server(
            "warn-src",
            McpServerConfig::stdio("__sage_missing_mcp_binary__", Vec::new()),
        );
        warn_config.auto_connect = false;
        warn_config.warn_on_tool_trust_drift = true;
        warn_config.warn_on_tool_trust_drift_set = true;

        let mut fail_closed = config_with_server(
            "reject-src",
            McpServerConfig::stdio("__sage_missing_mcp_binary__", Vec::new()),
        );
        fail_closed.auto_connect = false;
        fail_closed.warn_on_tool_trust_drift = false;
        fail_closed.warn_on_tool_trust_drift_set = true;

        let connected = manager
            .discover(vec![
                DiscoverySource::Config(warn_config),
                DiscoverySource::Config(fail_closed),
            ])
            .await
            .expect("discover should succeed when auto_connect is false");

        assert!(connected.is_empty());
        assert!(
            !manager.registry().warn_on_tool_trust_drift(),
            "later explicit fail-closed policy must win before any connect"
        );
    }

    #[tokio::test]
    async fn test_manager_discover_reentry_tightens_policy_and_revalidates() {
        let manager = McpServerManager::new();

        let mut warn_config = config_with_server(
            "warn-only",
            McpServerConfig::stdio("__sage_missing_mcp_binary__", Vec::new()),
        );
        warn_config.auto_connect = false;
        warn_config.warn_on_tool_trust_drift = true;
        warn_config.warn_on_tool_trust_drift_set = true;

        manager
            .discover(vec![DiscoverySource::Config(warn_config)])
            .await
            .expect("first discover should succeed");
        assert!(manager.registry().warn_on_tool_trust_drift());

        let mut fail_closed = config_with_server(
            "reject-later",
            McpServerConfig::stdio("__sage_missing_mcp_binary__", Vec::new()),
        );
        fail_closed.auto_connect = false;
        fail_closed.warn_on_tool_trust_drift = false;
        fail_closed.warn_on_tool_trust_drift_set = true;

        manager
            .discover(vec![DiscoverySource::Config(fail_closed)])
            .await
            .expect("second discover should succeed");
        assert!(
            !manager.registry().warn_on_tool_trust_drift(),
            "re-entry with explicit fail-closed must tighten the registry policy"
        );
    }

    #[tokio::test]
    async fn test_manager_discover_reentry_enabling_warn_mode_flips_policy() {
        let manager = McpServerManager::new();

        let mut fail_closed = config_with_server(
            "reject-first",
            McpServerConfig::stdio("__sage_missing_mcp_binary__", Vec::new()),
        );
        fail_closed.auto_connect = false;
        fail_closed.warn_on_tool_trust_drift = false;
        fail_closed.warn_on_tool_trust_drift_set = true;

        manager
            .discover(vec![DiscoverySource::Config(fail_closed)])
            .await
            .expect("first discover should succeed");
        assert!(!manager.registry().warn_on_tool_trust_drift());

        let mut warn_config = config_with_server(
            "warn-later",
            McpServerConfig::stdio("__sage_missing_mcp_binary__", Vec::new()),
        );
        warn_config.auto_connect = false;
        warn_config.warn_on_tool_trust_drift = true;
        // Programmatic opt-in without presence flag must still enable warn mode.
        warn_config.warn_on_tool_trust_drift_set = false;

        manager
            .discover(vec![DiscoverySource::Config(warn_config)])
            .await
            .expect("second discover should succeed");
        assert!(
            manager.registry().warn_on_tool_trust_drift(),
            "fail-closed→warn re-entry must flip the registry trust policy"
        );
    }

    #[tokio::test]
    async fn test_manager_discover_all_source_failure_preserves_warn_policy() {
        let manager = McpServerManager::new();
        manager
            .registry()
            .set_warn_on_tool_trust_drift(true)
            .await;
        assert!(manager.registry().warn_on_tool_trust_drift());

        let connected = manager
            .discover(vec![
                DiscoverySource::Environment(
                    "__SAGE_MISSING_MCP_DISCOVERY_ENV_FOR_TRUST_POLICY__".to_string(),
                ),
                DiscoverySource::File(std::path::PathBuf::from(
                    "/tmp/__sage_missing_mcp_config_for_trust_policy__.json",
                )),
            ])
            .await
            .expect("discover should succeed when source errors are only logged");

        assert!(connected.is_empty());
        assert!(
            manager.registry().warn_on_tool_trust_drift(),
            "all-source failure must not reset an existing warn-mode policy"
        );
    }
}
