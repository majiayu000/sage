//! Diagnostic check functions

use super::types::CheckResult;
use colored::*;
use sage_core::config::Config;
use std::env;
use std::path::Path;
use tokio::process::Command;

/// Check if configuration file exists and is valid JSON
pub fn check_config_file(config_file: &str) -> CheckResult {
    let path = Path::new(config_file);

    if !path.exists() {
        return CheckResult::fail("Config File", format!("Not found: {}", config_file))
            .with_hint("Run 'sage config init' to create a configuration file");
    }

    // Try to parse the config
    match std::fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(_) => CheckResult::pass("Config File", format!("Valid: {}", config_file)),
            Err(e) => {
                CheckResult::fail("Config File", format!("Invalid JSON: {}", e))
                    .with_hint("Check the configuration file for syntax errors")
            }
        },
        Err(e) => CheckResult::fail("Config File", format!("Cannot read: {}", e)),
    }
}

/// Check for API key environment variables
pub fn check_environment_variables() -> Vec<CheckResult> {
    let mut results = Vec::new();

    // Check for at least one API key
    let api_keys = [
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
        "GOOGLE_API_KEY",
        "DEEPSEEK_API_KEY",
    ];

    let has_any_key = api_keys.iter().any(|k| env::var(k).is_ok());

    if has_any_key {
        results.push(CheckResult::pass(
            "API Keys",
            "At least one API key is configured",
        ));
    } else {
        results.push(
            CheckResult::warn("API Keys", "No API keys found in environment")
                .with_hint("Set ANTHROPIC_API_KEY or configure in sage_config.json"),
        );
    }

    results
}

/// Check for required external tools (git, etc.)
pub async fn check_required_tools() -> Vec<CheckResult> {
    let mut results = Vec::new();

    // Check for git
    let git_check = tokio::process::Command::new("git")
        .arg("--version")
        .output()
        .await;

    match git_check {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            let version = version.trim().replace("git version ", "");
            results.push(CheckResult::pass("Git", format!("Installed ({})", version)));
        }
        _ => {
            results.push(
                CheckResult::fail("Git", "Not found or not working")
                    .with_hint("Install git for full functionality"),
            );
        }
    }

    results
}

/// Check API configuration from loaded config
pub fn check_api_config(config: &Config) -> Vec<CheckResult> {
    let mut results = Vec::new();

    // Check if any provider is configured
    if config.model_providers.is_empty() {
        results.push(
            CheckResult::warn("Providers", "No providers configured")
                .with_hint("Add at least one provider to sage_config.json"),
        );
    } else {
        results.push(CheckResult::pass(
            "Providers",
            format!("{} provider(s) configured", config.model_providers.len()),
        ));
    }

    // Check default provider
    if config.model_providers.contains_key(&config.default_provider) {
        results.push(CheckResult::pass(
            "Default Provider",
            format!("Set to '{}'", config.default_provider),
        ));
    } else {
        results.push(
            CheckResult::fail(
                "Default Provider",
                format!("'{}' not found in providers", config.default_provider),
            )
            .with_hint("Ensure default_provider matches a configured provider"),
        );
    }

    results
}

/// Check if working directory is accessible
pub fn check_working_directory() -> CheckResult {
    match env::current_dir() {
        Ok(path) => {
            if path.exists() {
                CheckResult::pass("Working Directory", path.display().to_string())
            } else {
                CheckResult::fail("Working Directory", "Does not exist")
            }
        }
        Err(e) => CheckResult::fail("Working Directory", format!("Cannot access: {}", e)),
    }
}

/// Check if current directory is a git repository
pub fn check_git_repository() -> CheckResult {
    if is_git_repository() {
        CheckResult::pass("Git Repository", "Current directory is a git repository")
    } else {
        CheckResult::warn("Git Repository", "Not a git repository")
            .with_hint("Some features work better within a git repository")
    }
}

/// Check if .git directory exists
pub fn is_git_repository() -> bool {
    Path::new(".git").exists()
}

/// Get current git branch name
pub async fn get_git_branch() -> Result<String, std::io::Error> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .await?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to get git branch",
        ))
    }
}

/// Get summary of git working tree status
pub async fn get_git_status_summary() -> Result<String, std::io::Error> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .await?;

    if output.status.success() {
        let status = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<_> = status.lines().collect();

        if lines.is_empty() {
            Ok("clean".green().to_string())
        } else {
            Ok(format!("{} changed file(s)", lines.len()).yellow().to_string())
        }
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to get git status",
        ))
    }
}
