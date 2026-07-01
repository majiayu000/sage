//! Tool error mapping for configured MCP runtime statuses.

use super::error::McpError;
use super::runtime_status::{McpRuntimeState, McpServerRuntimeStatus};

pub(super) fn tool_unavailable_error(status: McpServerRuntimeStatus) -> Option<McpError> {
    if status.auth_blocks_tools() {
        if let Some(prompt) = status.auth.prompt {
            return Some(McpError::auth_required(status.server_id, prompt));
        }
        return Some(McpError::connection(format!(
            "MCP server '{}' requires authorization before tools can run",
            status.server_id
        )));
    }

    match status.state {
        McpRuntimeState::Disabled => Some(McpError::disabled(status.server_id)),
        McpRuntimeState::Disconnected | McpRuntimeState::Connecting => Some(McpError::connection(
            format!("MCP server '{}' is not connected", status.server_id),
        )),
        McpRuntimeState::ConnectionError => Some(McpError::connection(status_failure_message(
            &status,
            "MCP server connection failed",
        ))),
        McpRuntimeState::SchemaError => Some(McpError::schema(status_failure_message(
            &status,
            "MCP server schema discovery failed",
        ))),
        McpRuntimeState::UnsupportedTransport => Some(McpError::unsupported_transport(
            "configured",
            status_failure_message(&status, "MCP server transport is unsupported"),
        )),
        McpRuntimeState::Connected | McpRuntimeState::AuthRequired => None,
    }
}

fn status_failure_message(status: &McpServerRuntimeStatus, fallback: &str) -> String {
    status
        .last_error
        .as_ref()
        .map(|failure| failure.message.clone())
        .unwrap_or_else(|| format!("{fallback}: {}", status.server_id))
}
