//! Status command implementation

use super::checks::{get_git_branch, get_git_status_summary, is_git_repository};
use colored::*;
use sage_core::config::load_config_from_file;
use sage_core::error::SageResult;
use std::env;
use std::path::Path;

/// Show current status and environment info
pub async fn status(config_file: &str) -> SageResult<()> {
    println!();
    println!("{}", "Sage Agent Status".bold().underline());
    println!("{}", "=".repeat(50).dimmed());
    println!();

    // Version info
    println!("{}", "Version".cyan().bold());
    println!("  Sage CLI: {}", env!("CARGO_PKG_VERSION").green());
    println!();

    // Environment info
    println!("{}", "Environment".cyan().bold());
    println!(
        "  Working Directory: {}",
        env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
            .cyan()
    );
    println!(
        "  Config File: {}",
        if Path::new(config_file).exists() {
            config_file.green()
        } else {
            format!("{} (not found)", config_file).red()
        }
    );
    println!(
        "  OS: {} {}",
        env::consts::OS.cyan(),
        env::consts::ARCH.cyan()
    );
    println!();

    // Configuration info
    if let Ok(config) = load_config_from_file(config_file) {
        println!("{}", "Configuration".cyan().bold());

        // Default provider
        println!("  Default Provider: {}", config.default_provider.green());

        // Default model (from default provider)
        if let Ok(params) = config.default_model_parameters() {
            println!("  Default Model: {}", params.model.cyan());
        }

        // Max steps
        if let Some(max_steps) = config.max_steps {
            println!("  Max Steps: {}", max_steps.to_string().yellow());
        }

        // Provider count
        let provider_count = config.model_providers.len();
        println!(
            "  Configured Providers: {}",
            provider_count.to_string().cyan()
        );

        println!();
    }

    // Git info
    if is_git_repository() {
        println!("{}", "Git Repository".cyan().bold());
        if let Ok(branch) = get_git_branch().await {
            println!("  Branch: {}", branch.green());
        }
        if let Ok(status) = get_git_status_summary().await {
            println!("  Status: {}", status);
        }
        println!();
    }

    // Environment variables
    println!("{}", "API Keys".cyan().bold());
    let api_keys = [
        ("ANTHROPIC_API_KEY", "Anthropic"),
        ("OPENAI_API_KEY", "OpenAI"),
        ("GOOGLE_API_KEY", "Google"),
        ("DEEPSEEK_API_KEY", "DeepSeek"),
    ];

    for (env_var, name) in api_keys {
        let status = if env::var(env_var).is_ok() {
            "configured".green()
        } else {
            "not set".dimmed()
        };
        println!("  {}: {}", name, status);
    }

    Ok(())
}
