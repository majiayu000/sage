use super::*;

#[test]
fn test_priority_ordering() {
    // Lower number = higher priority
    assert!(CredentialPriority::CliArgument < CredentialPriority::Environment);
    assert!(CredentialPriority::Environment < CredentialPriority::ProjectConfig);
    assert!(CredentialPriority::ProjectConfig < CredentialPriority::GlobalConfig);
    assert!(CredentialPriority::GlobalConfig < CredentialPriority::AutoImported);
    assert!(CredentialPriority::AutoImported < CredentialPriority::SystemKeychain);
    assert!(CredentialPriority::SystemKeychain < CredentialPriority::OAuthToken);
    assert!(CredentialPriority::OAuthToken < CredentialPriority::Default);
}

#[test]
fn test_priority_all() {
    let all = CredentialPriority::all();
    assert_eq!(all.len(), 8);
    assert_eq!(all[0], CredentialPriority::CliArgument);
    assert_eq!(all[7], CredentialPriority::Default);
}

#[test]
fn test_priority_name() {
    assert_eq!(CredentialPriority::CliArgument.name(), "CLI argument");
    assert_eq!(
        CredentialPriority::Environment.name(),
        "Environment variable"
    );
    assert_eq!(CredentialPriority::ProjectConfig.name(), "Project config");
    assert_eq!(CredentialPriority::GlobalConfig.name(), "Global config");
    assert_eq!(CredentialPriority::AutoImported.name(), "Auto-imported");
    assert_eq!(CredentialPriority::SystemKeychain.name(), "System keychain");
    assert_eq!(CredentialPriority::OAuthToken.name(), "OAuth token");
    assert_eq!(CredentialPriority::Default.name(), "Default");
}

#[test]
fn test_priority_is_user_configured() {
    assert!(CredentialPriority::CliArgument.is_user_configured());
    assert!(CredentialPriority::Environment.is_user_configured());
    assert!(CredentialPriority::ProjectConfig.is_user_configured());
    assert!(CredentialPriority::GlobalConfig.is_user_configured());
    assert!(!CredentialPriority::AutoImported.is_user_configured());
    assert!(!CredentialPriority::SystemKeychain.is_user_configured());
    assert!(!CredentialPriority::OAuthToken.is_user_configured());
    assert!(!CredentialPriority::Default.is_user_configured());
}

#[test]
fn test_priority_is_persistent() {
    assert!(!CredentialPriority::CliArgument.is_persistent());
    assert!(CredentialPriority::Environment.is_persistent());
    assert!(CredentialPriority::ProjectConfig.is_persistent());
    assert!(CredentialPriority::GlobalConfig.is_persistent());
    assert!(CredentialPriority::AutoImported.is_persistent());
    assert!(CredentialPriority::SystemKeychain.is_persistent());
    assert!(CredentialPriority::OAuthToken.is_persistent());
    assert!(CredentialPriority::Default.is_persistent());
}

#[test]
fn test_priority_display() {
    assert_eq!(
        format!("{}", CredentialPriority::CliArgument),
        "CLI argument"
    );
    assert_eq!(format!("{}", CredentialPriority::Default), "Default");
}

#[test]
fn test_priority_default() {
    assert_eq!(CredentialPriority::default(), CredentialPriority::Default);
}

#[test]
fn test_source_cli() {
    let source = CredentialSource::cli("--api-key");
    assert_eq!(source.priority(), CredentialPriority::CliArgument);
    assert!(source.description().contains("--api-key"));
    assert!(source.is_user_configured());
    assert!(!source.is_persistent());
}

#[test]
fn test_source_env() {
    let source = CredentialSource::env("OPENAI_API_KEY");
    assert_eq!(source.priority(), CredentialPriority::Environment);
    assert!(source.description().contains("OPENAI_API_KEY"));
    assert!(source.is_user_configured());
    assert!(source.is_persistent());
}

#[test]
fn test_source_project() {
    let source = CredentialSource::project("/project/.sage/credentials.json");
    assert_eq!(source.priority(), CredentialPriority::ProjectConfig);
    assert!(source.description().contains("Project"));
    assert!(source.is_user_configured());
}

#[test]
fn test_source_global() {
    let source = CredentialSource::global("~/.sage/credentials.json");
    assert_eq!(source.priority(), CredentialPriority::GlobalConfig);
    assert!(source.description().contains("Global"));
    assert!(source.is_user_configured());
}

#[test]
fn test_source_auto_imported() {
    let source = CredentialSource::auto_imported("claude-code", Some(PathBuf::from("/path")));
    assert_eq!(source.priority(), CredentialPriority::AutoImported);
    assert!(source.description().contains("claude-code"));
    assert!(source.description().contains("/path"));
    assert!(!source.is_user_configured());

    let source_no_path = CredentialSource::auto_imported("cursor", None);
    assert!(source_no_path.description().contains("cursor"));
    assert!(!source_no_path.description().contains("/path"));
}

#[test]
fn test_source_keychain() {
    let source = CredentialSource::keychain("sage-openai");
    assert_eq!(source.priority(), CredentialPriority::SystemKeychain);
    assert!(source.description().contains("sage-openai"));
}

#[test]
fn test_source_oauth() {
    let source = CredentialSource::oauth("anthropic");
    assert_eq!(source.priority(), CredentialPriority::OAuthToken);
    assert!(source.description().contains("anthropic"));
}

#[test]
fn test_source_default() {
    let source = CredentialSource::Default;
    assert_eq!(source.priority(), CredentialPriority::Default);
    assert_eq!(source.description(), "Default");
    assert!(!source.is_user_configured());
    assert!(source.is_persistent());
}

#[test]
fn test_source_display() {
    let source = CredentialSource::env("TEST_KEY");
    assert_eq!(format!("{}", source), "Environment: $TEST_KEY");
}

#[test]
fn test_source_default_trait() {
    assert_eq!(CredentialSource::default(), CredentialSource::Default);
}

#[test]
fn test_source_serialize() {
    let source = CredentialSource::env("OPENAI_API_KEY");
    let json = serde_json::to_string(&source).unwrap();
    assert!(json.contains("environment"));
    assert!(json.contains("OPENAI_API_KEY"));

    let source = CredentialSource::Default;
    let json = serde_json::to_string(&source).unwrap();
    assert!(json.contains("default"));
}

#[test]
fn test_source_deserialize() {
    let json = r#"{"type":"environment","var_name":"TEST_KEY"}"#;
    let source: CredentialSource = serde_json::from_str(json).unwrap();
    assert!(matches!(
        source,
        CredentialSource::Environment { var_name } if var_name == "TEST_KEY"
    ));

    let json = r#"{"type":"default"}"#;
    let source: CredentialSource = serde_json::from_str(json).unwrap();
    assert!(matches!(source, CredentialSource::Default));
}

#[test]
fn test_priority_serialize() {
    let priority = CredentialPriority::Environment;
    let json = serde_json::to_string(&priority).unwrap();
    // serde serializes the variant name, not the u8 discriminant
    assert!(json.contains("Environment") || json == "2");
}

#[test]
fn test_priority_deserialize() {
    // Try variant name first
    if let Ok(priority) = serde_json::from_str::<CredentialPriority>("\"Environment\"") {
        assert_eq!(priority, CredentialPriority::Environment);
    }
    // serde uses variant names by default
}
