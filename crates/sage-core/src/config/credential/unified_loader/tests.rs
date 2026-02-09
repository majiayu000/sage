//! Tests for the unified config loader

use super::*;
use serial_test::serial;
use std::env;
use tempfile::tempdir;

fn clean_env() {
    unsafe {
        env::remove_var("ANTHROPIC_API_KEY");
        env::remove_var("OPENAI_API_KEY");
    }
}

#[test]
fn test_unified_loader_new() {
    let loader = UnifiedConfigLoader::new();
    assert!(loader.config_file.is_none());
}

#[test]
fn test_unified_loader_builder() {
    let dir = tempdir().unwrap();
    let loader = UnifiedConfigLoader::new()
        .with_config_file("config.json")
        .with_working_dir(dir.path())
        .with_global_dir(dir.path().join("global"))
        .with_cli_overrides(CliOverrides::new().with_provider("openai"));

    assert_eq!(loader.config_file, Some(PathBuf::from("config.json")));
    assert_eq!(loader.working_dir, dir.path());
    assert_eq!(loader.global_dir, dir.path().join("global"));
    assert_eq!(loader.cli_overrides.provider, Some("openai".to_string()));
}

#[test]
#[serial]
fn test_unified_loader_load_no_file() {
    clean_env();

    let dir = tempdir().unwrap();
    let loader = UnifiedConfigLoader::new()
        .with_working_dir(dir.path())
        .with_global_dir(dir.path().join("global"));

    let result = loader.load();

    // Should return default config
    assert!(result.config.model_providers.contains_key("anthropic"));
    // Should be unconfigured (no keys found)
    assert!(result.needs_onboarding());
}

#[test]
#[serial]
fn test_unified_loader_load_with_file() {
    clean_env();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join("sage_config.json");

    // Create a config file
    let config_content = r#"{
        "default_provider": "openai",
        "model_providers": {
            "openai": {
                "model": "gpt-4",
                "api_key": "test-key"
            }
        }
    }"#;
    std::fs::write(&config_path, config_content).unwrap();

    let loader = UnifiedConfigLoader::new()
        .with_config_file(&config_path)
        .with_working_dir(dir.path())
        .with_global_dir(dir.path().join("global"));

    let result = loader.load();

    assert_eq!(result.config.default_provider, "openai");
    assert!(result.config_file.is_some());
    assert!(result.warnings.is_empty());
}

#[test]
#[serial]
fn test_unified_loader_load_nonexistent_file() {
    clean_env();

    let dir = tempdir().unwrap();
    let loader = UnifiedConfigLoader::new()
        .with_config_file("/nonexistent/path/config.json")
        .with_working_dir(dir.path())
        .with_global_dir(dir.path().join("global"));

    let result = loader.load();

    // Should still return valid config (defaults)
    assert!(!result.config.model_providers.is_empty());
    // Should have warning about missing file
    assert!(!result.warnings.is_empty());
}

#[test]
#[serial]
fn test_unified_loader_cli_overrides() {
    clean_env();

    let dir = tempdir().unwrap();
    let loader = UnifiedConfigLoader::new()
        .with_working_dir(dir.path())
        .with_global_dir(dir.path().join("global"))
        .with_cli_overrides(
            CliOverrides::new()
                .with_provider("openai")
                .with_max_steps(100)
                .with_api_key("cli-api-key"),
        );

    let result = loader.load();

    assert_eq!(result.config.default_provider, "openai");
    assert_eq!(result.config.max_steps, Some(100));

    // CLI API key should be applied
    let openai_params = result.config.model_providers.get("openai").unwrap();
    assert_eq!(openai_params.api_key, Some("cli-api-key".to_string()));
}

#[test]
#[serial]
fn test_unified_loader_env_var_resolution() {
    clean_env();

    unsafe {
        env::set_var("ANTHROPIC_API_KEY", "env-anthropic-key");
    }

    let dir = tempdir().unwrap();
    let loader = UnifiedConfigLoader::new()
        .with_working_dir(dir.path())
        .with_global_dir(dir.path().join("global"));

    let result = loader.load();

    // Should resolve API key from environment
    let anthropic_params = result.config.model_providers.get("anthropic").unwrap();
    assert_eq!(
        anthropic_params.api_key,
        Some("env-anthropic-key".to_string())
    );

    // Should be at least partial status
    assert!(result.is_ready());

    clean_env();
}

#[test]
#[serial]
fn test_unified_loader_project_config_discovery() {
    clean_env();

    let dir = tempdir().unwrap();

    // Create project-level config
    let config_content = r#"{
        "default_provider": "google",
        "model_providers": {
            "google": {
                "model": "gemini-pro",
                "api_key": "project-key"
            }
        }
    }"#;
    std::fs::write(dir.path().join("sage_config.json"), config_content).unwrap();

    let loader = UnifiedConfigLoader::new()
        .with_working_dir(dir.path())
        .with_global_dir(dir.path().join("global"));

    let result = loader.load();

    assert_eq!(result.config.default_provider, "google");
    assert!(result.config_file.is_some());
}

#[test]
fn test_unified_loader_default() {
    let loader = UnifiedConfigLoader::default();
    assert!(loader.config_file.is_none());
}

#[test]
#[serial]
fn test_load_config_unified_function() {
    clean_env();

    let result = load_config_unified(None);

    // Should return valid config with defaults
    assert!(!result.config.model_providers.is_empty());
}

#[test]
#[serial]
fn test_unified_loader_env_var_placeholder_replacement() {
    clean_env();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join("sage_config.json");

    // Create a config with env var placeholder
    let config_content = r#"{
        "default_provider": "anthropic",
        "model_providers": {
            "anthropic": {
                "model": "claude-3",
                "api_key": "${ANTHROPIC_API_KEY}"
            }
        }
    }"#;
    std::fs::write(&config_path, config_content).unwrap();

    // Set the env var
    unsafe {
        env::set_var("ANTHROPIC_API_KEY", "resolved-key");
    }

    let loader = UnifiedConfigLoader::new()
        .with_config_file(&config_path)
        .with_working_dir(dir.path())
        .with_global_dir(dir.path().join("global"));

    let result = loader.load();

    // The placeholder should be replaced with the actual env var value
    let anthropic_params = result.config.model_providers.get("anthropic").unwrap();
    assert_eq!(anthropic_params.api_key, Some("resolved-key".to_string()));

    clean_env();
}
