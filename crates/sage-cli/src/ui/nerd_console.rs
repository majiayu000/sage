//! Nerd Font Console - Beautiful CLI with Nerd Font icons
//!
//! A modern terminal UI inspired by tools like Starship, lazygit, and btop

use super::icons::IconProvider;
use colored::*;
use console::Term;
use std::io::{self, Write};

/// Session info for display
pub struct SessionInfo {
    pub title: String,
    pub time_ago: String,
    pub message_count: usize,
}

/// Nerd Font Console for beautiful terminal output
pub struct NerdConsole {
    icons: IconProvider,
    verbose: bool,
}

impl Default for NerdConsole {
    fn default() -> Self {
        Self::new()
    }
}

impl NerdConsole {
    /// Create a new Nerd Console with Nerd Font icons
    pub fn new() -> Self {
        Self {
            icons: IconProvider::new(),
            verbose: true,
        }
    }

    /// Create a console with ASCII fallback icons
    pub fn ascii() -> Self {
        Self {
            icons: IconProvider::ascii(),
            verbose: true,
        }
    }

    /// Set verbose mode
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Get terminal width dynamically
    fn terminal_width() -> usize {
        Term::stdout().size().1 as usize
    }

    // ========== Header / Banner ==========

    /// Print the main header with status line
    pub fn print_header(&self, model: &str, branch: Option<&str>, dir: &str) {
        let width = Self::terminal_width();
        let separator = "━".repeat(width);

        // Top line: sage icon + name
        println!();
        print!("  {} ", self.icons.sage().bright_cyan().bold());
        print!("{}", "sage".bright_white().bold());

        // Status items on the same line
        if let Some(branch) = branch {
            print!("   {} {}", self.icons.git_branch().bright_magenta(), branch.bright_magenta());
        }

        // Truncate dir if too long
        let max_dir_len = (width / 3).max(20);
        let display_dir = Self::truncate_path(dir, max_dir_len);
        print!("   {} {}", self.icons.folder().bright_blue(), display_dir.bright_blue());

        print!("   {} {}", self.icons.model().bright_yellow(), model.bright_yellow());

        println!();
        println!("{}", separator.bright_black());
        println!();
    }

    /// Print a minimal status line (for updates)
    pub fn print_status_line(&self, model: &str, tokens_in: u64, tokens_out: u64) {
        print!("\r\x1B[K"); // Clear line
        print!("  {} {} ", self.icons.model().dimmed(), model.dimmed());
        print!("  {} {} {} {}",
            self.icons.token_in().green(),
            tokens_in.to_string().green(),
            self.icons.token_out().cyan(),
            tokens_out.to_string().cyan()
        );
        io::stdout().flush().ok();
    }

    // ========== Session Display ==========

    /// Print recent sessions in tree format
    pub fn print_sessions_tree(&self, sessions: &[SessionInfo]) {
        if sessions.is_empty() {
            println!("  {} {}", self.icons.info().blue(), "No recent sessions".dimmed());
            return;
        }

        println!("  {} {}", self.icons.history().bright_cyan(), "Recent Sessions".bright_white().bold());
        println!();

        let count = sessions.len().min(5);
        for (i, session) in sessions.iter().take(count).enumerate() {
            let is_last = i == count - 1;
            let prefix = if is_last { "└──" } else { "├──" };

            let title = Self::truncate_str(&session.title, 40);

            println!(
                "  {} {} {} ({}, {} msgs)",
                prefix.bright_black(),
                self.icons.session().bright_blue(),
                title.bright_white(),
                session.time_ago.dimmed(),
                session.message_count.to_string().dimmed()
            );
        }

        if sessions.len() > count {
            println!("      {} {} more sessions...",
                "...".bright_black(),
                (sessions.len() - count).to_string().dimmed()
            );
        }

        println!();
    }

    // ========== Messages ==========

    /// Print an info message
    pub fn info(&self, message: &str) {
        if self.verbose {
            println!("  {} {}", self.icons.info().blue(), message);
        }
    }

    /// Print a success message
    pub fn success(&self, message: &str) {
        println!("  {} {}", self.icons.success().green(), message.green());
    }

    /// Print a warning message
    pub fn warn(&self, message: &str) {
        println!("  {} {}", self.icons.warning().yellow(), message.yellow());
    }

    /// Print an error message
    pub fn error(&self, message: &str) {
        eprintln!("  {} {}", self.icons.error().red(), message.red());
    }

    // ========== Tool Execution ==========

    /// Print tool start
    pub fn print_tool_start(&self, tool_name: &str) {
        let icon = self.get_tool_icon(tool_name);
        println!();
        println!("  {} {} {}",
            icon.bright_magenta(),
            tool_name.bright_magenta().bold(),
            "...".dimmed()
        );
    }

    /// Print tool result
    pub fn print_tool_result(&self, tool_name: &str, success: bool, output: Option<&str>) {
        let status_icon = if success {
            self.icons.success().green()
        } else {
            self.icons.error().red()
        };
        let tool_icon = self.get_tool_icon(tool_name);

        println!("  {} {} {}",
            status_icon,
            tool_icon.bright_magenta(),
            tool_name.bright_magenta()
        );

        if let Some(output) = output {
            // Show first few lines of output
            for line in output.lines().take(5) {
                println!("    {}", line.dimmed());
            }
            let line_count = output.lines().count();
            if line_count > 5 {
                println!("    {} ({} more lines)", "...".dimmed(), line_count - 5);
            }
        }
    }

    /// Get icon for specific tool
    fn get_tool_icon(&self, tool_name: &str) -> ColoredString {
        match tool_name.to_lowercase().as_str() {
            "bash" | "shell" | "execute" => self.icons.terminal().normal(),
            "read" | "cat" => self.icons.file().normal(),
            "write" | "edit" => self.icons.edit().normal(),
            "grep" | "search" | "glob" => self.icons.search().normal(),
            "code" | "lsp" => self.icons.code().normal(),
            _ => self.icons.tool().normal(),
        }
    }

    // ========== Progress / Thinking ==========

    /// Print thinking indicator
    pub fn print_thinking(&self) {
        print!("\r\x1B[K  {} {}",
            self.icons.thinking().bright_yellow(),
            "Thinking...".bright_yellow()
        );
        io::stdout().flush().ok();
    }

    /// Clear thinking indicator
    pub fn clear_thinking(&self) {
        print!("\r\x1B[K");
        io::stdout().flush().ok();
    }

    // ========== Prompt ==========

    /// Print the input prompt
    pub fn print_prompt(&self) {
        print!("\n  {} {} ",
            "sage".bright_cyan().bold(),
            self.icons.prompt().bright_cyan()
        );
        io::stdout().flush().ok();
    }

    /// Print separator line
    pub fn print_separator(&self) {
        let width = Self::terminal_width();
        println!("{}", "━".repeat(width).bright_black());
    }

    // ========== Summary ==========

    /// Print execution summary
    pub fn print_summary(&self, success: bool, steps: usize, tokens_in: u64, tokens_out: u64, duration_secs: f64) {
        println!();
        self.print_separator();
        println!();

        let status = if success {
            format!("{} {}", self.icons.success(), "Completed").green().bold()
        } else {
            format!("{} {}", self.icons.error(), "Failed").red().bold()
        };

        println!("  {}", status);
        println!();
        println!("  {} {} steps   {} {} in / {} out   {} {:.1}s",
            self.icons.lightning().dimmed(),
            steps.to_string().bright_white(),
            self.icons.token_in().dimmed(),
            tokens_in.to_string().bright_white(),
            tokens_out.to_string().bright_white(),
            self.icons.clock().dimmed(),
            duration_secs
        );
        println!();
    }

    // ========== Help ==========

    /// Print quick help
    pub fn print_help(&self) {
        println!();
        println!("  {} {}", self.icons.help().bright_cyan(), "Quick Help".bright_white().bold());
        println!();
        println!("  {}  {}   Clear conversation", "/clear".bright_yellow(), " ".dimmed());
        println!("  {}  {}   Show recent sessions", "/resume".bright_yellow(), "".dimmed());
        println!("  {}  {}   Update API key", "/login".bright_yellow(), " ".dimmed());
        println!("  {} {}   Clear credentials", "/logout".bright_yellow(), "".dimmed());
        println!("  {}   {}   Show help", "/help".bright_yellow(), "  ".dimmed());
        println!("  {}   {}   Exit sage", "/exit".bright_yellow(), "  ".dimmed());
        println!();
        println!("  {} Send   {} Cancel", "⏎".dimmed(), "Esc".dimmed());
        println!();
    }

    // ========== Utilities ==========

    /// Truncate a string (UTF-8 safe)
    fn truncate_str(s: &str, max_chars: usize) -> String {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() > max_chars {
            let truncated: String = chars[..max_chars.saturating_sub(3)].iter().collect();
            format!("{}...", truncated)
        } else {
            s.to_string()
        }
    }

    /// Truncate path for display
    fn truncate_path(path: &str, max_chars: usize) -> String {
        if path.len() <= max_chars {
            return path.to_string();
        }

        // Try to show ~/... format for home directory
        if let Ok(home) = std::env::var("HOME") {
            if path.starts_with(&home) {
                let short = format!("~{}", &path[home.len()..]);
                if short.len() <= max_chars {
                    return short;
                }
            }
        }

        // Just truncate from the start
        let chars: Vec<char> = path.chars().collect();
        if chars.len() > max_chars {
            let start = chars.len() - max_chars + 3;
            let truncated: String = chars[start..].iter().collect();
            format!("...{}", truncated)
        } else {
            path.to_string()
        }
    }
}
