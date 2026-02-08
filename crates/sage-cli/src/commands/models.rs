//! Models listing command

use crate::console::CliConsole;
use colored::*;
use sage_core::{
    config::{load_config_from_file, FetchedModel, ModelInfo, ModelsApiClient, ProviderRegistry},
    error::SageResult,
};
use std::path::Path;

/// List available models for providers
pub async fn list(provider: Option<String>, fetch: bool, config_file: &str) -> SageResult<()> {
    let console = CliConsole::new(true);
    console.print_header("Available Models");

    let mut registry = ProviderRegistry::with_defaults();

    // Load config if exists to get API keys and base URLs
    let config = if Path::new(config_file).exists() {
        Some(load_config_from_file(config_file)?)
    } else {
        None
    };

    let providers: Vec<String> = if let Some(ref p) = provider {
        vec![p.clone()]
    } else {
        registry.provider_ids()
    };

    for provider_name in providers {
        if let Some(info) = registry.get_provider(&provider_name) {
            console.info(&format!("\n{}", provider_name.magenta().bold()));
            console.info(&format!("  {}", info.description));

            if fetch {
                // Try to fetch models dynamically from API
                let (base_url, api_key) =
                    get_provider_credentials(&provider_name, config.as_ref(), &info.api_base_url);

                if api_key.is_some() || !info.requires_api_key {
                    console.info(&format!("  {} Fetching models from API...", "→".cyan()));

                    match fetch_models_for_provider(
                        &provider_name,
                        &base_url,
                        api_key.as_deref().unwrap_or(""),
                    )
                    .await
                    {
                        Ok(models) if !models.is_empty() => {
                            console.success(&format!("  Found {} models:", models.len()));
                            for model in models.iter().take(20) {
                                console.info(&format!(
                                    "    • {} ({})",
                                    model.name.green(),
                                    model.id
                                ));
                            }
                            if models.len() > 20 {
                                console.info(&format!("    ... and {} more", models.len() - 20));
                            }
                        }
                        Ok(_) => {
                            console.warn("  No models returned from API");
                            print_builtin_models(&console, &info.models);
                        }
                        Err(e) => {
                            console.warn(&format!("  Failed to fetch: {}", e));
                            print_builtin_models(&console, &info.models);
                        }
                    }
                } else {
                    console.warn("  No API key configured, showing built-in models");
                    print_builtin_models(&console, &info.models);
                }
            } else {
                print_builtin_models(&console, &info.models);
            }
        } else {
            console.warn(&format!("Unknown provider: {}", provider_name));
        }
    }

    if !fetch {
        console.print_separator();
        console.info("Tip: Use --fetch to get the latest models from provider APIs");
    }

    Ok(())
}

fn print_builtin_models(console: &CliConsole, models: &[ModelInfo]) {
    if models.is_empty() {
        console.info("  No built-in models (user must configure)");
        return;
    }
    console.info("  Built-in models:");
    for model in models {
        let default_marker = if model.default {
            " (default)".yellow()
        } else {
            "".normal()
        };
        console.info(&format!("    • {}{}", model.id.green(), default_marker));
    }
}

fn get_provider_credentials(
    provider_name: &str,
    config: Option<&sage_core::config::Config>,
    default_base_url: &str,
) -> (String, Option<String>) {
    let mut base_url = default_base_url.to_string();
    let mut api_key = None;

    // Check config file
    if let Some(cfg) = config {
        if let Some(params) = cfg.model_providers.get(provider_name) {
            if let Some(url) = &params.base_url {
                base_url = url.clone();
            }
            api_key = params.api_key.clone();
        }
    }

    // Check environment variables if no API key in config
    if api_key.is_none() {
        let env_var = match provider_name {
            "anthropic" => "ANTHROPIC_API_KEY",
            "openai" => "OPENAI_API_KEY",
            "google" => "GOOGLE_API_KEY",
            "glm" | "zhipu" => "GLM_API_KEY",
            "openrouter" => "OPENROUTER_API_KEY",
            "azure" => "AZURE_OPENAI_API_KEY",
            _ => "",
        };
        if !env_var.is_empty() {
            api_key = std::env::var(env_var).ok();
        }
    }

    (base_url, api_key)
}

async fn fetch_models_for_provider(
    provider_name: &str,
    base_url: &str,
    api_key: &str,
) -> SageResult<Vec<FetchedModel>> {
    let client = ModelsApiClient::new();

    match provider_name {
        "anthropic" | "glm" | "zhipu" => client.fetch_anthropic_models(base_url, api_key).await,
        "openai" | "openrouter" => client.fetch_openai_models(base_url, api_key).await,
        "ollama" => client.fetch_ollama_models(base_url).await,
        _ => Ok(vec![]),
    }
}
