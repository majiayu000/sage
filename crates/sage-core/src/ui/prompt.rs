//! User prompt utilities for interactive confirmation
//!
//! Provides functions for asking user confirmation before dangerous operations.

use colored::*;
use std::io::{self, BufRead, Write};

/// Result of a permission prompt
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionChoice {
    /// User approved the operation (one-time)
    YesOnce,
    /// User approved and wants to remember this choice
    YesAlways,
    /// User rejected the operation (one-time)
    NoOnce,
    /// User rejected and wants to always deny this
    NoAlways,
    /// User cancelled (Ctrl+C or Esc)
    Cancelled,
}

impl PermissionChoice {
    /// Check if this choice allows the operation
    pub fn is_allowed(&self) -> bool {
        matches!(self, PermissionChoice::YesOnce | PermissionChoice::YesAlways)
    }

    /// Check if this choice should be remembered
    pub fn should_remember(&self) -> bool {
        matches!(
            self,
            PermissionChoice::YesAlways | PermissionChoice::NoAlways
        )
    }
}

/// Configuration for a permission dialog
#[derive(Debug, Clone)]
pub struct PermissionDialogConfig {
    /// Title of the dialog
    pub title: String,
    /// Tool name that requires permission
    pub tool_name: String,
    /// Description of the operation
    pub operation: String,
    /// Warning message explaining the risk
    pub warning: String,
    /// Whether to show "remember" options
    pub show_remember_options: bool,
}

impl PermissionDialogConfig {
    /// Create a new permission dialog config
    pub fn new(tool_name: &str, operation: &str, warning: &str) -> Self {
        Self {
            title: "Permission Required".to_string(),
            tool_name: tool_name.to_string(),
            operation: operation.to_string(),
            warning: warning.to_string(),
            show_remember_options: true,
        }
    }

    /// Create a simple config without remember options
    pub fn simple(tool_name: &str, operation: &str, warning: &str) -> Self {
        Self {
            title: "Confirm Operation".to_string(),
            tool_name: tool_name.to_string(),
            operation: operation.to_string(),
            warning: warning.to_string(),
            show_remember_options: false,
        }
    }
}

/// Display a permission dialog and wait for user input
///
/// Returns the user's choice or Cancelled if interrupted
pub fn show_permission_dialog(config: &PermissionDialogConfig) -> PermissionChoice {
    let box_width = 60;

    // Print top border
    println!(
        "{}",
        format!("╭{}╮", "─".repeat(box_width - 2)).bright_yellow()
    );

    // Print title
    let title = format!("  ⚠️  {}", config.title);
    let title_padding = box_width - 2 - title.chars().count();
    println!(
        "{}",
        format!("│{}{}│", title, " ".repeat(title_padding))
            .bright_yellow()
            .bold()
    );

    // Print empty line
    println!(
        "{}",
        format!("│{}│", " ".repeat(box_width - 2)).bright_yellow()
    );

    // Print tool name
    let tool_line = format!("  Tool: {}", config.tool_name);
    let tool_padding = box_width - 2 - tool_line.chars().count();
    println!(
        "{}",
        format!("│{}{}│", tool_line, " ".repeat(tool_padding)).bright_yellow()
    );

    // Print operation (may need wrapping)
    let op_prefix = "  Command: ";
    let max_op_len = box_width - 4 - op_prefix.len();
    let operation_display = if config.operation.len() > max_op_len {
        format!("{}...", &config.operation[..max_op_len - 3])
    } else {
        config.operation.clone()
    };
    let op_line = format!("{}{}", op_prefix, operation_display);
    let op_padding = box_width - 2 - op_line.chars().count();
    println!(
        "{}",
        format!("│{}{}│", op_line, " ".repeat(op_padding)).bright_yellow()
    );

    // Print empty line
    println!(
        "{}",
        format!("│{}│", " ".repeat(box_width - 2)).bright_yellow()
    );

    // Print warning in red
    let warning_prefix = "  ";
    let max_warning_len = box_width - 4;
    let warning_display = if config.warning.len() > max_warning_len {
        format!("{}...", &config.warning[..max_warning_len - 3])
    } else {
        config.warning.clone()
    };
    let warning_line = format!("{}{}", warning_prefix, warning_display);
    let warning_padding = box_width - 2 - warning_line.chars().count();
    println!(
        "{}",
        format!(
            "│{}{}│",
            warning_line.bright_red(),
            " ".repeat(warning_padding)
        )
        .bright_yellow()
    );

    // Print empty line
    println!(
        "{}",
        format!("│{}│", " ".repeat(box_width - 2)).bright_yellow()
    );

    // Print options
    if config.show_remember_options {
        let options = [
            ("1", "Yes, execute once"),
            ("2", "Yes, always allow this"),
            ("3", "No, reject"),
            ("4", "No, always deny this"),
        ];
        for (key, desc) in options {
            let opt_line = format!("  [{}] {}", key, desc);
            let opt_padding = box_width - 2 - opt_line.chars().count();
            println!(
                "{}",
                format!("│{}{}│", opt_line, " ".repeat(opt_padding)).bright_yellow()
            );
        }
    } else {
        let options = [("y", "Yes, execute"), ("n", "No, cancel")];
        for (key, desc) in options {
            let opt_line = format!("  [{}] {}", key, desc);
            let opt_padding = box_width - 2 - opt_line.chars().count();
            println!(
                "{}",
                format!("│{}{}│", opt_line, " ".repeat(opt_padding)).bright_yellow()
            );
        }
    }

    // Print empty line
    println!(
        "{}",
        format!("│{}│", " ".repeat(box_width - 2)).bright_yellow()
    );

    // Print hint
    let hint = if config.show_remember_options {
        "  Enter 1-4 to choose · Ctrl+C to cancel"
    } else {
        "  Enter y/n · Ctrl+C to cancel"
    };
    let hint_padding = box_width - 2 - hint.chars().count();
    println!(
        "{}",
        format!("│{}{}│", hint.dimmed(), " ".repeat(hint_padding)).bright_yellow()
    );

    // Print bottom border
    println!(
        "{}",
        format!("╰{}╯", "─".repeat(box_width - 2)).bright_yellow()
    );

    // Get user input
    print!("{}", "  Choice: ".bright_cyan());
    io::stdout().flush().unwrap_or(());

    let stdin = io::stdin();
    let mut input = String::new();

    match stdin.lock().read_line(&mut input) {
        Ok(_) => {
            let input = input.trim().to_lowercase();

            if config.show_remember_options {
                match input.as_str() {
                    "1" | "y" | "yes" => PermissionChoice::YesOnce,
                    "2" => PermissionChoice::YesAlways,
                    "3" | "n" | "no" => PermissionChoice::NoOnce,
                    "4" => PermissionChoice::NoAlways,
                    "" => PermissionChoice::Cancelled, // Empty input = cancel
                    _ => {
                        println!("{}", "  Invalid choice, operation cancelled.".red());
                        PermissionChoice::Cancelled
                    }
                }
            } else {
                match input.as_str() {
                    "y" | "yes" | "1" => PermissionChoice::YesOnce,
                    "n" | "no" | "2" | "" => PermissionChoice::NoOnce,
                    _ => {
                        println!("{}", "  Invalid choice, operation cancelled.".red());
                        PermissionChoice::Cancelled
                    }
                }
            }
        }
        Err(_) => PermissionChoice::Cancelled,
    }
}

/// Simple yes/no confirmation prompt
///
/// Returns true if user confirms, false otherwise
pub fn confirm(message: &str) -> bool {
    let box_width = 50;

    // Print simple dialog
    println!(
        "{}",
        format!("╭{}╮", "─".repeat(box_width - 2)).bright_cyan()
    );

    let msg_line = format!("  {}", message);
    let msg_padding = box_width - 2 - msg_line.chars().count().min(box_width - 4);
    println!(
        "{}",
        format!(
            "│{}{}│",
            &msg_line[..msg_line.len().min(box_width - 4)],
            " ".repeat(msg_padding)
        )
        .bright_cyan()
    );

    println!(
        "{}",
        format!("╰{}╯", "─".repeat(box_width - 2)).bright_cyan()
    );

    print!("{}", "  [y/n]: ".bright_cyan());
    io::stdout().flush().unwrap_or(());

    let stdin = io::stdin();
    let mut input = String::new();

    match stdin.lock().read_line(&mut input) {
        Ok(_) => {
            let input = input.trim().to_lowercase();
            matches!(input.as_str(), "y" | "yes")
        }
        Err(_) => false,
    }
}

/// Print a success message with border
pub fn print_success(message: &str) {
    println!(
        "{}",
        format!("✅ {}", message).bright_green().bold()
    );
}

/// Print an error message with border
pub fn print_error(message: &str) {
    println!("{}", format!("❌ {}", message).bright_red().bold());
}

/// Print a warning message with border
pub fn print_warning(message: &str) {
    println!(
        "{}",
        format!("⚠️  {}", message).bright_yellow().bold()
    );
}

/// Print an info message
pub fn print_info(message: &str) {
    println!("{}", format!("ℹ️  {}", message).bright_blue());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_choice_is_allowed() {
        assert!(PermissionChoice::YesOnce.is_allowed());
        assert!(PermissionChoice::YesAlways.is_allowed());
        assert!(!PermissionChoice::NoOnce.is_allowed());
        assert!(!PermissionChoice::NoAlways.is_allowed());
        assert!(!PermissionChoice::Cancelled.is_allowed());
    }

    #[test]
    fn test_permission_choice_should_remember() {
        assert!(!PermissionChoice::YesOnce.should_remember());
        assert!(PermissionChoice::YesAlways.should_remember());
        assert!(!PermissionChoice::NoOnce.should_remember());
        assert!(PermissionChoice::NoAlways.should_remember());
        assert!(!PermissionChoice::Cancelled.should_remember());
    }

    #[test]
    fn test_permission_dialog_config_new() {
        let config = PermissionDialogConfig::new("bash", "rm -rf ./build", "This will delete files");
        assert_eq!(config.tool_name, "bash");
        assert_eq!(config.operation, "rm -rf ./build");
        assert!(config.show_remember_options);
    }

    #[test]
    fn test_permission_dialog_config_simple() {
        let config = PermissionDialogConfig::simple("bash", "ls", "List files");
        assert!(!config.show_remember_options);
    }
}
