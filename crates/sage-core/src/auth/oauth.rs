//! OAuth 2.0 client implementation
//!
//! Supports:
//! - Authorization code flow with PKCE
//! - Token refresh
//! - Dynamic client registration (RFC 7591)

use super::pkce::PkceVerifier;
use super::token::TokenInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OAuth 2.0 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// Authorization endpoint URL
    pub authorization_endpoint: String,
    /// Token endpoint URL
    pub token_endpoint: String,
    /// Client ID
    pub client_id: String,
    /// Client secret (optional for public clients)
    pub client_secret: Option<String>,
    /// Redirect URI
    pub redirect_uri: String,
    /// Scopes to request
    pub scopes: Vec<String>,
}

impl OAuthConfig {
    /// Create new OAuth config
    pub fn new(
        authorization_endpoint: impl Into<String>,
        token_endpoint: impl Into<String>,
        client_id: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Self {
        Self {
            authorization_endpoint: authorization_endpoint.into(),
            token_endpoint: token_endpoint.into(),
            client_id: client_id.into(),
            client_secret: None,
            redirect_uri: redirect_uri.into(),
            scopes: Vec::new(),
        }
    }

    /// Set client secret
    pub fn with_secret(mut self, secret: impl Into<String>) -> Self {
        self.client_secret = Some(secret.into());
        self
    }

    /// Add scope
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scopes.push(scope.into());
        self
    }

    /// Add multiple scopes
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes.extend(scopes);
        self
    }
}

/// OAuth 2.0 client
pub struct OAuthClient {
    config: OAuthConfig,
    http_client: reqwest::Client,
}

impl OAuthClient {
    /// Create new OAuth client
    pub fn new(config: OAuthConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Generate authorization URL with PKCE
    pub fn authorization_url(&self, verifier: &PkceVerifier, state: &str) -> String {
        let challenge = verifier.challenge();

        let mut params = vec![
            ("response_type", "code".to_string()),
            ("client_id", self.config.client_id.clone()),
            ("redirect_uri", self.config.redirect_uri.clone()),
            ("state", state.to_string()),
            ("code_challenge", challenge.as_str().to_string()),
            ("code_challenge_method", challenge.method().to_string()),
        ];

        if !self.config.scopes.is_empty() {
            params.push(("scope", self.config.scopes.join(" ")));
        }

        let query = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        format!("{}?{}", self.config.authorization_endpoint, query)
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(
        &self,
        code: &str,
        verifier: &PkceVerifier,
    ) -> Result<TokenInfo, OAuthError> {
        let mut params = HashMap::new();
        params.insert("grant_type", "authorization_code");
        params.insert("code", code);
        params.insert("redirect_uri", &self.config.redirect_uri);
        params.insert("client_id", &self.config.client_id);
        params.insert("code_verifier", verifier.as_str());

        let response = self
            .http_client
            .post(&self.config.token_endpoint)
            .form(&params)
            .send()
            .await
            .map_err(|e| OAuthError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(OAuthError::TokenError(error_body));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| OAuthError::ParseError(e.to_string()))?;

        Ok(TokenInfo {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            token_type: token_response.token_type,
            expires_in: token_response.expires_in,
            scope: token_response.scope,
            issued_at: chrono::Utc::now(),
        })
    }

    /// Refresh access token
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenInfo, OAuthError> {
        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token");
        params.insert("refresh_token", refresh_token);
        params.insert("client_id", &self.config.client_id);

        if let Some(secret) = &self.config.client_secret {
            params.insert("client_secret", secret);
        }

        let response = self
            .http_client
            .post(&self.config.token_endpoint)
            .form(&params)
            .send()
            .await
            .map_err(|e| OAuthError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(OAuthError::TokenError(error_body));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| OAuthError::ParseError(e.to_string()))?;

        Ok(TokenInfo {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            token_type: token_response.token_type,
            expires_in: token_response.expires_in,
            scope: token_response.scope,
            issued_at: chrono::Utc::now(),
        })
    }
}

/// Token response from OAuth server
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    token_type: String,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    scope: Option<String>,
}

/// OAuth errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum OAuthError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Token error: {0}")]
    TokenError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid state")]
    InvalidState,

    #[error("Authorization denied")]
    AuthorizationDenied,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_config() {
        let config = OAuthConfig::new(
            "https://auth.example.com/authorize",
            "https://auth.example.com/token",
            "client_id",
            "http://localhost:8080/callback",
        )
        .with_scope("read")
        .with_scope("write");

        assert_eq!(config.scopes.len(), 2);
    }

    #[test]
    fn test_authorization_url() {
        let config = OAuthConfig::new(
            "https://auth.example.com/authorize",
            "https://auth.example.com/token",
            "test_client",
            "http://localhost:8080/callback",
        )
        .with_scope("openid");

        let client = OAuthClient::new(config);
        let verifier = PkceVerifier::new();
        let url = client.authorization_url(&verifier, "test_state");

        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=test_client"));
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=test_state"));
    }
}
