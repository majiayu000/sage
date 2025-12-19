//! CLI console utilities

use colored::*;
use console::{Key, Term};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};

/// CLI console for formatted output
pub struct CLIConsole {
    verbose: bool,
    #[allow(dead_code)] // May be used in future features
    progress_bar: Option<ProgressBar>,
}

impl CLIConsole {
    /// Create a new CLI console
    pub const fn new(verbose: bool) -> Self {
        Self {
            verbose,
            progress_bar: None,
        }
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

    /// Start a progress indicator
    #[allow(dead_code)] // May be used in future features
    pub fn start_progress(&mut self, message: &str) {
        if self.verbose {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.blue} {msg}")
                    .expect("Invalid progress template"),
            );
            pb.set_message(message.to_string());
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            self.progress_bar = Some(pb);
        }
    }

    /// Update progress message
    #[allow(dead_code)] // May be used in future features
    pub fn update_progress(&self, message: &str) {
        if let Some(pb) = &self.progress_bar {
            pb.set_message(message.to_string());
        }
    }

    /// Finish progress indicator
    #[allow(dead_code)] // May be used in future features
    pub fn finish_progress(&mut self, final_message: &str) {
        if let Some(pb) = self.progress_bar.take() {
            pb.finish_with_message(final_message.to_string());
        }
    }

    /// Print a step in the execution
    #[allow(dead_code)] // May be used in future features
    pub fn print_step(&self, step_number: u32, description: &str) {
        if self.verbose {
            println!(
                "{} {} {}",
                format!("Step {step_number}:").cyan().bold(),
                "→".dimmed(),
                description
            );
        }
    }

    /// Print tool execution
    #[allow(dead_code)] // May be used in future features
    pub fn print_tool_execution(&self, tool_name: &str, args: &str) {
        if self.verbose {
            println!(
                "  {} {} {}",
                "🔧".to_string(),
                tool_name.magenta().bold(),
                args.dimmed()
            );
        }
    }

    /// Print tool result
    #[allow(dead_code)] // May be used in future features
    pub fn print_tool_result(&self, tool_name: &str, success: bool, output: &str) {
        if self.verbose {
            let status = if success { "✓".green() } else { "✗".red() };

            println!("  {} {} result:", status, tool_name.magenta());

            // Print output with indentation
            for line in output.lines().take(10) {
                // Limit output lines
                println!("    {}", line.dimmed());
            }

            if output.lines().count() > 10 {
                println!("    {} (output truncated)", "...".dimmed());
            }
        }
    }

    /// Ask for user confirmation
    #[allow(dead_code)] // May be used in future features
    pub fn confirm(&self, message: &str) -> io::Result<bool> {
        print!("{} {} [y/N]: ", "?".yellow().bold(), message);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
    }

    /// Get user input with proper Chinese character handling
    pub fn input(&self, prompt: &str) -> io::Result<String> {
        let term = Term::stdout();

        loop {
            // 清除当前行并显示提示符
            print!("\r\x1B[2K{} {}: ", "?".blue().bold(), prompt);
            io::stdout().flush()?;

            let mut input = String::new();

            loop {
                match term.read_key()? {
                    Key::Enter => {
                        println!(); // 换行
                        break;
                    }
                    Key::Backspace => {
                        if !input.is_empty() {
                            // 正确处理中文字符的删除
                            input.pop(); // 删除最后一个字符（正确处理UTF-8）

                            // 重新显示整行以确保正确的视觉效果
                            print!("\r\x1B[2K{} {}: {}", "?".blue().bold(), prompt, input);
                            io::stdout().flush()?;
                        }
                    }
                    Key::Char(c) => {
                        // 处理 Ctrl+U (清除整行)
                        if c == '\u{15}' {
                            // Ctrl+U 的 ASCII 码
                            input.clear();
                            print!("\r\x1B[2K{} {}: ", "?".blue().bold(), prompt);
                            io::stdout().flush()?;
                        } else {
                            input.push(c);
                            print!("{}", c);
                            io::stdout().flush()?;
                        }
                    }
                    Key::CtrlC => {
                        // Let the global signal handler deal with Ctrl+C
                        // Don't return an error here, just ignore the key
                        continue;
                    }
                    _ => {
                        // 忽略其他按键
                    }
                }
            }

            let trimmed = input.trim();

            // Handle special commands
            if trimmed == "clear" || trimmed == "cls" {
                print!("\x1B[2J\x1B[1;1H");
                io::stdout().flush().ok();
                continue;
            }

            if trimmed == "reset" || trimmed == "refresh" {
                print!("\r\x1B[2K\x1B[2J\x1B[1;1H");
                io::stdout().flush().ok();
                continue;
            }

            // 空输入继续循环
            if trimmed.is_empty() {
                continue;
            }

            return Ok(trimmed.to_string());
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

    /// Print JSON in a formatted way
    #[allow(dead_code)] // May be used in future features
    pub fn print_json(&self, json: &serde_json::Value) {
        match serde_json::to_string_pretty(json) {
            Ok(formatted) => println!("{formatted}"),
            Err(_) => println!("{json}"),
        }
    }

    /// Print a code block
    #[allow(dead_code)] // May be used in future features
    pub fn print_code(&self, language: &str, code: &str) {
        println!("```{language}");
        println!("{code}");
        println!("```");
    }

    /// Print execution summary
    #[allow(dead_code)] // May be used in future features
    pub fn print_execution_summary(
        &self,
        success: bool,
        steps: usize,
        tokens: u32,
        duration: std::time::Duration,
    ) {
        self.print_execution_summary_with_cache(success, steps, tokens, None, None, duration);
    }

    /// Print execution summary with cache token info
    #[allow(dead_code)] // May be used in future features
    pub fn print_execution_summary_with_cache(
        &self,
        success: bool,
        steps: usize,
        tokens: u32,
        cache_created: Option<u32>,
        cache_read: Option<u32>,
        duration: std::time::Duration,
    ) {
        self.print_header("Execution Summary");

        let status = if success {
            "SUCCESS".green().bold()
        } else {
            "FAILED".red().bold()
        };

        println!("Status: {status}");
        println!("Steps: {steps}");
        println!("Tokens: {tokens}");

        // Print cache info if available
        let has_cache = cache_created.is_some() || cache_read.is_some();
        if has_cache {
            let mut cache_parts = Vec::new();
            if let Some(created) = cache_created {
                if created > 0 {
                    cache_parts.push(format!("{} created", created.to_string().cyan()));
                }
            }
            if let Some(read) = cache_read {
                if read > 0 {
                    cache_parts.push(format!("{} read", read.to_string().green()));
                }
            }
            if !cache_parts.is_empty() {
                println!("Cache: {}", cache_parts.join(", "));
            }
        }

        println!("Duration: {:.2}s", duration.as_secs_f64());
    }

    /// Clear the current line (for progress updates)
    #[allow(dead_code)] // May be used in future features
    pub fn clear_line(&self) {
        if self.verbose {
            print!("\r\x1b[K");
            io::stdout().flush().ok();
        }
    }
}

impl Default for CLIConsole {
    fn default() -> Self {
        Self::new(true)
    }
}

/// Utility functions for console formatting
pub mod format {
    use colored::*;

    /// Format a file path
    #[allow(dead_code)] // May be used in future features
    pub fn path(path: &std::path::Path) -> String {
        path.display().to_string().cyan().to_string()
    }

    /// Format a duration
    #[allow(dead_code)] // May be used in future features
    pub fn duration(duration: std::time::Duration) -> String {
        format!("{:.2}s", duration.as_secs_f64())
            .yellow()
            .to_string()
    }

    /// Format a number with commas
    #[allow(dead_code)] // May be used in future features
    pub fn number(n: u64) -> String {
        let s = n.to_string();
        let mut result = String::new();

        for (i, c) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                result.push(',');
            }
            result.push(c);
        }

        result.chars().rev().collect::<String>().green().to_string()
    }

    /// Format bytes as human readable
    #[allow(dead_code)] // May be used in future features
    pub fn bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.1} {}", size, UNITS[unit_index])
            .magenta()
            .to_string()
    }
}
