//! Slash command processing for the unified command

use crate::console::CliConsole;
use sage_core::commands::{CommandExecutor, CommandRegistry};
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
            use sage_core::config::{ModelsApiClient, ProviderRegistry, load_config};

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
            let provider_info = registry.get_provider(provider_name).cloned();

            // Get credentials
            let (base_url, api_key) = {
                let mut base_url = provider_info
                    .as_ref()
                    .map(|p| p.api_base_url.clone())
                    .unwrap_or_default();
                let mut api_key = None;

                if let Some(params) = config.model_providers.get(provider_name) {
                    if let Some(url) = &params.base_url {
                        base_url = url.clone();
                    }
                    api_key = params.api_key.clone();
                }

                // Check environment variables
                if api_key.is_none() {
                    let env_var = match provider_name {
                        "anthropic" => "ANTHROPIC_API_KEY",
                        "openai" => "OPENAI_API_KEY",
                        "zai" => "ZAI_API_KEY",
                        "google" => "GOOGLE_API_KEY",
                        "glm" | "zhipu" => "GLM_API_KEY",
                        "moonshot" | "kimi" => "MOONSHOT_API_KEY",
                        _ => "",
                    };
                    if !env_var.is_empty() {
                        api_key = std::env::var(env_var).ok();
                    }
                }

                (base_url, api_key)
            };

            // Fetch models from API
            let client = ModelsApiClient::new();
            let static_models = || -> Vec<String> {
                provider_info
                    .as_ref()
                    .map(|p| p.models.iter().map(|m| m.id.clone()).collect())
                    .unwrap_or_default()
            };
            let mut fallback_warning = None;
            let models: Vec<String> = {
                let mut fallback_on_error = |error: &dyn std::fmt::Display| -> Vec<String> {
                    tracing::warn!(
                        provider = provider_name,
                        reason = model_fetch_fallback_reason(error),
                        "failed to fetch live model list; falling back to static models"
                    );
                    fallback_warning.get_or_insert_with(|| {
                        model_fetch_fallback_warning(
                            provider_name,
                            model_fetch_fallback_reason(error),
                        )
                    });
                    static_models()
                };

                match provider_name {
                    "anthropic" | "glm" | "zhipu" => {
                        match client
                            .fetch_anthropic_models(&base_url, api_key.as_deref().unwrap_or(""))
                            .await
                        {
                            Ok(m) => m.into_iter().map(|m| m.id).collect(),
                            Err(e) => fallback_on_error(&e),
                        }
                    }
                    "openai" | "openrouter" | "zai" | "moonshot" | "kimi" => {
                        match client
                            .fetch_openai_models(&base_url, api_key.as_deref().unwrap_or(""))
                            .await
                        {
                            Ok(m) => m.into_iter().map(|m| m.id).collect(),
                            Err(e) => fallback_on_error(&e),
                        }
                    }
                    "ollama" => match client.fetch_ollama_models(&base_url).await {
                        Ok(m) => m.into_iter().map(|m| m.id).collect(),
                        Err(e) => fallback_on_error(&e),
                    },
                    _ => static_models(),
                }
            };

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

fn model_fetch_fallback_warning(provider_name: &str, reason: &'static str) -> String {
    format!(
        "Failed to fetch live model list for provider '{}' ({reason}); using static model list.",
        provider_name,
    )
}

fn no_models_output(warning: Option<&str>) -> String {
    match warning {
        Some(warning) => format!("{warning}\nNo models available for this provider"),
        None => "No models available for this provider".to_string(),
    }
}

fn model_fetch_fallback_reason(error: &dyn std::fmt::Display) -> &'static str {
    let message = error.to_string().to_ascii_lowercase();
    if message.contains("401")
        || message.contains("403")
        || message.contains("unauthorized")
        || message.contains("forbidden")
        || message.contains("invalid api key")
    {
        "authentication or authorization error"
    } else if message.contains("429") || message.contains("rate limit") {
        "rate limit error"
    } else if message.contains("timeout") || message.contains("timed out") {
        "network timeout"
    } else if message.contains("parse response")
        || message.contains("decode")
        || message.contains("json")
    {
        "response parse error"
    } else if message.contains("failed to fetch")
        || message.contains("connection")
        || message.contains("dns")
        || message.contains("request")
    {
        "network or endpoint error"
    } else {
        "provider request error"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_fetch_fallback_warning_omits_provider_error_text() {
        let raw_error = "API key provided: abcdef1234567890abcdef";
        let warning = model_fetch_fallback_warning(
            "zai",
            model_fetch_fallback_reason(&format!("401 Unauthorized: {raw_error}")),
        );

        assert!(warning.contains("zai"));
        assert!(warning.contains("authentication or authorization error"));
        assert!(warning.contains("using static model list"));
        assert!(!warning.contains(raw_error));
        assert!(!warning.contains("abcdef1234567890abcdef"));
    }

    #[test]
    fn model_fetch_fallback_reason_classifies_safe_error_categories() {
        assert_eq!(
            model_fetch_fallback_reason(&"Failed to fetch OpenAI models: dns error"),
            "network or endpoint error"
        );
        assert_eq!(
            model_fetch_fallback_reason(&"Failed to parse response: expected value"),
            "response parse error"
        );
        assert_eq!(
            model_fetch_fallback_reason(&"429 Too Many Requests"),
            "rate limit error"
        );
    }

    #[test]
    fn no_models_output_preserves_fallback_warning() {
        let warning = "Failed to fetch live model list for provider 'kimi' (network timeout); using static model list.";
        let output = no_models_output(Some(warning));

        assert!(output.contains(warning));
        assert!(output.contains("No models available for this provider"));
    }
}
