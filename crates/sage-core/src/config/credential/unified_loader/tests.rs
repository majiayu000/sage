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
        for provider in default_providers() {
            vars.extend(
                crate::config::api_key_helpers::get_standard_env_vars_for_provider(&provider.name),
            );
            let prefix = provider.name.to_uppercase();
            vars.push(format!("SAGE_{prefix}_API_KEY"));
            vars.push(format!("{prefix}_BASE_URL"));
        }
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
fn strict_loader_rejects_missing_explicit_config() {
    let _env = EnvVarGuard::clean_config_env();
    let dir = tempdir().unwrap();
    let global_dir = dir.path().join("global");
    std::fs::create_dir(&global_dir).unwrap();
    std::fs::write(dir.path().join("sage_config.json"), "{}").unwrap();
    std::fs::write(global_dir.join("config.json"), "{}").unwrap();

    for missing_path in [
        dir.path().join("missing.json"),
        global_dir.join("missing.json"),
    ] {
        let loader = UnifiedConfigLoader::new()
            .with_working_dir(dir.path())
            .with_global_dir(&global_dir)
            .with_config_file(&missing_path);
        let error = format!("{:?}", loader.load_strict().unwrap_err());
        assert!(error.contains("Failed to read config file"), "{error}");
        assert!(error.contains(missing_path.to_str().unwrap()), "{error}");
    }

    std::fs::remove_file(global_dir.join("config.json")).unwrap();
    let error = UnifiedConfigLoader::new()
        .with_working_dir(dir.path())
        .with_global_dir(&global_dir)
        .with_config_file(global_dir.join("config.json"))
        .load_strict()
        .unwrap_err()
        .to_string();
    assert!(error.contains("Failed to read config file"), "{error}");
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

fn write_project_endpoint(path: &Path, api_key: Option<&str>) {
    let config = serde_json::json!({
        "default_provider": "anthropic",
        "model_providers": {
            "anthropic": {
                "model": "claude-test",
                "api_key": api_key,
                "base_url": "https://project-endpoint.example.test"
            }
        }
    });
    let content = match path.extension().and_then(|ext| ext.to_str()) {
        Some("toml") => {
            let key = api_key
                .map(|key| format!("api_key = {key:?}\n"))
                .unwrap_or_default();
            format!(
                "default_provider = \"anthropic\"\n[model_providers.anthropic]\nmodel = \"claude-test\"\nbase_url = \"https://project-endpoint.example.test\"\n{key}"
            )
        }
        Some("yaml" | "yml") => serde_yaml::to_string(&config).unwrap(),
        _ => serde_json::to_string(&config).unwrap(),
    };
    std::fs::write(path, content).unwrap();
}

#[test]
#[serial]
fn project_base_urls_cannot_receive_inherited_credentials() {
    let _env = EnvVarGuard::clean_config_env();
    let expected_url = Config::default().model_providers["anthropic"]
        .base_url
        .clone();

    for extension in ["json", "toml", "yaml", "yml"] {
        for explicit in [false, true] {
            for environment_key in [false, true] {
                let dir = tempdir().unwrap();
                let global_dir = dir.path().join("global");
                let mut credentials = CredentialsFile::default();
                credentials.set_api_key("anthropic", "stored-test-key");
                credentials
                    .save(&global_dir.join("credentials.json"))
                    .unwrap();
                unsafe {
                    if environment_key {
                        env::set_var("ANTHROPIC_API_KEY", "environment-test-key");
                    } else {
                        env::remove_var("ANTHROPIC_API_KEY");
                    }
                }
                let path = dir.path().join(format!("sage_config.{extension}"));
                let mut loader = UnifiedConfigLoader::new()
                    .with_working_dir(dir.path())
                    .with_global_dir(&global_dir);
                if explicit {
                    loader = loader.with_config_file(&path);
                }
                for project_key in [None, Some("${ANTHROPIC_API_KEY}"), Some("project-test-key")] {
                    write_project_endpoint(&path, project_key);
                    for config in [loader.load().config, loader.load_strict().unwrap()] {
                        let params = &config.model_providers["anthropic"];
                        assert_eq!(
                            params.base_url, expected_url,
                            "{extension}, explicit={explicit}, env={environment_key}"
                        );
                        let key = params
                            .get_api_key_info_for_provider("anthropic")
                            .key
                            .unwrap();
                        let expected_key = if environment_key {
                            "environment-test-key"
                        } else {
                            project_key
                                .filter(|key| !key.starts_with("${"))
                                .unwrap_or("stored-test-key")
                        };
                        assert_eq!(key, expected_key);
                        let (client, _, _) = crate::llm::LlmClient::from_config(&config).unwrap();
                        assert_eq!(
                            client.config().base_url().map(String::as_str),
                            expected_url.as_deref()
                        );
                    }
                }
            }
        }
    }
}

#[test]
#[serial]
fn project_base_urls_preserve_user_endpoint_overrides() {
    let _env = EnvVarGuard::clean_config_env();
    let dir = tempdir().unwrap();
    let global_dir = dir.path().join("global");
    let path = dir.path().join("sage_config.json");
    write_project_endpoint(&path, None);
    unsafe {
        env::set_var("ANTHROPIC_API_KEY", "environment-test-key");
        env::set_var("ANTHROPIC_BASE_URL", "https://env-endpoint.example.test");
    }
    for explicit in [false, true] {
        let mut loader = UnifiedConfigLoader::new()
            .with_working_dir(dir.path())
            .with_global_dir(&global_dir);
        if explicit {
            loader = loader.with_config_file(&path);
        }
        for config in [loader.load().config, loader.load_strict().unwrap()] {
            assert_eq!(
                config.model_providers["anthropic"].base_url.as_deref(),
                Some("https://env-endpoint.example.test")
            );
        }
        std::fs::create_dir_all(&global_dir).unwrap();
        std::fs::write(
            global_dir.join("config.json"),
            r#"{
            "model_providers": {"anthropic": {
                "model": "claude-user", "base_url": "https://user-endpoint.example.test"
            }}
        }"#,
        )
        .unwrap();
        for config in [loader.load().config, loader.load_strict().unwrap()] {
            assert_eq!(
                config.model_providers["anthropic"].base_url.as_deref(),
                Some("https://user-endpoint.example.test")
            );
        }
        let loader = loader.with_cli_overrides(
            CliOverrides::new()
                .with_provider("anthropic")
                .with_model_base_url("https://cli-endpoint.example.test"),
        );
        for config in [loader.load().config, loader.load_strict().unwrap()] {
            assert_eq!(
                config.model_providers["anthropic"].base_url.as_deref(),
                Some("https://cli-endpoint.example.test")
            );
        }
        std::fs::remove_file(global_dir.join("config.json")).unwrap();
    }
}

#[test]
#[serial]
fn project_base_urls_are_ignored_for_every_provider() {
    let _env = EnvVarGuard::clean_config_env();
    let dir = tempdir().unwrap();
    let path = dir.path().join("sage_config.json");
    let providers = default_providers();
    let model_providers = providers
        .iter()
        .map(|provider| {
            (
                provider.name.clone(),
                serde_json::json!({
                    "model": "test-model", "api_key": "project-test-key",
                    "base_url": "https://project-endpoint.example.test"
                }),
            )
        })
        .collect::<serde_json::Map<String, serde_json::Value>>();
    std::fs::write(
        &path,
        serde_json::to_string(&serde_json::json!({
            "model_providers": model_providers
        }))
        .unwrap(),
    )
    .unwrap();
    let loader = UnifiedConfigLoader::new()
        .with_working_dir(dir.path())
        .with_global_dir(dir.path().join("global"));
    let loaded = loader.load();
    assert_eq!(loaded.warnings.len(), providers.len());
    for mut config in [loaded.config, loader.load_strict().unwrap()] {
        for provider in &providers {
            config.set_default_provider(provider.name.clone()).unwrap();
            let (client, _, _) = crate::llm::LlmClient::from_config(&config).unwrap();
            assert_ne!(
                client.config().get_base_url(),
                "https://project-endpoint.example.test",
                "{} must not use a project endpoint",
                provider.name
            );
        }
    }
}

#[test]
#[serial]
fn project_base_urls_preserve_config_error_contracts() {
    let _env = EnvVarGuard::clean_config_env();
    let dir = tempdir().unwrap();
    let global_dir = dir.path().join("global");
    let path = dir.path().join("sage_config.json");
    write_project_endpoint(&path, None);
    std::fs::create_dir_all(&global_dir).unwrap();
    std::fs::write(global_dir.join("config.json"), "{ invalid json }").unwrap();
    let loader = UnifiedConfigLoader::new()
        .with_config_file(&path)
        .with_working_dir(dir.path())
        .with_global_dir(&global_dir);
    assert!(
        loader
            .load_strict()
            .unwrap_err()
            .to_string()
            .contains("Failed to parse JSON config")
    );
    let loaded = loader.load();
    assert!(
        loaded
            .warnings
            .iter()
            .any(|warning| warning.contains("User provider endpoints could not be loaded"))
    );
    assert_ne!(
        loaded.config.model_providers["anthropic"]
            .base_url
            .as_deref(),
        Some("https://project-endpoint.example.test")
    );

    for user_config in [r#"{"max_steps":"invalid"}"#, "null"] {
        std::fs::write(global_dir.join("config.json"), user_config).unwrap();
        assert!(
            loader
                .load_strict()
                .unwrap_err()
                .to_string()
                .contains("Failed to parse JSON config")
        );
        assert!(
            loader
                .load()
                .warnings
                .iter()
                .any(|warning| warning.contains("User provider endpoints could not be loaded"))
        );
    }

    std::fs::write(&path, "{ invalid json }").unwrap();
    assert!(
        loader
            .load_strict()
            .unwrap_err()
            .to_string()
            .contains("Failed to parse JSON config")
    );
    assert!(!loader.load().warnings.is_empty());
}

#[test]
#[serial]
fn project_base_urls_do_not_trust_relative_global_fallback() {
    let _env = EnvVarGuard::clean_config_env();
    let dir = tempfile::tempdir_in(".").unwrap();
    let global_dir = PathBuf::from(".").join(dir.path().file_name().unwrap());
    let path = global_dir.join("config.json");
    write_project_endpoint(&path, None);
    let loader = UnifiedConfigLoader::new()
        .with_config_file(&path)
        .with_working_dir(dir.path())
        .with_global_dir(&global_dir);
    for config in [loader.load().config, loader.load_strict().unwrap()] {
        assert_eq!(
            config.model_providers["anthropic"].base_url,
            Config::default().model_providers["anthropic"].base_url
        );
    }
}

#[test]
#[serial]
fn project_base_urls_preserve_all_provider_overrides() {
    let _env = EnvVarGuard::clean_config_env();
    for provider in default_providers() {
        let dir = tempdir().unwrap();
        let global_dir = dir.path().join("global");
        let path = dir.path().join("sage_config.json");
        let params = std::collections::HashMap::from([(
            provider.name.clone(),
            serde_json::json!({
                "model": "test-model", "api_key": "project-test-key",
                "base_url": "https://project-endpoint.example.test"
            }),
        )]);
        std::fs::write(
            &path,
            serde_json::to_string(&serde_json::json!({
                "default_provider": provider.name,
                "model_providers": params
            }))
            .unwrap(),
        )
        .unwrap();
        let env_name = format!("{}_BASE_URL", provider.name.to_uppercase());
        unsafe {
            env::set_var(&env_name, "https://env-endpoint.example.test");
        }
        for explicit in [false, true] {
            let mut loader = UnifiedConfigLoader::new()
                .with_working_dir(dir.path())
                .with_global_dir(&global_dir);
            if explicit {
                loader = loader.with_config_file(&path);
            }
            for config in [loader.load().config, loader.load_strict().unwrap()] {
                assert_eq!(
                    config.model_providers[&provider.name].base_url.as_deref(),
                    Some("https://env-endpoint.example.test"),
                    "{}",
                    provider.name
                );
            }
        }
        unsafe {
            env::remove_var(&env_name);
        }

        // CLI can select an alias that the project did not declare.
        std::fs::write(&path, "{}").unwrap();
        std::fs::create_dir_all(&global_dir).unwrap();
        let mut user_params = params;
        user_params.get_mut(&provider.name).unwrap()["base_url"] =
            serde_json::json!("https://user-endpoint.example.test");
        std::fs::write(
            global_dir.join("config.json"),
            serde_json::to_string(&serde_json::json!({
                "model_providers": user_params
            }))
            .unwrap(),
        )
        .unwrap();
        let loader = UnifiedConfigLoader::new()
            .with_config_file(&path)
            .with_working_dir(dir.path())
            .with_global_dir(&global_dir)
            .with_cli_overrides(
                CliOverrides::new()
                    .with_provider(&provider.name)
                    .with_api_key("cli-test-key"),
            );
        for config in [loader.load().config, loader.load_strict().unwrap()] {
            assert_eq!(
                config.model_providers[&provider.name].base_url.as_deref(),
                Some("https://user-endpoint.example.test"),
                "{}",
                provider.name
            );
        }
    }
}

#[cfg(unix)]
#[tokio::test]
#[serial]
async fn mcp_consent_does_not_trust_project_provider_endpoints()
-> Result<(), Box<dyn std::error::Error>> {
    let _env = EnvVarGuard::clean_config_env();
    unsafe {
        env::set_var("ANTHROPIC_API_KEY", "composition-test-key");
    }
    let expected_url = Config::default().model_providers["anthropic"]
        .base_url
        .clone();
    for extension in ["json", "toml", "yaml", "yml"] {
        for explicit in [false, true] {
            let dir = tempdir()?;
            let path = dir.path().join(format!("sage_config.{extension}"));
            let marker = dir.path().join("spawned");
            let value = serde_json::json!({
                "default_provider": "anthropic",
                "model_providers": {"anthropic": {
                    "model": "claude-test", "api_key": "${ANTHROPIC_API_KEY}",
                    "base_url": "https://project-endpoint.example.test"
                }},
                "mcp": {"enabled": true, "default_timeout_secs": 1, "servers": {
                    "workspace": {"transport": "stdio", "command": "/bin/sh",
                        "args": ["-c", "printf spawned > \"$1\"", "sage-mcp-test", marker]}
                }}
            });
            let content = match extension {
                "json" => serde_json::to_string(&value)?,
                "toml" => toml::to_string(&value)?,
                _ => serde_yaml::to_string(&value)?,
            };
            std::fs::write(&path, content)?;
            let mut loader = UnifiedConfigLoader::new()
                .with_working_dir(dir.path())
                .with_global_dir(dir.path().join("global"));
            if explicit {
                loader = loader.with_config_file(&path);
            }
            for config in [loader.load_strict()?, loader.load().config] {
                let params = &config.model_providers["anthropic"];
                assert_eq!(params.base_url, expected_url);
                assert_eq!(
                    params
                        .get_api_key_info_for_provider("anthropic")
                        .key
                        .as_deref(),
                    Some("composition-test-key")
                );
                let (client, _, _) = crate::llm::LlmClient::from_config(&config)?;
                assert_eq!(client.config().base_url(), expected_url.as_ref());
                let registry =
                    crate::mcp::build_mcp_registry_from_config_and_packages(&config, []).await?;
                assert_eq!(config.mcp.enabled, explicit);
                assert_eq!(marker.exists(), explicit);
                assert_eq!(registry.runtime_statuses().is_empty(), !explicit);
                if explicit {
                    std::fs::remove_file(&marker)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
#[serial]
async fn discovered_workspace_mcp_never_spawns_commands() -> Result<(), Box<dyn std::error::Error>>
{
    let _env = EnvVarGuard::clean_config_env();
    for extension in ["json", "toml", "yaml", "yml"] {
        let dir = tempdir()?;
        let marker = dir.path().join("spawned");
        let value = serde_json::json!({
            "default_provider": "ollama",
            "mcp": {
                "enabled": true,
                "default_timeout_secs": 1,
                "servers": {
                    "workspace": {
                        "transport": "stdio", "command": "/bin/sh",
                        "args": ["-c", "printf spawned > \"$1\"", "sage-mcp-test", marker]
                    }
                }
            }
        });
        let content = match extension {
            "json" => serde_json::to_string(&value)?,
            "toml" => toml::to_string(&value)?,
            _ => serde_yaml::to_string(&value)?,
        };
        std::fs::write(dir.path().join(format!("sage_config.{extension}")), content)?;
        let loader = UnifiedConfigLoader::new()
            .with_working_dir(dir.path())
            .with_global_dir(dir.path().join("global"));
        for config in [loader.load_strict()?, loader.load().config] {
            let registry =
                crate::mcp::build_mcp_registry_from_config_and_packages(&config, []).await?;
            assert!(
                !marker.exists(),
                "{extension} workspace command spawned without consent"
            );
            assert!(!config.mcp.enabled);
            assert!(config.mcp.servers.is_empty());
            assert!(registry.runtime_statuses().is_empty());
            assert_eq!(config.default_provider, "ollama");
        }
    }
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
#[serial]
async fn trusted_global_mcp_survives_workspace_overrides() -> Result<(), Box<dyn std::error::Error>>
{
    let _env = EnvVarGuard::clean_config_env();
    let dir = tempdir()?;
    let global_dir = dir.path().join("global");
    std::fs::create_dir(&global_dir)?;
    let marker = dir.path().join("trusted-spawned");
    std::fs::write(
        dir.path().join("sage_config.json"),
        r#"{"default_provider":"ollama","max_steps":73,"mcp":{"enabled":true,"auto_connect":false,
            "warn_on_tool_trust_drift":true,"servers":{"trusted":{"transport":"stdio",
            "command":"__workspace_override__"},"extra":{"transport":"stdio","command":"__workspace_extra__"}}}}"#,
    )?;
    std::fs::write(
        global_dir.join("config.json"),
        serde_json::to_string(&serde_json::json!({
            "mcp": {"enabled": true, "default_timeout_secs": 1, "servers": {
                "trusted": {"transport":"stdio","command":"/bin/sh",
                "args":["-c", "printf spawned > \"$1\"", "sage-mcp-test", marker]}
            }}
        }))?,
    )?;
    let loader = UnifiedConfigLoader::new()
        .with_working_dir(dir.path())
        .with_global_dir(&global_dir);
    let lenient = loader.load();
    assert!(
        lenient
            .warnings
            .iter()
            .any(|w| w.contains("Ignoring MCP settings"))
    );
    for config in [loader.load_strict()?, lenient.config] {
        assert_eq!(config.default_provider, "ollama");
        assert_eq!(config.max_steps, Some(73));
        assert!(config.mcp.enabled);
        assert!(config.mcp.auto_connect);
        assert!(!config.mcp.warn_on_tool_trust_drift);
        assert_eq!(config.mcp.servers.len(), 1);
        let registry = crate::mcp::build_mcp_registry_from_config_and_packages(&config, []).await?;
        assert!(marker.exists(), "trusted user command did not start");
        assert_eq!(
            registry.server_runtime_status("trusted").unwrap().state,
            crate::mcp::McpRuntimeState::ConnectionError
        );
        std::fs::remove_file(&marker)?;
    }
    Ok(())
}

#[test]
#[serial]
fn user_mcp_config_preserves_environment_endpoints() -> Result<(), Box<dyn std::error::Error>> {
    let _env = EnvVarGuard::clean_config_env();
    for provider in default_providers() {
        unsafe {
            env::set_var(
                format!("{}_BASE_URL", provider.name.to_uppercase()),
                "https://env-endpoint.example.test",
            );
        }
    }
    let dir = tempdir()?;
    let global_dir = dir.path().join("global");
    std::fs::create_dir(&global_dir)?;
    let path = dir.path().join("sage_config.json");
    std::fs::write(&path, r#"{"default_provider":"ollama"}"#)?;
    for user_config in [
        r#"{"mcp":{"enabled":false}}"#,
        r#"{"model_providers":{}}"#,
        "[]",
    ] {
        std::fs::write(global_dir.join("config.json"), user_config)?;
        for explicit in [false, true] {
            let mut loader = UnifiedConfigLoader::new()
                .with_working_dir(dir.path())
                .with_global_dir(&global_dir);
            if explicit {
                loader = loader.with_config_file(&path);
            }
            for config in [loader.load().config, loader.load_strict()?] {
                for provider in default_providers() {
                    assert_eq!(
                        config.model_providers[&provider.name].base_url.as_deref(),
                        Some("https://env-endpoint.example.test"),
                        "{}, explicit={explicit}",
                        provider.name
                    );
                }
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
#[serial]
async fn relative_global_mcp_requires_explicit_selection() -> Result<(), Box<dyn std::error::Error>>
{
    let _env = EnvVarGuard::clean_config_env();
    let dir = tempfile::tempdir_in(".")?;
    let global_dir = PathBuf::from(".").join(dir.path().file_name().unwrap());
    let path = global_dir.join("config.json");
    let working_dir = dir.path().canonicalize()?;
    let marker = working_dir.join("spawned");
    std::fs::write(
        &path,
        serde_json::to_string(&serde_json::json!({
            "default_provider":"ollama",
            "mcp":{"enabled":true,"default_timeout_secs":1,"servers":{
                "workspace":{"transport":"stdio","command":"/bin/sh",
                    "args":["-c","printf spawned > \"$1\"","sage-mcp-test",marker]}
            }}
        }))?,
    )?;
    for explicit in [false, true] {
        let mut loader = UnifiedConfigLoader::new()
            .with_working_dir(&working_dir)
            .with_global_dir(&global_dir);
        if explicit {
            loader = loader.with_config_file(&path);
        }
        for config in [loader.load_strict()?, loader.load().config] {
            let registry =
                crate::mcp::build_mcp_registry_from_config_and_packages(&config, []).await?;
            assert_eq!(config.mcp.enabled, explicit);
            assert_eq!(marker.exists(), explicit);
            assert_eq!(registry.runtime_statuses().is_empty(), !explicit);
            if explicit {
                std::fs::remove_file(&marker)?;
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
#[serial]
async fn explicit_config_consent_preserves_startup_and_auto_connect()
-> Result<(), Box<dyn std::error::Error>> {
    let _env = EnvVarGuard::clean_config_env();
    let dir = tempdir()?;
    let path = dir.path().join("sage_config.json");
    let marker = dir.path().join("selected-spawned");
    let loader = UnifiedConfigLoader::new()
        .with_config_file(&path)
        .with_working_dir(dir.path())
        .with_global_dir(dir.path().join("global"));
    for auto_connect in [true, false] {
        std::fs::write(
            &path,
            serde_json::to_string(&serde_json::json!({
                "default_provider":"ollama",
                "mcp":{"enabled":true,"auto_connect":auto_connect,"default_timeout_secs":1,
                    "servers":{"selected":{"transport":"stdio","command":"/bin/sh",
                        "args":["-c", "printf spawned > \"$1\"", "sage-mcp-test", marker]}}}
            }))?,
        )?;
        for config in [loader.load_strict()?, loader.load().config] {
            let registry =
                crate::mcp::build_mcp_registry_from_config_and_packages(&config, []).await?;
            assert_eq!(marker.exists(), auto_connect);
            let expected = if auto_connect {
                std::fs::remove_file(&marker)?;
                crate::mcp::McpRuntimeState::ConnectionError
            } else {
                crate::mcp::McpRuntimeState::Disconnected
            };
            assert_eq!(
                registry.server_runtime_status("selected").unwrap().state,
                expected
            );
        }
    }
    Ok(())
}

#[test]
#[serial]
fn workspace_mcp_cannot_override_user_disable_or_lenient_fallback()
-> Result<(), Box<dyn std::error::Error>> {
    let _env = EnvVarGuard::clean_config_env();
    let dir = tempdir()?;
    let global_dir = dir.path().join("global");
    std::fs::create_dir(&global_dir)?;
    std::fs::write(
        dir.path().join("sage_config.json"),
        r#"{"mcp":{"enabled":true,
        "servers":{"workspace":{"transport":"stdio","command":"__untrusted__"}}}}"#,
    )?;
    std::fs::write(
        global_dir.join("config.json"),
        r#"{"mcp":{"enabled":false}}"#,
    )?;
    let loader = UnifiedConfigLoader::new()
        .with_working_dir(dir.path())
        .with_global_dir(&global_dir);
    for config in [
        loader.load_strict()?,
        loader.load().config,
        loader
            .with_config_file(dir.path().join("missing.json"))
            .load()
            .config,
    ] {
        assert!(!config.mcp.enabled);
        assert!(config.mcp.servers.is_empty());
    }
    std::fs::write(dir.path().join("sage_config.json"), "{invalid")?;
    let loader = UnifiedConfigLoader::new()
        .with_working_dir(dir.path())
        .with_global_dir(&global_dir);
    assert!(loader.load_strict().is_err());
    assert!(!loader.load().warnings.is_empty());
    Ok(())
}
