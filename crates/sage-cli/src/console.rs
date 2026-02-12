//! CLI console utilities

use colored::*;

/// CLI console for formatted output
pub struct CliConsole {
    verbose: bool,
}

impl CliConsole {
    /// Create a new CLI console
    pub const fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    /// Print an info message
    pub fn info(&self, message: &str) {
        if self.verbose {
            println!("{} {}", "ℹ".blue().bold(), message);
        }
    }

    /// Print a success message
    pub fn success(&self, message: &str) {
        println!("{} {}", "✓".green().bold(), message.green());
    }

    /// Print a warning message
    pub fn warn(&self, message: &str) {
        println!("{} {}", "⚠".yellow().bold(), message.yellow());
    }

    /// Print an error message
    pub fn error(&self, message: &str) {
        eprintln!("{} {}", "✗".red().bold(), message.red());
    }

    /// Print a header
    pub fn print_header(&self, title: &str) {
        println!();
        println!("{}", title.bold().underline());
        println!("{}", "=".repeat(title.len()).dimmed());
    }

    /// Print a separator
    pub fn print_separator(&self) {
        if self.verbose {
            println!("{}", "-".repeat(50).dimmed());
        }
    }

    /// Print a table header
    pub fn print_table_header(&self, headers: &[&str]) {
        if self.verbose {
            let header_line = headers
                .iter()
                .map(|h| format!("{:15}", h.bold()))
                .collect::<Vec<_>>()
                .join(" | ");

            println!("{header_line}");
            println!("{}", "-".repeat(header_line.len()).dimmed());
        }
    }

    /// Print a table row
    pub fn print_table_row(&self, cells: &[&str]) {
        if self.verbose {
            let row_line = cells
                .iter()
                .map(|c| format!("{:15}", c))
                .collect::<Vec<_>>()
                .join(" | ");

            println!("{row_line}");
        }
    }
}

impl Default for CliConsole {
    fn default() -> Self {
        Self::new(true)
    }
}
