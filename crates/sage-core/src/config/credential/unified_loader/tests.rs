//! Tests for the unified config loader

use super::*;
use crate::config::credential::providers::default_providers;
use serial_test::serial;
use std::env;
use tempfile::tempdir;

struct EnvVarGuard {
    values: Vec<(String, Option<String>)>,
}

impl EnvVarGuard {
    fn clean_config_env() -> Self {
        let mut vars: Vec<String> = default_providers()
            .into_iter()
            .map(|provider| provider.env_var)
            .collect();
        vars.sort();
        vars.dedup();

        let values = vars
            .iter()
            .map(|var| {
                let value = env::var(var).ok();
                unsafe {
                    env::remove_var(var);
                }
                (var.clone(), value)
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
                    Some(value) => env::set_var(var, value),
                    None => env::remove_var(var),
                }
            }
        }
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
    let _env = EnvVarGuard::clean_config_env();

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
    let _env = EnvVarGuard::clean_config_env();

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
    let _env = EnvVarGuard::clean_config_env();

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
    let _env = EnvVarGuard::clean_config_env();

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
    let _env = EnvVarGuard::clean_config_env();

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
}

#[test]
#[serial]
fn test_unified_loader_project_config_discovery() {
    let _env = EnvVarGuard::clean_config_env();

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
    let _env = EnvVarGuard::clean_config_env();

    let result = load_config_unified(None);

    // Should return valid config with defaults
    assert!(!result.config.model_providers.is_empty());
}

#[test]
#[serial]
fn test_unified_loader_env_var_placeholder_replacement() {
    let _env = EnvVarGuard::clean_config_env();

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
}

#[test]
#[serial]
fn strict_loader_skips_missing_global_config_after_project_config()
-> Result<(), Box<dyn std::error::Error>> {
    let _env = EnvVarGuard::clean_config_env();
    let dir = tempdir()?;
    let global_dir = dir.path().join("global");
    std::fs::write(
        dir.path().join("sage_config.toml"),
        r#"default_provider = "ollama""#,
    )?;

    let loader = UnifiedConfigLoader::new()
        .with_working_dir(dir.path())
        .with_global_dir(&global_dir);

    let config = loader.load_strict()?;

    assert_eq!(config.default_provider, "ollama");
    Ok(())
}

#[test]
#[serial]
fn strict_loader_applies_cli_overrides_before_validation() -> Result<(), Box<dyn std::error::Error>>
{
    let _env = EnvVarGuard::clean_config_env();
    let dir = tempdir()?;
    std::fs::write(dir.path().join("sage_config.toml"), "max_steps = 0")?;

    let loader = UnifiedConfigLoader::new()
        .with_working_dir(dir.path())
        .with_global_dir(dir.path().join("global"))
        .with_cli_overrides(CliOverrides::new().with_max_steps(5));

    let config = loader.load_strict()?;

    assert_eq!(config.max_steps, Some(5));
    Ok(())
}

#[test]
#[serial]
fn strict_loader_preserves_cli_overrides_for_provider_aliases()
-> Result<(), Box<dyn std::error::Error>> {
    let _env = EnvVarGuard::clean_config_env();
    let dir = tempdir()?;

    let loader = UnifiedConfigLoader::new()
        .with_working_dir(dir.path())
        .with_global_dir(dir.path().join("global"))
        .with_cli_overrides(
            CliOverrides::new()
                .with_provider("zhipu")
                .with_model("glm-test")
                .with_api_key("zhipu-test-key"),
        );

    let config = loader.load_strict()?;

    assert_eq!(config.default_provider, "zhipu");
    let Some(params) = config.model_providers.get("zhipu") else {
        panic!("zhipu provider entry missing");
    };
    assert_eq!(params.model, "glm-test");
    assert_eq!(params.api_key.as_deref(), Some("zhipu-test-key"));
    assert_eq!(
        params.base_url.as_deref(),
        Some("https://open.bigmodel.cn/api/anthropic")
    );
    Ok(())
}

#[test]
#[serial]
fn strict_loader_applies_persisted_doubao_credentials() -> Result<(), Box<dyn std::error::Error>> {
    let _env = EnvVarGuard::clean_config_env();
    let dir = tempdir()?;
    let global_dir = dir.path().join("global");
    let mut credentials = CredentialsFile::default();
    credentials.set_api_key("doubao", "doubao-test-key");
    credentials.save(&global_dir.join("credentials.json"))?;

    let loader = UnifiedConfigLoader::new()
        .with_working_dir(dir.path())
        .with_global_dir(&global_dir);

    let config = loader.load_strict()?;

    let Some(params) = config.model_providers.get("doubao") else {
        panic!("doubao provider entry missing");
    };
    assert_eq!(params.api_key.as_deref(), Some("doubao-test-key"));
    Ok(())
}
