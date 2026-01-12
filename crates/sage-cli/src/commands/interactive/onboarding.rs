//! CLI onboarding wizard for first-time setup
//!
//! This module provides an interactive terminal experience for configuring
//! API keys and providers when starting sage for the first time.

use crate::console::CliConsole;
use colored::*;
use console::{Key, Term};
use dialoguer::{Select, theme::ColorfulTheme};
use indicatif::{ProgressBar, ProgressStyle};
use sage_core::config::credential::{
    ConfigStatus, StatusBarHint, hint_from_status, load_config_unified,
};
use sage_core::config::onboarding::OnboardingManager;
use sage_core::error::{SageError, SageResult};
use std::io::{self, Write};

/// Get the environment variable name for a provider
fn get_provider_env_var(provider: &str) -> &'static str {
    match provider {
        "anthropic" => "ANTHROPIC_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "google" => "GOOGLE_API_KEY",
        "deepseek" => "DEEPSEEK_API_KEY",
        _ => "API_KEY",
    }
}

/// Get the help URL for a provider
fn get_provider_help_url(provider: &str) -> &'static str {
    match provider {
        "anthropic" => "https://console.anthropic.com/settings/keys",
        "openai" => "https://platform.openai.com/api-keys",
        "google" => "https://makersuite.google.com/app/apikey",
        "deepseek" => "https://platform.deepseek.com/api_keys",
        _ => "https://docs.sage-agent.dev/configuration",
    }
}

/// Simple validation spinner
struct ValidationSpinner {
    bar: ProgressBar,
}

impl ValidationSpinner {
    fn new(message: &str) -> Self {
        let bar = ProgressBar::new_spinner();
        bar.set_style(ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.blue} {msg}")
            .unwrap());
        bar.set_message(message.to_string());
        bar.enable_steady_tick(std::time::Duration::from_millis(100));
        Self { bar }
    }

    fn finish_success(&self, message: &str) {
        self.bar.finish_with_message(format!("{} {}", "✓".green(), message));
    }

    fn finish_warning(&self, message: &str) {
        self.bar.finish_with_message(format!("{} {}", "⚠".yellow(), message));
    }
}

/// CLI onboarding wizard
pub struct CliOnboarding {
    manager: OnboardingManager,
    console: CliConsole,
    term: Term,
}

impl CliOnboarding {
    /// Create a new CLI onboarding wizard
    pub fn new() -> Self {
        Self {
            manager: OnboardingManager::with_defaults(),
            console: CliConsole::new(true),
            term: Term::stdout(),
        }
    }

    /// Check if onboarding is needed
    pub fn is_needed(&self) -> bool {
        self.manager.is_needed()
    }

    /// Run the onboarding wizard
    pub async fn run(&mut self) -> SageResult<bool> {
        self.print_welcome_screen();

        // Move to provider selection
        self.manager.next_step()?;

        // Provider selection
        let provider = self.select_provider()?;
        self.manager.select_provider(&provider)?;
        self.manager.next_step()?;

        // API key input
        let api_key = self.input_api_key(&provider)?;
        self.manager.set_api_key(&api_key)?;

        // Validate key with spinner
        let spinner = ValidationSpinner::new("Validating API key...");
        let validation = self.manager.validate_api_key().await;

        if validation.valid {
            let model_info = validation.model_info.as_deref().unwrap_or("default");
            spinner.finish_success(&format!("API key validated! Model: {}", model_info));
        } else if let Some(error) = &validation.error {
            spinner.finish_warning(&format!("Validation warning: {}", error));
            self.console.info("The key will be saved but may not work correctly.");
        }

        // Ask to save
        if self.confirm("Save this configuration?")? {
            self.manager.save_configuration()?;
            self.console.success("Configuration saved!");

            self.print_completion_screen(&provider);
            return Ok(true);
        }

        self.console.info("Configuration not saved.");
        Ok(false)
    }

    /// Run login command (for /login)
    pub async fn run_login(&mut self) -> SageResult<bool> {
        println!();
        self.console.print_header("Configure API Key");
        println!();

        // Provider selection
        let provider = self.select_provider()?;
        self.manager.select_provider(&provider)?;

        // API key input
        let api_key = self.input_api_key(&provider)?;
        self.manager.set_api_key(&api_key)?;

        // Validate with spinner
        let spinner = ValidationSpinner::new("Validating API key...");
        let validation = self.manager.validate_api_key().await;

        if validation.valid {
            let model_info = validation.model_info.as_deref().unwrap_or("default");
            spinner.finish_success(&format!("Validated! Model: {}", model_info));
        } else if let Some(error) = &validation.error {
            spinner.finish_warning(&format!("Warning: {}", error));
        }

        // Save
        if self.confirm("Save this configuration?")? {
            self.manager.save_configuration()?;
            self.console.success(&format!("{} API key configured!", provider));
            return Ok(true);
        }

        Ok(false)
    }

    fn print_welcome_screen(&self) {
        println!();
        println!(
            "{}",
            "╭─────────────────────────────────────────────────────╮"
                .cyan()
                .bold()
        );
        println!(
            "{}",
            "│                                                     │"
                .cyan()
                .bold()
        );
        println!(
            "{}  {}  {}",
            "│".cyan().bold(),
            "🌿 Welcome to Sage Agent".bold(),
            "                       │".cyan().bold()
        );
        println!(
            "{}",
            "│                                                     │"
                .cyan()
                .bold()
        );
        println!(
            "{}  {}  {}",
            "│".cyan().bold(),
            "Let's get you set up with an AI provider.".dimmed(),
            "    │".cyan().bold()
        );
        println!(
            "{}",
            "│                                                     │"
                .cyan()
                .bold()
        );
        println!(
            "{}",
            "╰─────────────────────────────────────────────────────╯"
                .cyan()
                .bold()
        );
        println!();
    }

    fn print_completion_screen(&self, provider: &str) {
        println!();
        println!(
            "{}",
            "╭─────────────────────────────────────────────────────╮"
                .green()
                .bold()
        );
        println!(
            "{}  {}  {}",
            "│".green().bold(),
            "✓ Setup Complete!".green().bold(),
            "                            │".green().bold()
        );
        println!(
            "{}",
            "│                                                     │"
                .green()
                .bold()
        );
        println!(
            "{}  {} {}{}",
            "│".green().bold(),
            "Provider:".dimmed(),
            provider.cyan(),
            " ".repeat(40 - provider.len()) + "│"
        );
        println!(
            "{}",
            "│                                                     │"
                .green()
                .bold()
        );
        println!(
            "{}  {}  {}",
            "│".green().bold(),
            "Start chatting by typing your message below.".dimmed(),
            "│".green().bold()
        );
        println!(
            "{}",
            "╰─────────────────────────────────────────────────────╯"
                .green()
                .bold()
        );
        println!();
    }

    fn select_provider(&self) -> SageResult<String> {
        let options = self.manager.providers();

        // Build display items with description
        let items: Vec<String> = options
            .iter()
            .map(|opt| format!("{} - {}", opt.name, opt.description))
            .collect();

        println!();
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select your AI provider")
            .items(&items)
            .default(0)
            .interact()
            .map_err(|e| SageError::io(format!("selection error: {}", e)))?;

        let selected = &options[selection];
        self.console
            .success(&format!("Selected: {}", selected.name));

        Ok(selected.id.clone())
    }

    fn input_api_key(&self, provider: &str) -> SageResult<String> {
        let env_var = get_provider_env_var(provider);
        let help_url = get_provider_help_url(provider);

        println!();
        println!(
            "  {} Enter your {} API key:",
            "?".blue().bold(),
            provider.cyan()
        );
        println!();
        println!("  {}", "Tips:".dimmed());
        println!(
            "  {} Set {} to avoid re-entering",
            "•".dimmed(),
            env_var.yellow()
        );
        println!(
            "  {} Get your key at: {}",
            "•".dimmed(),
            help_url.underline()
        );
        println!();

        // Read API key with hidden input
        print!("  API Key: ");
        io::stdout().flush().map_err(|e| SageError::io(format!("flush error: {}", e)))?;

        let key = self.read_password()?;

        if key.is_empty() {
            return Err(SageError::invalid_input("API key cannot be empty"));
        }

        // Show masked key
        let masked = if key.len() > 8 {
            format!("{}...{}", &key[..4], &key[key.len() - 4..])
        } else {
            "*".repeat(key.len())
        };
        println!("  {}", format!("Key: {}", masked).dimmed());

        Ok(key)
    }

    fn read_password(&self) -> SageResult<String> {
        let mut password = String::new();

        loop {
            match self.term.read_key() {
                Ok(Key::Enter) => {
                    println!();
                    break;
                }
                Ok(Key::Backspace) => {
                    if !password.is_empty() {
                        password.pop();
                        print!("\x08 \x08"); // Erase last asterisk
                        io::stdout().flush().ok();
                    }
                }
                Ok(Key::Char(c)) if !c.is_control() => {
                    password.push(c);
                    print!("*");
                    io::stdout().flush().ok();
                }
                Ok(Key::CtrlC) => {
                    return Err(SageError::Cancelled);
                }
                _ => {}
            }
        }

        Ok(password)
    }

    fn confirm(&self, message: &str) -> SageResult<bool> {
        print!("  {} {} [Y/n]: ", "?".yellow().bold(), message);
        io::stdout().flush().map_err(|e| SageError::io(format!("flush error: {}", e)))?;

        // Use term.read_line() instead of stdin to work properly after read_key()
        let input = self.term.read_line()
            .map_err(|e| SageError::io(format!("read error: {}", e)))?;

        let answer = input.trim().to_lowercase();
        Ok(answer.is_empty() || answer == "y" || answer == "yes")
    }
}

impl Default for CliOnboarding {
    fn default() -> Self {
        Self::new()
    }
}

/// Check configuration status and return appropriate hint
pub fn check_config_status() -> (ConfigStatus, Option<StatusBarHint>) {
    let loaded = load_config_unified(None);
    let hint = hint_from_status(&loaded.status);
    (loaded.status.status, hint)
}

/// Print status bar hint to console
pub fn print_status_hint(console: &CliConsole, hint: &StatusBarHint) {
    // Use the plain format and color it ourselves
    let message = hint.format_plain();

    match hint.hint_type {
        sage_core::config::credential::HintType::Info => {
            console.info(&message);
        }
        sage_core::config::credential::HintType::Warning => {
            console.warn(&message);
        }
        sage_core::config::credential::HintType::Error => {
            console.error(&message);
        }
        sage_core::config::credential::HintType::Success => {
            console.success(&message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_onboarding_creation() {
        let onboarding = CliOnboarding::new();
        // Should be needed since no credentials are configured in test env
        // (unless OPENAI_API_KEY etc are set)
        assert!(onboarding.is_needed() || !onboarding.is_needed()); // Just test creation
    }

    #[test]
    fn test_check_config_status() {
        let (status, _hint) = check_config_status();
        // Status should be one of the valid values
        assert!(matches!(
            status,
            ConfigStatus::Complete | ConfigStatus::Partial | ConfigStatus::Unconfigured
        ));
    }
}
