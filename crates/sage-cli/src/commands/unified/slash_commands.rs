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
    ModelSelect { models: Vec<String> },
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
            use sage_core::config::{load_config, ModelsApiClient, ProviderRegistry};

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
                        "google" => "GOOGLE_API_KEY",
                        "glm" | "zhipu" => "GLM_API_KEY",
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
            let models: Vec<String> = match provider_name {
                "anthropic" | "glm" | "zhipu" => {
                    match client
                        .fetch_anthropic_models(&base_url, api_key.as_deref().unwrap_or(""))
                        .await
                    {
                        Ok(m) => m.into_iter().map(|m| m.id).collect(),
                        Err(_) => provider_info
                            .as_ref()
                            .map(|p| p.models.iter().map(|m| m.id.clone()).collect())
                            .unwrap_or_default(),
                    }
                }
                "openai" | "openrouter" => {
                    match client
                        .fetch_openai_models(&base_url, api_key.as_deref().unwrap_or(""))
                        .await
                    {
                        Ok(m) => m.into_iter().map(|m| m.id).collect(),
                        Err(_) => provider_info
                            .as_ref()
                            .map(|p| p.models.iter().map(|m| m.id.clone()).collect())
                            .unwrap_or_default(),
                    }
                }
                "ollama" => match client.fetch_ollama_models(&base_url).await {
                    Ok(m) => m.into_iter().map(|m| m.id).collect(),
                    Err(_) => provider_info
                        .as_ref()
                        .map(|p| p.models.iter().map(|m| m.id.clone()).collect())
                        .unwrap_or_default(),
                },
                _ => provider_info
                    .as_ref()
                    .map(|p| p.models.iter().map(|m| m.id.clone()).collect())
                    .unwrap_or_default(),
            };

            if models.is_empty() {
                return Ok(SlashCommandAction::HandledWithOutput(
                    "No models available for this provider".to_string(),
                ));
            }

            // Return models for interactive selection
            Ok(SlashCommandAction::ModelSelect { models })
        }
        InteractiveCommand::Doctor => Ok(SlashCommandAction::Doctor),
        InteractiveCommand::Exit => {
            console.info("Exiting...");
            Ok(SlashCommandAction::Exit)
        }
    }
}
