//! Reusable TUI components for interactive workflows
//!
//! This module provides styled components for:
//! - Animated spinners for async operations
//! - Styled selectors for provider selection
//! - Progress indicators

use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Spinner styles for different operation types
#[derive(Debug, Clone, Copy)]
pub enum SpinnerStyle {
    /// Default spinner for general operations
    Default,
    /// Spinner for validation operations
    Validation,
    /// Spinner for network operations
    Network,
}

impl SpinnerStyle {
    fn get_template(&self) -> &'static str {
        match self {
            SpinnerStyle::Default => "{spinner:.cyan} {msg}",
            SpinnerStyle::Validation => "{spinner:.yellow} {msg}",
            SpinnerStyle::Network => "{spinner:.blue} {msg}",
        }
    }

    fn get_tick_chars(&self) -> &'static str {
        match self {
            SpinnerStyle::Default => "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏",
            SpinnerStyle::Validation => "◐◓◑◒",
            SpinnerStyle::Network => "▹▸▹▹▹ ▹▹▸▹▹ ▹▹▹▸▹ ▹▹▹▹▸",
        }
    }
}

/// Animated spinner for async operations
pub struct WaitingSpinner {
    spinner: ProgressBar,
    message: String,
}

impl WaitingSpinner {
    /// Create a new waiting spinner with a message
    pub fn new(message: &str) -> Self {
        Self::with_style(message, SpinnerStyle::Default)
    }

    /// Create a spinner with a specific style
    pub fn with_style(message: &str, style: SpinnerStyle) -> Self {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template(style.get_template())
                .unwrap()
                .tick_chars(style.get_tick_chars()),
        );
        spinner.set_message(message.to_string());
        spinner.enable_steady_tick(Duration::from_millis(80));

        Self {
            spinner,
            message: message.to_string(),
        }
    }

    /// Create a spinner for validation operations
    pub fn validation(message: &str) -> Self {
        Self::with_style(message, SpinnerStyle::Validation)
    }

    /// Create a spinner for network operations
    pub fn network(message: &str) -> Self {
        Self::with_style(message, SpinnerStyle::Network)
    }

    /// Update the spinner message
    pub fn set_message(&self, message: &str) {
        self.spinner.set_message(message.to_string());
    }

    /// Tick the spinner (usually not needed with enable_steady_tick)
    pub fn tick(&self) {
        self.spinner.tick();
    }

    /// Finish with a success message
    pub fn finish_success(&self, message: &str) {
        self.spinner.finish_with_message(format!("{} {}", "✓".green(), message));
    }

    /// Finish with a warning message
    pub fn finish_warning(&self, message: &str) {
        self.spinner.finish_with_message(format!("{} {}", "!".yellow(), message));
    }

    /// Finish with an error message
    pub fn finish_error(&self, message: &str) {
        self.spinner.finish_with_message(format!("{} {}", "✗".red(), message));
    }

    /// Finish and clear the spinner
    pub fn finish_and_clear(&self) {
        self.spinner.finish_and_clear();
    }
}

impl Drop for WaitingSpinner {
    fn drop(&mut self) {
        if !self.spinner.is_finished() {
            self.spinner.finish_and_clear();
        }
    }
}

/// Provider selection item with display formatting
#[derive(Debug, Clone)]
pub struct ProviderItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub help_url: Option<String>,
}

impl ProviderItem {
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            help_url: None,
        }
    }

    pub fn with_help_url(mut self, url: &str) -> Self {
        self.help_url = Some(url.to_string());
        self
    }

    /// Format for display in selector
    pub fn display(&self) -> String {
        format!("{} - {}", self.name.cyan(), self.description.dimmed())
    }
}

/// Get help URL for a provider
pub fn get_provider_help_url(provider: &str) -> &'static str {
    match provider.to_lowercase().as_str() {
        "anthropic" => "https://console.anthropic.com/settings/keys",
        "openai" => "https://platform.openai.com/api-keys",
        "google" => "https://aistudio.google.com/apikey",
        "glm" | "zhipu" => "https://open.bigmodel.cn/usercenter/apikeys",
        "azure" => "https://portal.azure.com/#view/Microsoft_Azure_ProjectOxford/CognitiveServicesHub/~/OpenAI",
        "openrouter" => "https://openrouter.ai/keys",
        _ => "the provider's dashboard",
    }
}

/// Get the environment variable name for a provider
pub fn get_provider_env_var(provider: &str) -> String {
    match provider.to_lowercase().as_str() {
        "anthropic" => "ANTHROPIC_API_KEY".to_string(),
        "openai" => "OPENAI_API_KEY".to_string(),
        "google" => "GOOGLE_API_KEY".to_string(),
        "glm" => "GLM_API_KEY".to_string(),
        "zhipu" => "ZHIPU_API_KEY".to_string(),
        "azure" => "AZURE_OPENAI_API_KEY".to_string(),
        "openrouter" => "OPENROUTER_API_KEY".to_string(),
        "ollama" => "OLLAMA_API_KEY".to_string(),
        _ => format!("{}_API_KEY", provider.to_uppercase()),
    }
}

/// Display a styled header
pub fn print_header(title: &str) {
    let width = 55;
    let padding = (width - title.len() - 4) / 2;

    println!();
    println!("{}", "╭".to_string() + &"─".repeat(width) + "╮");
    println!(
        "│{}{}{}│",
        " ".repeat(padding),
        title.bold(),
        " ".repeat(width - padding - title.len())
    );
    println!("{}", "╰".to_string() + &"─".repeat(width) + "╯");
    println!();
}

/// Display a styled box with content
pub fn print_box(lines: &[&str], color: &str) {
    let width = 55;
    let border_color = match color {
        "green" => "╭─╮│╰─╯".green(),
        "cyan" => "╭─╮│╰─╯".cyan(),
        "yellow" => "╭─╮│╰─╯".yellow(),
        "red" => "╭─╮│╰─╯".red(),
        _ => "╭─╮│╰─╯".white(),
    };

    let chars: Vec<char> = border_color.to_string().chars().collect();
    let (tl, h, tr, v, bl, _, br) = (chars[0], chars[1], chars[2], chars[3], chars[4], chars[5], chars[6]);

    // Top border
    print!("{}", format!("{}", tl).color(color));
    for _ in 0..width {
        print!("{}", format!("{}", h).color(color));
    }
    println!("{}", format!("{}", tr).color(color));

    // Content lines
    for line in lines {
        let padding = width - line.chars().count() - 2;
        println!(
            "{} {}{} {}",
            format!("{}", v).color(color),
            line,
            " ".repeat(padding),
            format!("{}", v).color(color)
        );
    }

    // Bottom border
    print!("{}", format!("{}", bl).color(color));
    for _ in 0..width {
        print!("{}", format!("{}", h).color(color));
    }
    println!("{}", format!("{}", br).color(color));
}

/// Tips display for API key input
pub fn print_api_key_tips(provider: &str) {
    let env_var = get_provider_env_var(provider);
    let help_url = get_provider_help_url(provider);

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_item_display() {
        let item = ProviderItem::new("anthropic", "Anthropic", "Claude models");
        let display = item.display();
        assert!(display.contains("Anthropic"));
        assert!(display.contains("Claude models"));
    }

    #[test]
    fn test_get_provider_help_url() {
        assert!(get_provider_help_url("anthropic").contains("anthropic.com"));
        assert!(get_provider_help_url("openai").contains("openai.com"));
        assert!(get_provider_help_url("unknown").contains("dashboard"));
    }

    #[test]
    fn test_get_provider_env_var() {
        assert_eq!(get_provider_env_var("anthropic"), "ANTHROPIC_API_KEY");
        assert_eq!(get_provider_env_var("openai"), "OPENAI_API_KEY");
        assert_eq!(get_provider_env_var("custom"), "CUSTOM_API_KEY");
    }

    #[test]
    fn test_spinner_styles() {
        assert!(!SpinnerStyle::Default.get_template().is_empty());
        assert!(!SpinnerStyle::Validation.get_tick_chars().is_empty());
    }
}
