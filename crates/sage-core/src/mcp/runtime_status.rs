//! Structured MCP runtime status.

use super::auth_status::{McpAuthState, McpAuthStatus};
use super::error::McpError;
use super::source::{McpSourceMetadata, MergedMcpServerSource};
use crate::error::UnifiedError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Runtime lifecycle state for one MCP server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpRuntimeState {
    /// Source is present but disabled.
    Disabled,
    /// Source is enabled but not connected.
    Disconnected,
    /// A connect attempt is in progress.
    Connecting,
    /// Server is connected and initialized.
    Connected,
    /// Authentication must be completed before tools may run.
    AuthRequired,
    /// Connection or initialization failed.
    ConnectionError,
    /// Tool schema discovery failed.
    SchemaError,
    /// Transport is unsupported or not controlled by Sage.
    UnsupportedTransport,
}

/// Deferred tool discovery state for a server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpToolDiscoveryState {
    /// No discovery attempt has been made.
    NotStarted,
    /// Tools are intentionally deferred until connect or refresh.
    Deferred,
    /// Cached tools are fresh.
    Fresh,
    /// Cached tools may be stale.
    Stale,
    /// Tool schema discovery failed.
    SchemaError,
    /// Tools cannot be discovered while the source is unavailable.
    Unavailable,
}

/// Machine-readable failure kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpFailureKind {
    /// Connection failed.
    Connection,
    /// Auth is missing or failed.
    Auth,
    /// Schema discovery or validation failed.
    Schema,
    /// Source is disabled.
    Disabled,
    /// Source merge produced duplicate declarations.
    DuplicateSource,
    /// Transport is not supported or not controlled.
    UnsupportedTransport,
    /// Config is invalid.
    Config,
}

/// Structured MCP failure for status surfaces and tool errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpStructuredFailure {
    /// Failure kind.
    pub kind: McpFailureKind,
    /// Stable error code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Whether a retry or user action can recover.
    pub recoverable: bool,
}

/// Runtime status for one MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerRuntimeStatus {
    /// Runtime server id.
    pub server_id: String,
    /// Selected source metadata.
    pub source: McpSourceMetadata,
    /// Lower-precedence sources overridden by the selected declaration.
    pub overridden_sources: Vec<McpSourceMetadata>,
    /// Whether this source is enabled.
    pub enabled: bool,
    /// Runtime lifecycle state.
    pub state: McpRuntimeState,
    /// Authentication status.
    pub auth: McpAuthStatus,
    /// Last connect attempt.
    pub last_connect_attempt: Option<DateTime<Utc>>,
    /// Last structured failure.
    pub last_error: Option<McpStructuredFailure>,
    /// Tool discovery state.
    pub tool_discovery_state: McpToolDiscoveryState,
}

/// Runtime action requested for an MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpRuntimeAction {
    /// Connect an enabled source.
    Connect,
    /// Disconnect an active server.
    Disconnect,
    /// Retry a failed or disconnected server.
    Retry,
}

/// Result of a runtime action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRuntimeActionResult {
    /// Requested action.
    pub action: McpRuntimeAction,
    /// Updated server status.
    pub status: McpServerRuntimeStatus,
}

impl McpStructuredFailure {
    /// Build a structured failure from an MCP error.
    pub fn from_error(err: &McpError) -> Self {
        match err {
            McpError::AuthRequired { server, .. } => Self {
                kind: McpFailureKind::Auth,
                code: "MCP_AUTH_REQUIRED".to_string(),
                message: format!("MCP server '{server}' requires authorization"),
                recoverable: true,
            },
            McpError::Disabled { server, .. } => Self {
                kind: McpFailureKind::Disabled,
                code: "MCP_DISABLED".to_string(),
                message: format!("MCP server '{server}' is disabled"),
                recoverable: true,
            },
            McpError::Schema { message, .. } => Self {
                kind: McpFailureKind::Schema,
                code: "MCP_SCHEMA".to_string(),
                message: message.clone(),
                recoverable: true,
            },
            McpError::DuplicateSource {
                server, message, ..
            } => Self {
                kind: McpFailureKind::DuplicateSource,
                code: "MCP_DUPLICATE_SOURCE".to_string(),
                message: format!("MCP server '{server}' has duplicate sources: {message}"),
                recoverable: true,
            },
            McpError::UnsupportedTransport { message, .. } => Self {
                kind: McpFailureKind::UnsupportedTransport,
                code: "MCP_UNSUPPORTED_TRANSPORT".to_string(),
                message: message.clone(),
                recoverable: false,
            },
            McpError::InvalidRequest { message, .. } => Self {
                kind: McpFailureKind::Config,
                code: "MCP_INVALID_REQUEST".to_string(),
                message: message.clone(),
                recoverable: true,
            },
            McpError::Connection { message, .. } => Self {
                kind: McpFailureKind::Connection,
                code: "MCP_CONNECTION".to_string(),
                message: message.clone(),
                recoverable: true,
            },
            other => Self {
                kind: McpFailureKind::Connection,
                code: other.error_code().to_string(),
                message: other.to_string(),
                recoverable: other.is_retryable(),
            },
        }
    }
}

impl McpServerRuntimeStatus {
    /// Initial status from a merged source.
    pub fn from_source(source: &MergedMcpServerSource) -> Self {
        let selected = &source.selected;
        let auth = McpAuthStatus::from_server_config(&selected.server_id, &selected.config);
        let state = if !selected.metadata.enabled {
            McpRuntimeState::Disabled
        } else if auth.blocks_tools() {
            McpRuntimeState::AuthRequired
        } else {
            McpRuntimeState::Disconnected
        };
        let tool_discovery_state = match state {
            McpRuntimeState::Disabled => McpToolDiscoveryState::Unavailable,
            McpRuntimeState::AuthRequired => McpToolDiscoveryState::Deferred,
            _ => McpToolDiscoveryState::Deferred,
        };

        Self {
            server_id: selected.server_id.clone(),
            source: selected.metadata.clone(),
            overridden_sources: source.overridden_sources.clone(),
            enabled: selected.metadata.enabled,
            state,
            auth,
            last_connect_attempt: None,
            last_error: None,
            tool_discovery_state,
        }
    }

    /// Whether auth currently blocks tool execution.
    pub fn auth_blocks_tools(&self) -> bool {
        matches!(
            self.auth.state,
            McpAuthState::AuthRequired | McpAuthState::Pending | McpAuthState::Failed
        )
    }

    /// Mark a connect attempt.
    pub fn mark_connecting(&mut self) {
        self.state = McpRuntimeState::Connecting;
        self.last_connect_attempt = Some(Utc::now());
        self.last_error = None;
    }

    /// Mark successful connection.
    pub fn mark_connected(&mut self) {
        self.state = McpRuntimeState::Connected;
        self.tool_discovery_state = McpToolDiscoveryState::Fresh;
        self.last_error = None;
    }

    /// Mark disconnection.
    pub fn mark_disconnected(&mut self) {
        self.state = McpRuntimeState::Disconnected;
        self.tool_discovery_state = McpToolDiscoveryState::Stale;
        self.last_error = None;
    }

    /// Mark a structured failure.
    pub fn mark_error(&mut self, error: &McpError) {
        let failure = McpStructuredFailure::from_error(error);
        self.state = match failure.kind {
            McpFailureKind::Auth => McpRuntimeState::AuthRequired,
            McpFailureKind::Schema => McpRuntimeState::SchemaError,
            McpFailureKind::Disabled => McpRuntimeState::Disabled,
            McpFailureKind::UnsupportedTransport => McpRuntimeState::UnsupportedTransport,
            _ => McpRuntimeState::ConnectionError,
        };
        self.tool_discovery_state = match failure.kind {
            McpFailureKind::Schema => McpToolDiscoveryState::SchemaError,
            McpFailureKind::Disabled | McpFailureKind::UnsupportedTransport => {
                McpToolDiscoveryState::Unavailable
            }
            _ => McpToolDiscoveryState::Deferred,
        };
        self.last_error = Some(failure);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{McpAuthConfig, McpAuthKind, McpServerConfig};
    use crate::mcp::source::{McpServerSource, merge_mcp_sources};

    #[test]
    fn mcp_runtime_status_auth_required_blocks_tools() {
        let source = McpServerSource::direct(
            "secure",
            McpServerConfig::http("https://mcp.example.test").with_auth(McpAuthConfig {
                required: true,
                kind: McpAuthKind::OAuth,
                token_env: None,
                authorization_url: Some("https://auth.example.test/start".to_string()),
                scopes: Vec::new(),
            }),
            true,
        );
        let source_set = merge_mcp_sources([source]).expect("merge");
        let status =
            McpServerRuntimeStatus::from_source(source_set.get("secure").expect("secure source"));

        assert_eq!(status.state, McpRuntimeState::AuthRequired);
        assert!(status.auth_blocks_tools());
        assert_eq!(status.tool_discovery_state, McpToolDiscoveryState::Deferred);
    }

    #[test]
    fn mcp_runtime_status_disabled_source_is_structured() {
        let mut config = McpServerConfig::stdio("docs", Vec::new());
        config.enabled = false;
        let source_set =
            merge_mcp_sources([McpServerSource::direct("docs", config, true)]).expect("merge");
        let status =
            McpServerRuntimeStatus::from_source(source_set.get("docs").expect("docs source"));

        assert_eq!(status.state, McpRuntimeState::Disabled);
        assert_eq!(
            status.tool_discovery_state,
            McpToolDiscoveryState::Unavailable
        );
    }

    #[test]
    fn mcp_schema_failure_marks_only_that_server_schema_error() {
        let source_set = merge_mcp_sources([
            McpServerSource::direct(
                "broken",
                McpServerConfig::stdio("broken-docs", Vec::new()),
                true,
            ),
            McpServerSource::direct("ok", McpServerConfig::stdio("ok-docs", Vec::new()), true),
        ])
        .expect("merge");
        let mut broken =
            McpServerRuntimeStatus::from_source(source_set.get("broken").expect("broken"));
        let ok = McpServerRuntimeStatus::from_source(source_set.get("ok").expect("ok"));

        broken.mark_error(&McpError::schema("invalid tool schema"));

        assert_eq!(broken.state, McpRuntimeState::SchemaError);
        assert_eq!(
            broken.tool_discovery_state,
            McpToolDiscoveryState::SchemaError
        );
        assert_eq!(ok.state, McpRuntimeState::Disconnected);
    }
}
