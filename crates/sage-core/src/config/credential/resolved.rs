//! Resolved credential with source tracking
//!
//! This module provides the ResolvedCredential type which wraps a credential value
//! with metadata about where it came from, enabling transparency and debugging.

use super::source::{CredentialPriority, CredentialSource};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A resolved credential with its source information
///
/// This struct wraps the actual credential value with metadata about:
/// - Where the credential came from (source)
/// - When it was resolved
/// - Whether it's been validated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedCredential {
    /// The credential value (API key, token, etc.)
    /// Stored as a string but should be treated as sensitive
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,

    /// The provider this credential is for (e.g., "anthropic", "openai")
    pub provider: String,

    /// Where this credential came from
    pub source: CredentialSource,

    /// When this credential was resolved
    pub resolved_at: DateTime<Utc>,

    /// Whether this credential has been validated (e.g., by making a test API call)
    #[serde(default)]
    pub validated: bool,

    /// Optional expiration time for tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl ResolvedCredential {
    /// Create a new resolved credential
    pub fn new(
        value: impl Into<String>,
        provider: impl Into<String>,
        source: CredentialSource,
    ) -> Self {
        Self {
            value: Some(value.into()),
            provider: provider.into(),
            source,
            resolved_at: Utc::now(),
            validated: false,
            expires_at: None,
        }
    }

    /// Create an empty/missing credential for a provider
    pub fn missing(provider: impl Into<String>) -> Self {
        Self {
            value: None,
            provider: provider.into(),
            source: CredentialSource::Default,
            resolved_at: Utc::now(),
            validated: false,
            expires_at: None,
        }
    }

    /// Get the credential value
    ///
    /// Returns None if the credential is missing or expired
    pub fn value(&self) -> Option<&str> {
        if self.is_expired() {
            return None;
        }
        self.value.as_deref()
    }

    /// Get the credential value, consuming self
    pub fn into_value(self) -> Option<String> {
        if self.is_expired() {
            return None;
        }
        self.value
    }

    /// Check if this credential has a value
    pub fn has_value(&self) -> bool {
        self.value.is_some() && !self.is_expired()
    }

    /// Check if this credential is missing
    pub fn is_missing(&self) -> bool {
        self.value.is_none()
    }

    /// Check if this credential is expired
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| Utc::now() > exp)
            .unwrap_or(false)
    }

    /// Check if this credential is valid (has value, not expired, validated)
    pub fn is_valid(&self) -> bool {
        self.has_value() && self.validated
    }

    /// Get the priority of this credential's source
    pub fn priority(&self) -> CredentialPriority {
        self.source.priority()
    }

    /// Mark this credential as validated
    pub fn mark_validated(mut self) -> Self {
        self.validated = true;
        self
    }

    /// Set an expiration time
    pub fn with_expiration(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Create a masked version for display (e.g., "sk-...abc123")
    pub fn masked_value(&self) -> String {
        match &self.value {
            Some(v) if v.len() > 8 => {
                let prefix = &v[..3.min(v.len())];
                let suffix = &v[v.len().saturating_sub(6)..];
                format!("{}...{}", prefix, suffix)
            }
            Some(v) => "*".repeat(v.len()),
            None => "(missing)".to_string(),
        }
    }

    /// Get a summary of where this credential came from
    pub fn source_summary(&self) -> String {
        format!(
            "{} from {}",
            if self.has_value() {
                "Loaded"
            } else {
                "Missing"
            },
            self.source
        )
    }
}

impl fmt::Display for ResolvedCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} ({})",
            self.provider,
            self.masked_value(),
            self.source.priority().name()
        )
    }
}

impl PartialEq for ResolvedCredential {
    fn eq(&self, other: &Self) -> bool {
        self.provider == other.provider
            && self.value == other.value
            && self.source == other.source
            && self.validated == other.validated
    }
}

impl Eq for ResolvedCredential {}

/// A collection of resolved credentials for multiple providers
#[derive(Debug, Clone, Default)]
pub struct ResolvedCredentials {
    credentials: Vec<ResolvedCredential>,
}

impl ResolvedCredentials {
    /// Create a new empty collection
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a resolved credential
    pub fn add(&mut self, credential: ResolvedCredential) {
        // Replace existing credential for same provider if new one has higher priority
        if let Some(existing) = self
            .credentials
            .iter_mut()
            .find(|c| c.provider == credential.provider)
        {
            if credential.priority() < existing.priority() {
                *existing = credential;
            }
        } else {
            self.credentials.push(credential);
        }
    }

    /// Get a credential for a specific provider
    pub fn get(&self, provider: &str) -> Option<&ResolvedCredential> {
        self.credentials.iter().find(|c| c.provider == provider)
    }

    /// Get the API key for a provider
    pub fn get_api_key(&self, provider: &str) -> Option<&str> {
        self.get(provider).and_then(|c| c.value())
    }

    /// Check if any credentials are configured
    pub fn has_any(&self) -> bool {
        self.credentials.iter().any(|c| c.has_value())
    }

    /// Get all configured providers
    pub fn configured_providers(&self) -> Vec<&str> {
        self.credentials
            .iter()
            .filter(|c| c.has_value())
            .map(|c| c.provider.as_str())
            .collect()
    }

    /// Get all missing providers
    pub fn missing_providers(&self) -> Vec<&str> {
        self.credentials
            .iter()
            .filter(|c| c.is_missing())
            .map(|c| c.provider.as_str())
            .collect()
    }

    /// Iterate over all credentials
    pub fn iter(&self) -> impl Iterator<Item = &ResolvedCredential> {
        self.credentials.iter()
    }

    /// Get the number of credentials
    pub fn len(&self) -> usize {
        self.credentials.len()
    }

    /// Check if the collection is empty
    pub fn is_empty(&self) -> bool {
        self.credentials.is_empty()
    }
}

impl IntoIterator for ResolvedCredentials {
    type Item = ResolvedCredential;
    type IntoIter = std::vec::IntoIter<ResolvedCredential>;

    fn into_iter(self) -> Self::IntoIter {
        self.credentials.into_iter()
    }
}

impl<'a> IntoIterator for &'a ResolvedCredentials {
    type Item = &'a ResolvedCredential;
    type IntoIter = std::slice::Iter<'a, ResolvedCredential>;

    fn into_iter(self) -> Self::IntoIter {
        self.credentials.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolved_credential_new() {
        let cred = ResolvedCredential::new(
            "sk-test-12345",
            "openai",
            CredentialSource::env("OPENAI_API_KEY"),
        );

        assert_eq!(cred.provider, "openai");
        assert_eq!(cred.value(), Some("sk-test-12345"));
        assert!(cred.has_value());
        assert!(!cred.is_missing());
        assert!(!cred.validated);
    }

    #[test]
    fn test_resolved_credential_missing() {
        let cred = ResolvedCredential::missing("anthropic");

        assert_eq!(cred.provider, "anthropic");
        assert!(cred.value().is_none());
        assert!(!cred.has_value());
        assert!(cred.is_missing());
    }

    #[test]
    fn test_resolved_credential_into_value() {
        let cred = ResolvedCredential::new("test-key", "test", CredentialSource::Default);
        assert_eq!(cred.into_value(), Some("test-key".to_string()));

        let missing = ResolvedCredential::missing("test");
        assert_eq!(missing.into_value(), None);
    }

    #[test]
    fn test_resolved_credential_expiration() {
        let cred = ResolvedCredential::new("test-key", "test", CredentialSource::Default);
        assert!(!cred.is_expired());

        let expired = ResolvedCredential::new("test-key", "test", CredentialSource::Default)
            .with_expiration(Utc::now() - chrono::Duration::hours(1));
        assert!(expired.is_expired());
        assert!(!expired.has_value()); // Expired credential should not have value
        assert!(expired.value().is_none());
    }

    #[test]
    fn test_resolved_credential_validation() {
        let cred = ResolvedCredential::new("test-key", "test", CredentialSource::Default);
        assert!(!cred.validated);
        assert!(!cred.is_valid());

        let validated = cred.mark_validated();
        assert!(validated.validated);
        assert!(validated.is_valid());
    }

    #[test]
    fn test_resolved_credential_priority() {
        let env_cred =
            ResolvedCredential::new("key", "test", CredentialSource::env("TEST_KEY"));
        assert_eq!(env_cred.priority(), CredentialPriority::Environment);

        let cli_cred = ResolvedCredential::new("key", "test", CredentialSource::cli("--api-key"));
        assert_eq!(cli_cred.priority(), CredentialPriority::CliArgument);
    }

    #[test]
    fn test_resolved_credential_masked_value() {
        let cred = ResolvedCredential::new(
            "sk-proj-abcdefghijklmnop",
            "openai",
            CredentialSource::Default,
        );
        let masked = cred.masked_value();
        assert!(masked.starts_with("sk-"));
        assert!(masked.contains("..."));
        assert!(!masked.contains("abcdefghijklmnop"));

        let short_cred = ResolvedCredential::new("short", "test", CredentialSource::Default);
        assert_eq!(short_cred.masked_value(), "*****");

        let missing = ResolvedCredential::missing("test");
        assert_eq!(missing.masked_value(), "(missing)");
    }

    #[test]
    fn test_resolved_credential_source_summary() {
        let cred =
            ResolvedCredential::new("key", "openai", CredentialSource::env("OPENAI_API_KEY"));
        let summary = cred.source_summary();
        assert!(summary.contains("Loaded"));
        assert!(summary.contains("OPENAI_API_KEY"));

        let missing = ResolvedCredential::missing("anthropic");
        let summary = missing.source_summary();
        assert!(summary.contains("Missing"));
    }

    #[test]
    fn test_resolved_credential_display() {
        let cred =
            ResolvedCredential::new("sk-test-1234567890", "openai", CredentialSource::Default);
        let display = format!("{}", cred);
        assert!(display.contains("openai"));
        assert!(display.contains("..."));
        assert!(display.contains("Default"));
    }

    #[test]
    fn test_resolved_credential_equality() {
        let cred1 =
            ResolvedCredential::new("key", "openai", CredentialSource::env("OPENAI_API_KEY"));
        let cred2 =
            ResolvedCredential::new("key", "openai", CredentialSource::env("OPENAI_API_KEY"));
        assert_eq!(cred1, cred2);

        let cred3 =
            ResolvedCredential::new("different", "openai", CredentialSource::env("OPENAI_API_KEY"));
        assert_ne!(cred1, cred3);
    }

    #[test]
    fn test_resolved_credentials_new() {
        let creds = ResolvedCredentials::new();
        assert!(creds.is_empty());
        assert_eq!(creds.len(), 0);
    }

    #[test]
    fn test_resolved_credentials_add() {
        let mut creds = ResolvedCredentials::new();

        creds.add(ResolvedCredential::new(
            "key1",
            "openai",
            CredentialSource::Default,
        ));
        assert_eq!(creds.len(), 1);

        // Adding for different provider
        creds.add(ResolvedCredential::new(
            "key2",
            "anthropic",
            CredentialSource::Default,
        ));
        assert_eq!(creds.len(), 2);
    }

    #[test]
    fn test_resolved_credentials_priority_replacement() {
        let mut creds = ResolvedCredentials::new();

        // Add with lower priority first
        creds.add(ResolvedCredential::new(
            "default-key",
            "openai",
            CredentialSource::Default,
        ));
        assert_eq!(creds.get_api_key("openai"), Some("default-key"));

        // Add with higher priority - should replace
        creds.add(ResolvedCredential::new(
            "env-key",
            "openai",
            CredentialSource::env("OPENAI_API_KEY"),
        ));
        assert_eq!(creds.get_api_key("openai"), Some("env-key"));
        assert_eq!(creds.len(), 1); // Should still be 1, not 2

        // Add with lower priority - should NOT replace
        creds.add(ResolvedCredential::new(
            "global-key",
            "openai",
            CredentialSource::global("/path"),
        ));
        assert_eq!(creds.get_api_key("openai"), Some("env-key"));
    }

    #[test]
    fn test_resolved_credentials_get() {
        let mut creds = ResolvedCredentials::new();
        creds.add(ResolvedCredential::new(
            "key",
            "openai",
            CredentialSource::Default,
        ));

        assert!(creds.get("openai").is_some());
        assert!(creds.get("anthropic").is_none());
    }

    #[test]
    fn test_resolved_credentials_get_api_key() {
        let mut creds = ResolvedCredentials::new();
        creds.add(ResolvedCredential::new(
            "test-key",
            "openai",
            CredentialSource::Default,
        ));

        assert_eq!(creds.get_api_key("openai"), Some("test-key"));
        assert_eq!(creds.get_api_key("anthropic"), None);
    }

    #[test]
    fn test_resolved_credentials_has_any() {
        let mut creds = ResolvedCredentials::new();
        assert!(!creds.has_any());

        creds.add(ResolvedCredential::missing("openai"));
        assert!(!creds.has_any());

        creds.add(ResolvedCredential::new(
            "key",
            "anthropic",
            CredentialSource::Default,
        ));
        assert!(creds.has_any());
    }

    #[test]
    fn test_resolved_credentials_configured_providers() {
        let mut creds = ResolvedCredentials::new();
        creds.add(ResolvedCredential::new(
            "key1",
            "openai",
            CredentialSource::Default,
        ));
        creds.add(ResolvedCredential::missing("anthropic"));
        creds.add(ResolvedCredential::new(
            "key2",
            "google",
            CredentialSource::Default,
        ));

        let configured = creds.configured_providers();
        assert_eq!(configured.len(), 2);
        assert!(configured.contains(&"openai"));
        assert!(configured.contains(&"google"));
        assert!(!configured.contains(&"anthropic"));
    }

    #[test]
    fn test_resolved_credentials_missing_providers() {
        let mut creds = ResolvedCredentials::new();
        creds.add(ResolvedCredential::new(
            "key",
            "openai",
            CredentialSource::Default,
        ));
        creds.add(ResolvedCredential::missing("anthropic"));

        let missing = creds.missing_providers();
        assert_eq!(missing.len(), 1);
        assert!(missing.contains(&"anthropic"));
    }

    #[test]
    fn test_resolved_credentials_iter() {
        let mut creds = ResolvedCredentials::new();
        creds.add(ResolvedCredential::new(
            "key1",
            "openai",
            CredentialSource::Default,
        ));
        creds.add(ResolvedCredential::new(
            "key2",
            "anthropic",
            CredentialSource::Default,
        ));

        let providers: Vec<&str> = creds.iter().map(|c| c.provider.as_str()).collect();
        assert_eq!(providers.len(), 2);
    }

    #[test]
    fn test_resolved_credentials_into_iter() {
        let mut creds = ResolvedCredentials::new();
        creds.add(ResolvedCredential::new(
            "key",
            "openai",
            CredentialSource::Default,
        ));

        for cred in creds {
            assert_eq!(cred.provider, "openai");
        }
    }

    #[test]
    fn test_resolved_credentials_ref_into_iter() {
        let mut creds = ResolvedCredentials::new();
        creds.add(ResolvedCredential::new(
            "key",
            "openai",
            CredentialSource::Default,
        ));

        for cred in &creds {
            assert_eq!(cred.provider, "openai");
        }

        // creds should still be usable
        assert_eq!(creds.len(), 1);
    }
}
