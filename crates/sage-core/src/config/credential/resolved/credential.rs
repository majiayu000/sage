//! Single resolved credential with source tracking

use super::super::source::{CredentialPriority, CredentialSource};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A resolved credential with its source information
///
/// Wraps the actual credential value with metadata about:
/// - Where the credential came from (source)
/// - When it was resolved
/// - Whether it's been validated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedCredential {
    /// The credential value (API key, token, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,

    /// The provider this credential is for (e.g., "anthropic", "openai")
    pub provider: String,

    /// Where this credential came from
    pub source: CredentialSource,

    /// When this credential was resolved
    pub resolved_at: DateTime<Utc>,

    /// Whether this credential has been validated
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

    /// Get the credential value (None if missing or expired)
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
        self.expires_at.map(|exp| Utc::now() > exp).unwrap_or(false)
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
