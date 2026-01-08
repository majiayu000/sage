//! Display management for user interface

use super::markdown::render_markdown;
use colored::*;

/// Theme colors for consistent UI styling
pub struct Theme {
    pub primary: &'static str,
    pub secondary: &'static str,
    pub success: &'static str,
    pub warning: &'static str,
    pub error: &'static str,
    pub info: &'static str,
    pub accent: &'static str,
}

impl Theme {
    /// Default modern theme with vibrant colors
    pub const fn default() -> Self {
        Self {
            primary: "bright_blue",
            secondary: "bright_cyan",
            success: "bright_green",
            warning: "bright_yellow",
            error: "bright_red",
            info: "bright_magenta",
            accent: "bright_white",
        }
    }
}

/// Display manager for consistent UI formatting
pub struct DisplayManager {
    _theme: Theme,
}

impl DisplayManager {
    /// Create a new display manager with default theme
    pub fn new() -> Self {
        Self {
            _theme: Theme::default(),
        }
    }

    /// Create a display manager with custom theme
    pub fn with_theme(theme: Theme) -> Self {
        Self { _theme: theme }
    }
    /// Apply theme color to text
    fn apply_color(&self, text: &str, color_name: &str) -> ColoredString {
        match color_name {
            "primary" => text.bright_blue(),
            "secondary" => text.bright_cyan(),
            "success" => text.bright_green(),
            "warning" => text.bright_yellow(),
            "error" => text.bright_red(),
            "info" => text.bright_magenta(),
            "accent" => text.bright_white(),
            "blue" => text.blue(),
            "green" => text.green(),
            "yellow" => text.yellow(),
            "cyan" => text.cyan(),
            "red" => text.red(),
            "magenta" => text.magenta(),
            "white" => text.white(),
            "bright_blue" => text.bright_blue(),
            "bright_green" => text.bright_green(),
            "bright_yellow" => text.bright_yellow(),
            "bright_cyan" => text.bright_cyan(),
            "bright_red" => text.bright_red(),
            "bright_magenta" => text.bright_magenta(),
            "bright_white" => text.bright_white(),
            _ => text.normal(),
        }
    }

    /// Print a modern gradient-style header
    pub fn print_gradient_header(&self, title: &str) {
        let title_len = title.chars().count();
        let total_width = std::cmp::max(60, title_len + 10);

        // Top border with gradient effect
        let top_border = "╭".to_string() + &"─".repeat(total_width - 2) + "╮";
        println!("{}", self.apply_color(&top_border, "primary"));

        // Title line with centered text
        let padding = (total_width - title_len - 2) / 2;
        let title_line = format!(
            "│{}{title}{}│",
            " ".repeat(padding),
            " ".repeat(total_width - title_len - padding - 2)
        );
        println!("{}", self.apply_color(&title_line, "accent").bold());

        // Bottom border
        let bottom_border = "╰".to_string() + &"─".repeat(total_width - 2) + "╯";
        println!("{}", self.apply_color(&bottom_border, "primary"));
        println!();
    }

    /// Print a decorative separator box with modern styling (static version)
    pub fn print_separator(title: &str, color: &str) {
        let display_manager = DisplayManager::new();
        display_manager.print_separator_styled(title, color);
    }

    /// Print a decorative separator box with modern styling (instance method)
    pub fn print_separator_styled(&self, title: &str, color: &str) {
        let title_len = title.chars().count();
        let box_width = std::cmp::max(50, title_len + 4);
        let padding = (box_width - title_len - 2) / 2;

        // Use modern Unicode box drawing characters
        let top_line = "╭".to_string() + &"─".repeat(box_width - 2) + "╮";
        let title_line = format!(
            "│{}{title}{}│",
            " ".repeat(padding),
            " ".repeat(box_width - title_len - padding - 2)
        );
        let bottom_line = "╰".to_string() + &"─".repeat(box_width - 2) + "╯";

        println!("{}", self.apply_color(&top_line, color));
        println!("{}", self.apply_color(&title_line, color).bold());
        println!("{}", self.apply_color(&bottom_line, color));
    }

    /// Print a beautiful status message with icon
    pub fn print_status(&self, icon: &str, message: &str, color: &str) {
        let formatted = format!("{} {}", icon, message);
        println!("{}", self.apply_color(&formatted, color).bold());
    }

    /// Print a beautiful status message with icon (static version)
    pub fn print_status_static(icon: &str, message: &str, color: &str) {
        let display_manager = DisplayManager::new();
        display_manager.print_status(icon, message, color);
    }

    /// Print a progress indicator with percentage
    pub fn print_progress(&self, current: usize, total: usize, message: &str) {
        let percentage = (current as f64 / total as f64 * 100.0) as usize;
        let bar_width = 30;
        let filled = (percentage * bar_width / 100).min(bar_width);
        let empty = bar_width - filled;

        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));

        let progress_line = format!(
            "⚡ {} [{}] {}% ({}/{})",
            message, bar, percentage, current, total
        );
        println!("{}", self.apply_color(&progress_line, "primary"));
    }

    /// Print a timing result with color coding
    pub fn print_timing(operation: &str, duration: std::time::Duration) {
        let seconds = duration.as_secs_f64();
        let timing_color = if seconds < 2.0 {
            "green"
        } else if seconds < 5.0 {
            "yellow"
        } else {
            "red"
        };

        let message = format!("{} completed in {:.2}s", operation, seconds);
        match timing_color {
            "green" => println!("{}", message.green()),
            "yellow" => println!("{}", message.yellow()),
            "red" => println!("{}", message.red()),
            _ => println!("{}", message),
        }
    }

    /// Clear the current line
    pub fn clear_line() {
        print!("\r\x1b[K");
        std::io::Write::flush(&mut std::io::stdout()).unwrap_or(());
    }

    /// Print markdown content with formatting
    pub fn print_markdown(content: &str) {
        let rendered = render_markdown(content);
        println!("{}", rendered);
    }

    /// Print markdown content with custom width
    pub fn print_markdown_with_width(content: &str, width: usize) {
        let rendered = super::markdown::render_markdown_with_width(content, width);
        println!("{}", rendered);
    }

    /// Render markdown and return lines for custom formatting
    pub fn render_markdown_lines(content: &str) -> Vec<String> {
        let rendered = render_markdown(content);
        rendered.lines().map(|s| s.to_string()).collect()
    }
}

impl Default for DisplayManager {
    fn default() -> Self {
        Self::new()
    }
}
