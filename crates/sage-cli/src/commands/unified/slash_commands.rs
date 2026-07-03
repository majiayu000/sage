//! Slash command processing for the unified command

use crate::console::CliConsole;
use sage_core::commands::{CommandExecutor, CommandRegistry};
use sage_core::config::{CatalogFreshness, ProviderCatalogSnapshot};
use sage_core::error::SageResult;
use sage_core::output::OutputMode;
use std::sync::Arc;

/// Result of processing a slash command
pub enum SlashCommandAction {
    /// Send this prompt to the LLM
    Prompt(String),
    /// Command was handled locally, no further action needed
    Handled,
    /// Command was handled locally with output to display
    HandledWithOutput(String),
    /// Set output mode
    SetOutputMode(OutputMode),
    /// Resume a session
    Resume { session_id: Option<String> },
    /// Switch model
    SwitchModel { model: String },
    /// Enter model selection mode with available models
    ModelSelect {
        models: Vec<String>,
        warning: Option<String>,
    },
    /// Run diagnostics
    Doctor,
    /// Exit the application
    Exit,
}

/// Process slash commands
pub async fn process_slash_command(
    input: &str,
    console: &CliConsole,
    working_dir: &std::path::Path,
) -> SageResult<SlashCommandAction> {
    if !CommandExecutor::is_command(input) {
        return Ok(SlashCommandAction::Prompt(input.to_string()));
    }

    let mut registry = CommandRegistry::new(working_dir);
    registry.register_builtins();
    if let Err(e) = registry.discover().await {
        console.warn(&format!("Failed to discover commands: {}", e));
    }

    let cmd_executor = CommandExecutor::new(Arc::new(tokio::sync::RwLock::new(registry)));

    match cmd_executor.process(input).await {
        Ok(Some(result)) => {
            // Handle interactive commands (e.g., /resume)
            if let Some(interactive_cmd) = result.interactive {
                return handle_interactive_command_v2(&interactive_cmd, console).await;
            }

            // Handle local commands (output directly, no LLM)
            if result.is_local {
                if let Some(status) = &result.status_message {
                    console.info(status);
                }
                if let Some(output) = &result.local_output {
                    return Ok(SlashCommandAction::HandledWithOutput(output.clone()));
                }
                return Ok(SlashCommandAction::Handled);
            }

            if result.show_expansion {
                console.info(&format!(
                    "Command expanded: {}",
                    &result.expanded_prompt[..result.expanded_prompt.len().min(100)]
                ));
            }
            if let Some(status) = &result.status_message {
                console.info(status);
            }
            Ok(SlashCommandAction::Prompt(result.expanded_prompt))
        }
        Ok(None) => Ok(SlashCommandAction::Prompt(input.to_string())),
        Err(e) => Err(e),
    }
}

/// Handle interactive commands, returning the appropriate action
pub async fn handle_interactive_command_v2(
    cmd: &sage_core::commands::types::InteractiveCommand,
    console: &CliConsole,
) -> SageResult<SlashCommandAction> {
    use sage_core::commands::types::InteractiveCommand;

    match cmd {
        InteractiveCommand::Resume { session_id, .. } => Ok(SlashCommandAction::Resume {
            session_id: session_id.clone(),
        }),
        InteractiveCommand::Title { title } => {
            console.warn(&format!(
                "Title command not available in non-interactive mode. Title: {}",
                title
            ));
            Ok(SlashCommandAction::Handled)
        }
        InteractiveCommand::Login => {
            // Run the login flow directly
            use crate::commands::interactive::CliOnboarding;

            let mut onboarding = CliOnboarding::new();
            match onboarding.run_login().await {
                Ok(true) => {
                    console.success("API key updated! Restart sage to use the new key.");
                }
                Ok(false) => {
                    console.info("API key not changed.");
                }
                Err(e) => {
                    console.error(&format!("Login failed: {}", e));
                }
            }
            Ok(SlashCommandAction::Handled)
        }
        InteractiveCommand::OutputMode { mode } => {
            let output_mode = match mode.as_str() {
                "streaming" => OutputMode::Streaming,
                "batch" => OutputMode::Batch,
                "silent" => OutputMode::Silent,
                _ => {
                    console.warn(&format!("Unknown output mode: {}", mode));
                    return Ok(SlashCommandAction::Handled);
                }
            };
            Ok(SlashCommandAction::SetOutputMode(output_mode))
        }
        InteractiveCommand::Clear => {
            // Clear is handled locally - just acknowledge
            console.info("Conversation cleared.");
            Ok(SlashCommandAction::Handled)
        }
        InteractiveCommand::Logout => {
            console.info("Credentials cleared.");
            Ok(SlashCommandAction::Handled)
        }
        InteractiveCommand::Model { model } => {
            // Return SwitchModel action - the executor will handle the actual switch
            Ok(SlashCommandAction::SwitchModel {
                model: model.clone(),
            })
        }
        InteractiveCommand::ModelSelect => {
            // Fetch models and return them for interactive selection
            use sage_core::config::{
                ProviderRegistry, default_data_dir_or_warn, load_config,
                refresh_provider_model_catalog,
            };

            // Get current provider
            let config = match load_config() {
                Ok(c) => c,
                Err(e) => {
                    return Ok(SlashCommandAction::HandledWithOutput(format!(
                        "Failed to load config: {}",
                        e
                    )));
                }
            };
            let provider_name = config.get_default_provider();

            // Get provider info
            let mut registry = ProviderRegistry::with_defaults();
            let Some(provider_info) = registry.get_provider(provider_name).cloned() else {
                return Ok(SlashCommandAction::HandledWithOutput(no_models_output(
                    None,
                )));
            };

            // Get credentials
            let (base_url, api_key) = {
                let mut base_url = provider_info.api_base_url.clone();
                let mut api_key = None;

                if let Some(params) = config.model_providers.get(provider_name) {
                    if let Some(url) = &params.base_url {
                        base_url = url.clone();
                    }
                    api_key = params.get_api_key_info_for_provider(provider_name).key;
                }

                (base_url, api_key)
            };

            let snapshot = match refresh_provider_model_catalog(
                provider_name,
                &provider_info,
                &base_url,
                api_key.as_deref(),
                default_data_dir_or_warn(),
            )
            .await
            {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    return Ok(SlashCommandAction::HandledWithOutput(format!(
                        "Failed to load model catalog: {error}"
                    )));
                }
            };
            let fallback_warning = model_catalog_warning(provider_name, &snapshot);
            let models: Vec<String> = snapshot
                .provider
                .models
                .iter()
                .map(|model| model.id.clone())
                .collect();

            if models.is_empty() {
                return Ok(SlashCommandAction::HandledWithOutput(no_models_output(
                    fallback_warning.as_deref(),
                )));
            }

            // Return models for interactive selection
            Ok(SlashCommandAction::ModelSelect {
                models,
                warning: fallback_warning,
            })
        }
        InteractiveCommand::Doctor => Ok(SlashCommandAction::Doctor),
        InteractiveCommand::Exit => {
            console.info("Exiting...");
            Ok(SlashCommandAction::Exit)
        }
    }
}

fn no_models_output(warning: Option<&str>) -> String {
    match warning {
        Some(warning) => format!("{warning}\nNo models available for this provider"),
        None => "No models available for this provider".to_string(),
    }
}

fn model_catalog_warning(
    provider_name: &str,
    snapshot: &ProviderCatalogSnapshot,
) -> Option<String> {
    let reason = snapshot
        .last_error
        .as_deref()
        .unwrap_or("live model catalog unavailable");
    match snapshot.freshness {
        CatalogFreshness::StaticFallback => Some(format!(
            "Using static model list for provider '{provider_name}' ({reason})."
        )),
        CatalogFreshness::Stale => Some(format!(
            "Using stale cached model list for provider '{provider_name}' ({reason})."
        )),
        CatalogFreshness::Fresh if snapshot.last_error.is_some() => Some(format!(
            "Fetched live model list for provider '{provider_name}', but cache update failed ({reason})."
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sage_core::config::{CatalogSource, ModelInfo, ProviderInfo, model_catalog_error_reason};

    fn provider_snapshot(
        freshness: CatalogFreshness,
        last_error: Option<&str>,
    ) -> ProviderCatalogSnapshot {
        ProviderCatalogSnapshot {
            provider: ProviderInfo {
                id: "zai".to_string(),
                name: "Z.ai".to_string(),
                description: "Z.ai".to_string(),
                api_base_url: "https://api.z.ai/api/paas/v4".to_string(),
                env_var: "ZAI_API_KEY".to_string(),
                help_url: None,
                requires_api_key: true,
                models: vec![ModelInfo {
                    id: "glm-test".to_string(),
                    name: "GLM Test".to_string(),
                    default: true,
                    context_window: None,
                    max_output_tokens: None,
                }],
            },
            freshness,
            source: CatalogSource::StaticFallback,
            etag: None,
            fetched_at: None,
            ttl_seconds: 86_400,
            last_error: last_error.map(ToString::to_string),
        }
    }

    #[test]
    fn model_catalog_warning_omits_provider_error_text() {
        let raw_error = "API key provided: abcdef1234567890abcdef";
        let Some(warning) = model_catalog_warning(
            "zai",
            &provider_snapshot(
                CatalogFreshness::StaticFallback,
                Some(model_catalog_error_reason(&format!(
                    "401 Unauthorized: {raw_error}"
                ))),
            ),
        ) else {
            panic!("expected static fallback warning");
        };

        assert!(warning.contains("zai"));
        assert!(warning.contains("authentication or authorization error"));
        assert!(warning.contains("Using static model list"));
        assert!(!warning.contains(raw_error));
        assert!(!warning.contains("abcdef1234567890abcdef"));
    }

    #[test]
    fn model_catalog_error_reason_classifies_safe_error_categories() {
        assert_eq!(
            model_catalog_error_reason(&"Failed to fetch OpenAI models: dns error"),
            "network or endpoint error"
        );
        assert_eq!(
            model_catalog_error_reason(&"Failed to parse response: expected value"),
            "response parse error"
        );
        assert_eq!(
            model_catalog_error_reason(&"429 Too Many Requests"),
            "rate limit error"
        );
    }

    #[test]
    fn no_models_output_preserves_fallback_warning() {
        let warning = "Using stale cached model list for provider 'kimi' (network timeout).";
        let output = no_models_output(Some(warning));

        assert!(output.contains(warning));
        assert!(output.contains("No models available for this provider"));
    }
}
