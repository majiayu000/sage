//! Utility functions for MCP server discovery

use crate::config::McpServerConfig;
use crate::mcp::error::McpError;
use crate::mcp::transport::TransportConfig;

/// Convert McpServerConfig to TransportConfig
pub fn server_config_to_transport(config: &McpServerConfig) -> Result<TransportConfig, McpError> {
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
