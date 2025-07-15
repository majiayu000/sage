//! Enhanced console with beautiful UI components

use colored::*;

/// Enhanced console for beautiful terminal UI
pub struct EnhancedConsole;

impl EnhancedConsole {
    /// Print a beautiful welcome banner
    pub fn print_welcome_banner() {
        println!();
        println!("{}", "╭─────────────────────────────────────────────────────────────╮".bright_cyan());
        println!("{}", "│                                                             │".bright_cyan());
        println!("{}", format!("│  🚀 {}                                    │", "Sage Agent - Enhanced Terminal Experience".bright_white().bold()).bright_cyan());
        println!("{}", "│                                                             │".bright_cyan());
        println!("{}", format!("│  {}                                          │", "AI-Powered Software Engineering Assistant".bright_blue()).bright_cyan());
        println!("{}", "│                                                             │".bright_cyan());
        println!("{}", "╰─────────────────────────────────────────────────────────────╯".bright_cyan());
        println!();
    }

    /// Print a beautiful section header with gradient effect
    pub fn print_section_header(title: &str, subtitle: Option<&str>) {
        let title_len = title.chars().count();
        let width = std::cmp::max(60, title_len + 10);
        
        // Top border with rounded corners
        println!("{}", format!("╭{}╮", "─".repeat(width - 2)).bright_blue());
        
        // Title line
        let padding = (width - title_len - 2) / 2;
        let title_line = format!("│{}{title}{}│", 
            " ".repeat(padding), 
            " ".repeat(width - title_len - padding - 2)
        );
        println!("{}", title_line.bright_white().bold());
        
        // Subtitle if provided
        if let Some(sub) = subtitle {
            let sub_len = sub.chars().count();
            let sub_padding = (width - sub_len - 2) / 2;
            let sub_line = format!("│{}{sub}{}│", 
                " ".repeat(sub_padding), 
                " ".repeat(width - sub_len - sub_padding - 2)
            );
            println!("{}", sub_line.bright_cyan());
        }
        
        // Bottom border
        println!("{}", format!("╰{}╯", "─".repeat(width - 2)).bright_blue());
        println!();
    }

    /// Print a beautiful task status with progress
    pub fn print_task_status(task: &str, status: &str, progress: Option<(usize, usize)>) {
        let status_icon = match status {
            "starting" => "🤔",
            "thinking" => "🧠",
            "executing" => "⚡",
            "completed" => "✅",
            "failed" => "❌",
            _ => "ℹ️",
        };

        let status_color = match status {
            "starting" => "bright_yellow",
            "thinking" => "bright_blue", 
            "executing" => "bright_cyan",
            "completed" => "bright_green",
            "failed" => "bright_red",
            _ => "bright_white",
        };

        let mut message = format!("{} {}", status_icon, task);
        
        if let Some((current, total)) = progress {
            let _percentage = (current as f64 / total as f64 * 100.0) as usize;
            message = format!("{} ({}/{})", message, current, total);
        }

        match status_color {
            "bright_yellow" => println!("{}", message.bright_yellow().bold()),
            "bright_blue" => println!("{}", message.bright_blue().bold()),
            "bright_cyan" => println!("{}", message.bright_cyan().bold()),
            "bright_green" => println!("{}", message.bright_green().bold()),
            "bright_red" => println!("{}", message.bright_red().bold()),
            _ => println!("{}", message.bright_white().bold()),
        }
    }

    /// Print a beautiful code block with syntax highlighting hint
    pub fn print_code_block(code: &str, language: &str) {
        let lines: Vec<&str> = code.lines().collect();
        let max_width = lines.iter().map(|line| line.chars().count()).max().unwrap_or(50);
        let width = std::cmp::max(max_width + 4, 50);

        // Header with language
        println!("{}", format!("╭─ {} {}", language.bright_magenta().bold(), "─".repeat(width - language.len() - 4)).bright_black());
        
        // Code lines
        for line in lines {
            println!("{} {}", "│".bright_black(), line);
        }
        
        // Footer
        println!("{}", format!("╰{}", "─".repeat(width - 1)).bright_black());
    }

    /// Print a beautiful result summary
    pub fn print_result_summary(
        success: bool, 
        execution_time: std::time::Duration,
        steps: usize,
        tokens: usize
    ) {
        let status_icon = if success { "✅" } else { "❌" };
        let status_text = if success { "Task Completed Successfully!" } else { "Task Failed!" };
        let status_color = if success { "bright_green" } else { "bright_red" };

        println!();
        println!("{}", "╭─────────────────────────────────────────────────────────────╮".bright_blue());
        println!("{}", "│                                                             │".bright_blue());
        
        let status_line = format!("│  {} {}                                    │", 
            status_icon, status_text);
        match status_color {
            "bright_green" => println!("{}", status_line.bright_green().bold()),
            "bright_red" => println!("{}", status_line.bright_red().bold()),
            _ => println!("{}", status_line.bright_white().bold()),
        }
        
        println!("{}", "│                                                             │".bright_blue());
        
        // Execution details
        let time_line = format!("│  ⏱️  Execution time: {:.2}s                                │", 
            execution_time.as_secs_f64());
        println!("{}", time_line.bright_cyan());
        
        let steps_line = format!("│  📊 Steps: {}                                              │", steps);
        println!("{}", steps_line.bright_cyan());
        
        let tokens_line = format!("│  🔤 Tokens: {}                                            │", tokens);
        println!("{}", tokens_line.bright_cyan());
        
        println!("{}", "│                                                             │".bright_blue());
        println!("{}", "╰─────────────────────────────────────────────────────────────╯".bright_blue());
        println!();
    }

    /// Print a beautiful error message
    pub fn print_error(title: &str, message: &str, suggestion: Option<&str>) {
        println!();
        println!("{}", "╭─ Error ─────────────────────────────────────────────────────╮".bright_red());
        println!("{}", format!("│ ❌ {}                                                │", title).bright_red().bold());
        println!("{}", "│                                                             │".bright_red());
        
        // Wrap message text
        let wrapped_lines = Self::wrap_text(message, 57);
        for line in wrapped_lines {
            println!("{}", format!("│ {}                                                │", line).bright_white());
        }
        
        if let Some(suggestion) = suggestion {
            println!("{}", "│                                                             │".bright_red());
            println!("{}", "│ 💡 Suggestion:                                              │".bright_yellow().bold());
            let suggestion_lines = Self::wrap_text(suggestion, 57);
            for line in suggestion_lines {
                println!("{}", format!("│ {}                                                │", line).bright_yellow());
            }
        }
        
        println!("{}", "│                                                             │".bright_red());
        println!("{}", "╰─────────────────────────────────────────────────────────────╯".bright_red());
        println!();
    }

    /// Helper function to wrap text to specified width
    fn wrap_text(text: &str, width: usize) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in words {
            if current_line.len() + word.len() + 1 <= width {
                if !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(word);
            } else {
                if !current_line.is_empty() {
                    lines.push(current_line);
                    current_line = String::new();
                }
                current_line.push_str(word);
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }

    /// Print a beautiful separator line
    pub fn print_separator() {
        println!("{}", "─".repeat(60).bright_black());
    }

    /// Print a beautiful info box
    pub fn print_info_box(title: &str, items: &[&str]) {
        let max_item_len = items.iter().map(|item| item.chars().count()).max().unwrap_or(20);
        let width = std::cmp::max(max_item_len + 6, title.chars().count() + 6);

        println!("{}", format!("╭─ {} {}", title.bright_blue().bold(), "─".repeat(width - title.len() - 4)).bright_blue());
        
        for item in items {
            println!("{}", format!("│ • {}                                                │", item).bright_white());
        }
        
        println!("{}", format!("╰{}", "─".repeat(width - 1)).bright_blue());
    }
}
