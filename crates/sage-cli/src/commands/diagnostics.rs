//! Diagnostic commands (doctor, status, usage)
//!
//! Provides system health checks and usage statistics similar to Claude Code's
//! `/doctor` command.

use crate::console::CliConsole;
use colored::*;
use sage_core::config::{load_config_from_file, Config};
use sage_core::error::SageResult;
use std::env;
use std::path::Path;

/// Check item result for diagnostics
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    pub hint: Option<String>,
}

/// Status of a diagnostic check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

impl CheckResult {
    fn pass(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Pass,
            message: message.into(),
            hint: None,
        }
    }

    fn warn(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Warn,
            message: message.into(),
            hint: None,
        }
    }

    fn fail(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Fail,
            message: message.into(),
            hint: None,
        }
    }

    fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    fn icon(&self) -> ColoredString {
        match self.status {
            CheckStatus::Pass => "✓".green().bold(),
            CheckStatus::Warn => "⚠".yellow().bold(),
            CheckStatus::Fail => "✗".red().bold(),
        }
    }
}

/// Run system health checks (doctor command)
pub async fn doctor(config_file: &str) -> SageResult<()> {
    let console = CliConsole::new(true);

    println!();
    println!("{}", "Sage Agent Health Check".bold().underline());
    println!("{}", "=".repeat(50).dimmed());
    println!();

    let mut checks = Vec::new();

    // 1. Check configuration file
    checks.push(check_config_file(config_file));

    // 2. Check environment variables
    checks.extend(check_environment_variables());

    // 3. Check required tools
    checks.extend(check_required_tools().await);

    // 4. Check API connectivity (if config loaded)
    if let Ok(config) = load_config_from_file(config_file) {
        checks.extend(check_api_config(&config));
    }

    // 5. Check working directory
    checks.push(check_working_directory());

    // 6. Check Git repository
    checks.push(check_git_repository());

    // Print results
    let mut pass_count = 0;
    let mut warn_count = 0;
    let mut fail_count = 0;

    for check in &checks {
        println!("{} {} - {}", check.icon(), check.name.bold(), check.message);

        if let Some(hint) = &check.hint {
            println!("    {} {}", "→".dimmed(), hint.dimmed());
        }

        match check.status {
            CheckStatus::Pass => pass_count += 1,
            CheckStatus::Warn => warn_count += 1,
            CheckStatus::Fail => fail_count += 1,
        }
    }

    // Summary
    println!();
    println!("{}", "-".repeat(50).dimmed());
    println!(
        "Summary: {} passed, {} warnings, {} failed",
        pass_count.to_string().green(),
        warn_count.to_string().yellow(),
        fail_count.to_string().red()
    );

    if fail_count > 0 {
        println!();
        console.error("Some checks failed. Please fix the issues above.");
    } else if warn_count > 0 {
        println!();
        console.warn("Some checks have warnings. Consider addressing them.");
    } else {
        println!();
        console.success("All checks passed! Sage Agent is ready to use.");
    }

    Ok(())
}

/// Show current status and environment info
pub async fn status(config_file: &str) -> SageResult<()> {
    println!();
    println!("{}", "Sage Agent Status".bold().underline());
    println!("{}", "=".repeat(50).dimmed());
    println!();

    // Version info
    println!("{}", "Version".cyan().bold());
    println!(
        "  Sage CLI: {}",
        env!("CARGO_PKG_VERSION").green()
    );
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
        println!("  Configured Providers: {}", provider_count.to_string().cyan());

        println!();
    }

    // Git info
    if is_git_repository() {
        println!("{}", "Git Repository".cyan().bold());
        if let Ok(branch) = get_git_branch() {
            println!("  Branch: {}", branch.green());
        }
        if let Ok(status) = get_git_status_summary() {
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

/// Show token usage statistics
pub async fn usage(session_dir: Option<&Path>, detailed: bool) -> SageResult<()> {
    let console = CliConsole::new(true);

    println!();
    println!("{}", "Token Usage Statistics".bold().underline());
    println!("{}", "=".repeat(50).dimmed());
    println!();

    // Determine session directory
    let dir = session_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| Path::new("trajectories").to_path_buf());

    if !dir.exists() {
        console.warn(&format!(
            "Session directory not found: {}",
            dir.display()
        ));
        console.info("Run some tasks first to generate usage data.");
        return Ok(());
    }

    // Collect trajectory files
    let entries: Vec<_> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "json" || ext == "jsonl")
        })
        .collect();

    if entries.is_empty() {
        console.warn("No session files found.");
        console.info("Run some tasks first to generate usage data.");
        return Ok(());
    }

    let mut total_prompt_tokens: u64 = 0;
    let mut total_completion_tokens: u64 = 0;
    let mut total_cache_read_tokens: u64 = 0;
    let mut total_cache_created_tokens: u64 = 0;
    let mut session_count = 0;

    // Process each trajectory file
    for entry in &entries {
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            // Try to extract usage data from the file
            if let Some(usage) = extract_usage_from_content(&content) {
                total_prompt_tokens += usage.prompt_tokens;
                total_completion_tokens += usage.completion_tokens;
                total_cache_read_tokens += usage.cache_read_tokens;
                total_cache_created_tokens += usage.cache_created_tokens;
                session_count += 1;

                if detailed {
                    println!(
                        "  {} - {} prompt, {} completion",
                        entry.path().file_name().unwrap_or_default().to_string_lossy().cyan(),
                        format_number(usage.prompt_tokens),
                        format_number(usage.completion_tokens)
                    );
                }
            }
        }
    }

    if detailed && session_count > 0 {
        println!();
    }

    // Print summary
    println!("{}", "Summary".cyan().bold());
    println!("  Sessions Analyzed: {}", session_count.to_string().cyan());
    println!(
        "  Total Prompt Tokens: {}",
        format_number(total_prompt_tokens).green()
    );
    println!(
        "  Total Completion Tokens: {}",
        format_number(total_completion_tokens).green()
    );
    println!(
        "  Total Tokens: {}",
        format_number(total_prompt_tokens + total_completion_tokens).yellow().bold()
    );

    if total_cache_read_tokens > 0 || total_cache_created_tokens > 0 {
        println!();
        println!("{}", "Cache Statistics".cyan().bold());
        println!(
            "  Cache Read Tokens: {}",
            format_number(total_cache_read_tokens).green()
        );
        println!(
            "  Cache Created Tokens: {}",
            format_number(total_cache_created_tokens).cyan()
        );

        // Calculate savings percentage
        if total_prompt_tokens > 0 {
            let savings_pct =
                (total_cache_read_tokens as f64 / total_prompt_tokens as f64) * 100.0;
            println!(
                "  Estimated Savings: {:.1}%",
                savings_pct
            );
        }
    }

    Ok(())
}

// Helper functions

fn check_config_file(config_file: &str) -> CheckResult {
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

fn check_environment_variables() -> Vec<CheckResult> {
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

async fn check_required_tools() -> Vec<CheckResult> {
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

fn check_api_config(config: &Config) -> Vec<CheckResult> {
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

fn check_working_directory() -> CheckResult {
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

fn check_git_repository() -> CheckResult {
    if is_git_repository() {
        CheckResult::pass("Git Repository", "Current directory is a git repository")
    } else {
        CheckResult::warn("Git Repository", "Not a git repository")
            .with_hint("Some features work better within a git repository")
    }
}

fn is_git_repository() -> bool {
    Path::new(".git").exists()
}

fn get_git_branch() -> Result<String, std::io::Error> {
    let output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to get git branch",
        ))
    }
}

fn get_git_status_summary() -> Result<String, std::io::Error> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()?;

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

/// Usage data extracted from a session file
struct UsageData {
    prompt_tokens: u64,
    completion_tokens: u64,
    cache_read_tokens: u64,
    cache_created_tokens: u64,
}

fn extract_usage_from_content(content: &str) -> Option<UsageData> {
    // Try to parse as JSON and extract usage information
    // This handles both single JSON objects and JSONL format

    let mut total = UsageData {
        prompt_tokens: 0,
        completion_tokens: 0,
        cache_read_tokens: 0,
        cache_created_tokens: 0,
    };

    let mut found_any = false;

    // Try each line as potentially valid JSON
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(usage) = extract_usage_from_json(&value) {
                total.prompt_tokens += usage.prompt_tokens;
                total.completion_tokens += usage.completion_tokens;
                total.cache_read_tokens += usage.cache_read_tokens;
                total.cache_created_tokens += usage.cache_created_tokens;
                found_any = true;
            }
        }
    }

    // Also try the entire content as a single JSON object
    if !found_any {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(usage) = extract_usage_from_json(&value) {
                return Some(usage);
            }
        }
    }

    if found_any {
        Some(total)
    } else {
        None
    }
}

fn extract_usage_from_json(value: &serde_json::Value) -> Option<UsageData> {
    // Look for usage data in common locations
    let usage = value.get("usage").or_else(|| value.get("token_usage"))?;

    Some(UsageData {
        prompt_tokens: usage
            .get("prompt_tokens")
            .or_else(|| usage.get("input_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        completion_tokens: usage
            .get("completion_tokens")
            .or_else(|| usage.get("output_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        cache_read_tokens: usage
            .get("cache_read_input_tokens")
            .or_else(|| usage.get("cache_read_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        cache_created_tokens: usage
            .get("cache_creation_input_tokens")
            .or_else(|| usage.get("cache_created_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    })
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();

    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }

    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(100), "100");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1000000), "1,000,000");
    }

    #[test]
    fn test_check_result_icons() {
        let pass = CheckResult::pass("test", "message");
        let warn = CheckResult::warn("test", "message");
        let fail = CheckResult::fail("test", "message");

        assert_eq!(pass.status, CheckStatus::Pass);
        assert_eq!(warn.status, CheckStatus::Warn);
        assert_eq!(fail.status, CheckStatus::Fail);
    }

    #[test]
    fn test_extract_usage_from_json() {
        let json = serde_json::json!({
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "cache_read_input_tokens": 20
            }
        });

        let usage = extract_usage_from_json(&json).unwrap();
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.cache_read_tokens, 20);
    }

    #[test]
    fn test_extract_usage_from_content_jsonl() {
        let content = r#"
{"usage": {"prompt_tokens": 100, "completion_tokens": 50}}
{"usage": {"prompt_tokens": 200, "completion_tokens": 100}}
"#;

        let usage = extract_usage_from_content(content).unwrap();
        assert_eq!(usage.prompt_tokens, 300);
        assert_eq!(usage.completion_tokens, 150);
    }
}
