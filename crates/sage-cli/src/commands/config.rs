//! Configuration management commands

use crate::console::CLIConsole;
use colored::*;
use sage_core::{
    config::{
        loader::load_config_from_file,
        model::{Config, TrajectoryConfig},
    },
    error::{SageError, SageResult},
};
use std::path::Path;

/// Show current configuration
pub async fn show(config_file: &str) -> SageResult<()> {
    let console = CLIConsole::new(true);

    console.print_header("Configuration");

    if !Path::new(config_file).exists() {
        console.warn(&format!("Configuration file not found: {config_file}"));
        console.info("Using default configuration");

        let config = Config::default();
        print_config(&console, &config);
        return Ok(());
    }

    let config = load_config_from_file(config_file)?;
    console.success(&format!("Loaded configuration from: {config_file}"));

    print_config(&console, &config);
    Ok(())
}

/// Validate configuration
pub async fn validate(config_file: &str) -> SageResult<()> {
    let console = CLIConsole::new(true);

    console.print_header("Configuration Validation");

    if !Path::new(config_file).exists() {
        return Err(SageError::config(format!(
            "Configuration file not found: {config_file}"
        )));
    }

    console.info(&format!("Validating configuration file: {config_file}"));

    match load_config_from_file(config_file) {
        Ok(config) => {
            console.success("Configuration file loaded successfully");

            match config.validate() {
                Ok(()) => {
                    console.success("Configuration is valid");

                    // Print summary
                    console.print_separator();
                    console.info(&format!("Default provider: {}", config.default_provider));
                    let max_steps_display = match config.max_steps {
                        Some(n) => n.to_string(),
                        None => "unlimited".to_string(),
                    };
                    console.info(&format!("Max steps: {}", max_steps_display));
                    console.info(&format!(
                        "Providers configured: {}",
                        config.model_providers.len()
                    ));
                    console.info(&format!(
                        "Tools enabled: {}",
                        config.tools.enabled_tools.len()
                    ));
                }
                Err(e) => {
                    console.error(&format!("Configuration validation failed: {e}"));
                    return Err(e);
                }
            }
        }
        Err(e) => {
            console.error(&format!("Failed to load configuration: {e}"));
            return Err(e);
        }
    }

    Ok(())
}

/// Initialize a new configuration file
pub async fn init(config_file: &str, force: bool) -> SageResult<()> {
    let console = CLIConsole::new(true);

    console.print_header("Configuration Initialization");

    if Path::new(config_file).exists() && !force {
        console.error(&format!("Configuration file already exists: {config_file}"));
        console.info("Use --force to overwrite");
        return Err(SageError::config("Configuration file already exists"));
    }

    let config = create_sample_config();

    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| SageError::config(format!("Failed to serialize configuration: {e}")))?;

    tokio::fs::write(config_file, config_json)
        .await
        .map_err(|e| SageError::config(format!("Failed to write configuration file: {e}")))?;

    console.success(&format!("Created configuration file: {config_file}"));
    console.info("Please edit the file to add your API keys and customize settings");

    Ok(())
}

/// Print configuration details
fn print_config(console: &CLIConsole, config: &Config) {
    console.info(&format!(
        "Default Provider: {}",
        config.default_provider.green()
    ));
    let max_steps_str = match config.max_steps {
        Some(n) => n.to_string(),
        None => "unlimited".to_string(),
    };
    console.info(&format!(
        "Max Steps: {}",
        max_steps_str.yellow()
    ));

    if let Some(working_dir) = &config.working_directory {
        console.info(&format!(
            "Working Directory: {}",
            working_dir.display().to_string().cyan()
        ));
    }

    console.print_separator();
    console.print_header("Model Providers");

    for (name, params) in &config.model_providers {
        console.info(&format!("Provider: {}", name.magenta().bold()));
        console.info(&format!("  Model: {}", params.model));
        console.info(&format!(
            "  API Key: {}",
            if params.api_key.is_some() {
                "✓ Set".green()
            } else {
                "✗ Not set".red()
            }
        ));

        if let Some(base_url) = &params.base_url {
            console.info(&format!("  Base URL: {base_url}"));
        }

        if let Some(max_tokens) = params.max_tokens {
            console.info(&format!("  Max Tokens: {max_tokens}"));
        }

        if let Some(temperature) = params.temperature {
            console.info(&format!("  Temperature: {temperature}"));
        }

        console.print_separator();
    }

    console.print_header("Tools Configuration");
    console.info(&format!(
        "Enabled Tools: {}",
        config.tools.enabled_tools.len()
    ));

    for tool in &config.tools.enabled_tools {
        console.info(&format!("  • {tool}"));
    }

    console.info(&format!(
        "Max Execution Time: {}s",
        config.tools.max_execution_time
    ));
    console.info(&format!(
        "Parallel Execution: {}",
        if config.tools.allow_parallel_execution {
            "✓ Enabled".green()
        } else {
            "✗ Disabled".red()
        }
    ));

    if config.enable_lakeview {
        console.print_separator();
        console.print_header("Lakeview Configuration");
        console.info(&format!("Enabled: {}", "✓ Yes".green()));

        if let Some(lakeview) = &config.lakeview_config {
            console.info(&format!("Model Provider: {}", lakeview.model_provider));
            console.info(&format!("Model Name: {}", lakeview.model_name));
        }
    }
}

/// Create a sample configuration
fn create_sample_config() -> Config {
    use sage_core::config::model::{LoggingConfig, ModelParameters, ToolConfig};
    use std::collections::HashMap;

    let mut model_providers = HashMap::new();

    // OpenAI configuration
    model_providers.insert(
        "openai".to_string(),
        ModelParameters {
            model: "gpt-4".to_string(),
            api_key: Some("your-openai-api-key-here".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            top_p: Some(1.0),
            top_k: None,
            parallel_tool_calls: Some(true),
            max_retries: Some(3),
            base_url: None,
            api_version: None,
            stop_sequences: None,
        },
    );

    // Anthropic configuration
    model_providers.insert(
        "anthropic".to_string(),
        ModelParameters {
            model: "claude-3-sonnet-20240229".to_string(),
            api_key: Some("your-anthropic-api-key-here".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            top_p: Some(1.0),
            top_k: None,
            parallel_tool_calls: Some(false),
            max_retries: Some(3),
            base_url: None,
            api_version: Some("2023-06-01".to_string()),
            stop_sequences: None,
        },
    );

    Config {
        default_provider: "openai".to_string(),
        max_steps: None, // None = unlimited
        total_token_budget: None,
        model_providers,
        lakeview_config: None,
        enable_lakeview: false,
        working_directory: None,
        tools: ToolConfig::default(),
        logging: LoggingConfig::default(),
        trajectory: TrajectoryConfig::default(),
        mcp: sage_core::config::McpConfig::default(),
    }
}
