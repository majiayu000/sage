//! MCP authentication status and recovery prompts.

use crate::config::{McpAuthConfig, McpAuthKind, McpServerConfig};
use serde::{Deserialize, Serialize};

/// Runtime authentication state for an MCP server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpAuthState {
    /// Server does not require authentication.
    NotRequired,
    /// Required authentication has not been completed.
    AuthRequired,
    /// Authorization has started but is not complete.
    Pending,
    /// Server has enough local auth material to run tools.
    Authorized,
    /// The previous authorization attempt failed.
    Failed,
}

/// Prompt shown to a caller when authorization is required.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpAuthorizationPrompt {
    /// Server id requiring authorization.
    pub server_id: String,
    /// Auth method requested by the server.
    pub kind: McpAuthKind,
    /// Human-readable action.
    pub message: String,
    /// Optional URL for OAuth or browser-based auth.
    pub authorization_url: Option<String>,
    /// Optional token environment variable expected by the server.
    pub token_env: Option<String>,
    /// Optional OAuth scopes or server-specific grants.
    pub scopes: Vec<String>,
}

/// Programmatic MCP auth status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpAuthStatus {
    /// Current auth state.
    pub state: McpAuthState,
    /// Authorization prompt when user action is needed.
    pub prompt: Option<McpAuthorizationPrompt>,
    /// Recovery hint suitable for diagnostics or tool errors.
    pub recovery_hint: Option<String>,
}

impl McpAuthStatus {
    /// No authentication is required.
    pub fn not_required() -> Self {
        Self {
            state: McpAuthState::NotRequired,
            prompt: None,
            recovery_hint: None,
        }
    }

    /// Authentication is complete.
    pub fn authorized() -> Self {
        Self {
            state: McpAuthState::Authorized,
            prompt: None,
            recovery_hint: None,
        }
    }

    /// Authentication is required before tools may run.
    pub fn required(server_id: impl Into<String>, auth: &McpAuthConfig) -> Self {
        let server_id = server_id.into();
        let action = match auth.kind {
            McpAuthKind::None => "Configure authentication".to_string(),
            McpAuthKind::Bearer => match auth.token_env.as_deref() {
                Some(name) => format!("Set {name} before connecting to this MCP server"),
                None => "Configure a bearer token for this MCP server".to_string(),
            },
            McpAuthKind::OAuth => "Complete OAuth authorization for this MCP server".to_string(),
        };

        Self {
            state: McpAuthState::AuthRequired,
            prompt: Some(McpAuthorizationPrompt {
                server_id,
                kind: auth.kind.clone(),
                message: action.clone(),
                authorization_url: auth.authorization_url.clone(),
                token_env: auth.token_env.clone(),
                scopes: auth.scopes.clone(),
            }),
            recovery_hint: Some(action),
        }
    }

    /// Authorization started and needs a later retry.
    pub fn pending(prompt: McpAuthorizationPrompt) -> Self {
        Self {
            state: McpAuthState::Pending,
            recovery_hint: Some("Retry after authorization completes".to_string()),
            prompt: Some(prompt),
        }
    }

    /// Authorization failed.
    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            state: McpAuthState::Failed,
            prompt: None,
            recovery_hint: Some(message.into()),
        }
    }

    /// Build auth status from MCP server configuration without storing secrets.
    pub fn from_server_config(server_id: &str, config: &McpServerConfig) -> Self {
        let Some(auth) = config.auth.as_ref() else {
            return Self::not_required();
        };

        if !auth.required {
            return Self::not_required();
        }

        if auth
            .token_env
            .as_deref()
            .and_then(|name| std::env::var(name).ok())
            .is_some()
        {
            return Self::authorized();
        }

        Self::required(server_id, auth)
    }

    /// Whether the current auth state must block tool execution.
    pub fn blocks_tools(&self) -> bool {
        matches!(
            self.state,
            McpAuthState::AuthRequired | McpAuthState::Pending | McpAuthState::Failed
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{McpAuthConfig, McpAuthKind, McpServerConfig};

    #[test]
    fn mcp_auth_status_not_required_without_config() {
        let status = McpAuthStatus::from_server_config(
            "docs",
            &McpServerConfig::stdio("docs-server", Vec::new()),
        );

        assert_eq!(status.state, McpAuthState::NotRequired);
        assert!(!status.blocks_tools());
    }

    #[test]
    fn mcp_auth_status_exposes_authorization_prompt_and_recovery() {
        let auth = McpAuthConfig {
            required: true,
            kind: McpAuthKind::OAuth,
            token_env: None,
            authorization_url: Some("https://auth.example.test/start".to_string()),
            scopes: vec!["docs.read".to_string()],
        };
        let status = McpAuthStatus::from_server_config(
            "docs",
            &McpServerConfig::http("https://mcp.example.test").with_auth(auth),
        );

        assert_eq!(status.state, McpAuthState::AuthRequired);
        assert!(status.blocks_tools());
        let prompt = status.prompt.expect("authorization prompt");
        assert_eq!(prompt.server_id, "docs");
        assert_eq!(prompt.kind, McpAuthKind::OAuth);
        assert_eq!(
            prompt.authorization_url.as_deref(),
            Some("https://auth.example.test/start")
        );
        assert_eq!(prompt.scopes, vec!["docs.read"]);
        assert!(status.recovery_hint.is_some());
    }
}
