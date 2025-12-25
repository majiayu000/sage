//! Tests for MCP server discovery

#[cfg(test)]
mod tests {
    use crate::config::McpServerConfig;
    use crate::mcp::discovery::manager::McpServerManager;
    use crate::mcp::discovery::scanner::get_standard_mcp_paths;
    use crate::mcp::transport::TransportConfig;

    fn server_config_to_transport(
        config: &McpServerConfig,
    ) -> Result<TransportConfig, crate::mcp::error::McpError> {
        use crate::mcp::error::McpError;

        match config.transport.as_str() {
            "stdio" => {
                let command = config.command.as_ref().ok_or_else(|| {
                    McpError::invalid_request("Stdio transport requires command")
                })?;

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
}
