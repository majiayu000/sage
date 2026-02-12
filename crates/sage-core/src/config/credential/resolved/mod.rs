//! Resolved credential with source tracking
//!
//! This module provides the ResolvedCredential type which wraps a credential value
//! with metadata about where it came from, enabling transparency and debugging.

mod collection;
mod credential;

pub use collection::ResolvedCredentials;
pub use credential::ResolvedCredential;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::credential::source::{CredentialPriority, CredentialSource};
    use chrono::Utc;

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
        assert!(!expired.has_value());
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
        let env_cred = ResolvedCredential::new("key", "test", CredentialSource::env("TEST_KEY"));
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

        let cred3 = ResolvedCredential::new(
            "different",
            "openai",
            CredentialSource::env("OPENAI_API_KEY"),
        );
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

        creds.add(ResolvedCredential::new(
            "default-key",
            "openai",
            CredentialSource::Default,
        ));
        assert_eq!(creds.get_api_key("openai"), Some("default-key"));

        // Higher priority should replace
        creds.add(ResolvedCredential::new(
            "env-key",
            "openai",
            CredentialSource::env("OPENAI_API_KEY"),
        ));
        assert_eq!(creds.get_api_key("openai"), Some("env-key"));
        assert_eq!(creds.len(), 1);

        // Lower priority should NOT replace
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
