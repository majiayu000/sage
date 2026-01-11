//! Tests for the credential resolver

use super::*;
use tempfile::tempdir;

#[test]
fn test_resolver_with_defaults() {
    let resolver = CredentialResolver::with_defaults();
    let config = resolver.config();
    assert!(!config.providers.is_empty());
}

#[test]
fn test_resolve_from_env() {
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

    // Clean up
    unsafe {
        std::env::remove_var("SAGE_TEST_ANTHROPIC_KEY");
    }
}

#[test]
fn test_resolve_missing_provider() {
    // Use a unique env var name that definitely doesn't exist
    let resolver = CredentialResolver::with_defaults();
    let credential = resolver.resolve_provider("nonexistent_provider_xyz", "NONEXISTENT_XYZ_API_KEY");

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
fn test_priority_cli_over_env() {
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

    // Clean up
    unsafe {
        std::env::remove_var("SAGE_TEST_PRIORITY_KEY");
    }
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
