//! Tests for the credential resolver

use super::*;
use serial_test::serial;
use tempfile::tempdir;

struct EnvVarGuard {
    values: Vec<(&'static str, Option<String>)>,
}

impl EnvVarGuard {
    fn clean(vars: &[&'static str]) -> Self {
        let values = vars
            .iter()
            .map(|var| {
                let value = std::env::var(var).ok();
                unsafe {
                    std::env::remove_var(var);
                }
                (*var, value)
            })
            .collect();

        Self { values }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        for (var, value) in &self.values {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(var, value),
                    None => std::env::remove_var(var),
                }
            }
        }
    }
}

#[test]
fn test_resolver_with_defaults() {
    let resolver = CredentialResolver::with_defaults();
    let config = resolver.config();
    assert!(!config.providers.is_empty());
}

#[test]
#[serial]
fn test_resolve_from_env() {
    let _env = EnvVarGuard::clean(&["SAGE_TEST_ANTHROPIC_KEY"]);

    // Set a test environment variable
    let test_key = "test_key_12345";

    // Use unsafe block for Rust 2024
    unsafe {
        std::env::set_var("SAGE_TEST_ANTHROPIC_KEY", test_key);
    }

    let config = ResolverConfig::default();
    let resolver = CredentialResolver::new(config);
    let credential = resolver.resolve_provider("test_anthropic", "SAGE_TEST_ANTHROPIC_KEY");

    assert!(credential.has_value());
    assert_eq!(credential.value(), Some(test_key));
}

#[test]
fn test_resolve_missing_provider() {
    // Use a unique env var name that definitely doesn't exist
    let resolver = CredentialResolver::with_defaults();
    let credential =
        resolver.resolve_provider("nonexistent_provider_xyz", "NONEXISTENT_XYZ_API_KEY");

    assert!(!credential.has_value());
}

#[test]
fn test_resolve_from_cli_key() {
    let config = ResolverConfig::default().with_cli_key("test_provider", "cli_key_value");
    let resolver = CredentialResolver::new(config);

    let credential = resolver.resolve_provider("test_provider", "TEST_PROVIDER_API_KEY");

    assert!(credential.has_value());
    assert_eq!(credential.value(), Some("cli_key_value"));
}

#[test]
fn test_save_and_load_credential() {
    let temp_dir = tempdir().unwrap();
    let global_dir = temp_dir.path().join(".sage");

    let config = ResolverConfig::default().with_global_dir(&global_dir);
    let resolver = CredentialResolver::new(config);

    // Save a credential
    resolver
        .save_credential("test_provider", "test_api_key")
        .unwrap();

    // Create a new resolver to load it
    let config2 = ResolverConfig::default().with_global_dir(&global_dir);
    let resolver2 = CredentialResolver::new(config2);
    let credential = resolver2.resolve_provider("test_provider", "NONEXISTENT_ENV");

    assert!(credential.has_value());
    assert_eq!(credential.value(), Some("test_api_key"));
}

#[test]
fn test_get_status() {
    let temp_dir = tempdir().unwrap();
    let config = ResolverConfig::default()
        .with_global_dir(temp_dir.path().join(".sage"))
        .with_auto_import(false);
    let resolver = CredentialResolver::new(config);

    let status = resolver.get_status();
    // The status should exist (we use is_ready or needs_onboarding)
    assert!(status.status.is_ready() || status.status.needs_onboarding());
}

#[test]
#[serial]
fn test_priority_cli_over_env() {
    let _env = EnvVarGuard::clean(&["SAGE_TEST_PRIORITY_KEY"]);

    // Set env var with unique name
    unsafe {
        std::env::set_var("SAGE_TEST_PRIORITY_KEY", "env_value");
    }

    let config = ResolverConfig::default().with_cli_key("test_priority", "cli_value");
    let resolver = CredentialResolver::new(config);

    let credential = resolver.resolve_provider("test_priority", "SAGE_TEST_PRIORITY_KEY");

    // CLI should take priority
    assert!(credential.has_value());
    assert_eq!(credential.value(), Some("cli_value"));
}

#[test]
fn test_for_directory() {
    let temp_dir = tempdir().unwrap();
    let resolver = CredentialResolver::for_directory(temp_dir.path());

    assert_eq!(resolver.config().working_dir, temp_dir.path());
}

#[test]
fn test_resolver_default() {
    let resolver = CredentialResolver::default();
    let config = resolver.config();

    // Should have default providers
    assert!(!config.providers.is_empty());
    assert!(config.enable_auto_import);
}

#[test]
#[serial]
fn has_default_provider_accepts_azure_legacy_env_var() {
    let _env = EnvVarGuard::clean(&["AZURE_API_KEY", "AZURE_OPENAI_API_KEY"]);

    // Regression guard for the Azure resolver regression: before this
    // fix, adding `AZURE_OPENAI_API_KEY` as the canonical Azure entry
    // caused `has_default_provider("azure")` to ignore users who only
    // had the long-supported `AZURE_API_KEY` set. The resolver now
    // tries every standard env-var name from
    // `get_standard_env_vars_for_provider("azure")`, so both names
    // resolve.
    //
    // This test must use the actual Azure env var names because they
    // are part of the production fallback list. EnvVarGuard restores
    // any caller-provided values on normal exit or panic.

    let resolver = CredentialResolver::new(ResolverConfig::default());

    // Case A: only the legacy env var is set.
    // SAFETY: this test holds the serial env lock and EnvVarGuard restores state.
    unsafe { std::env::set_var("AZURE_API_KEY", "legacy_secret") };
    let configured = resolver.has_default_provider("azure");
    if !configured {
        panic!(
            "AZURE_API_KEY-only users must show as configured (regression of multi-env-var fallback)"
        );
    }

    // Case B: only the canonical env var is set.
    // SAFETY: this test holds the serial env lock and EnvVarGuard restores state.
    unsafe {
        std::env::remove_var("AZURE_API_KEY");
        std::env::set_var("AZURE_OPENAI_API_KEY", "canonical_secret");
    }
    let configured_canonical = resolver.has_default_provider("azure");
    if !configured_canonical {
        panic!("AZURE_OPENAI_API_KEY-only users must show as configured");
    }

    // Case C: neither set.
    // SAFETY: this test holds the serial env lock and EnvVarGuard restores state.
    unsafe {
        std::env::remove_var("AZURE_API_KEY");
        std::env::remove_var("AZURE_OPENAI_API_KEY");
    }
    let configured_neither = resolver.has_default_provider("azure");
    if configured_neither {
        panic!("with neither env var set, azure must NOT show as configured");
    }
}
