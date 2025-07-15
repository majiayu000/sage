//! Interactive mode implementation

use crate::console::CLIConsole;
use std::io::Write;
use std::path::PathBuf;
use sage_core::error::{SageError, SageResult};
use sage_core::ui::EnhancedConsole;
use sage_sdk::{RunOptions, SageAgentSDK};

/// Arguments for interactive mode
pub struct InteractiveArgs {
    pub config_file: String,
    pub trajectory_file: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
}

/// Execute interactive mode
pub async fn execute(args: InteractiveArgs) -> SageResult<()> {
    let console = CLIConsole::new(true);

    // Use enhanced console for beautiful welcome
    EnhancedConsole::print_welcome_banner();
    EnhancedConsole::print_section_header("Interactive Mode", Some("Type 'help' for available commands, 'exit' to quit"));
    
    // Initialize SDK
    let mut sdk = if std::path::Path::new(&args.config_file).exists() {
        console.info(&format!("Loading configuration from: {}", args.config_file));
        SageAgentSDK::with_config_file(&args.config_file)?
    } else {
        console.warn(&format!("Configuration file not found: {}, using defaults", args.config_file));
        SageAgentSDK::new()?
    };
    
    if let Some(working_dir) = &args.working_dir {
        sdk = sdk.with_working_directory(working_dir);
    }
    
    if let Some(trajectory_file) = &args.trajectory_file {
        sdk = sdk.with_trajectory_path(trajectory_file);
    }
    
    console.success("Interactive mode initialized");
    console.print_separator();
    
    // Main interactive loop
    loop {
        // Ensure we're in a clean state before each iteration
        std::io::stdout().flush().unwrap_or(());
        std::io::stderr().flush().unwrap_or(());

        match console.input("sage") {
            Ok(input) => {
                let input = input.trim();

                // Skip empty input (including backspace artifacts)
                if input.is_empty() {
                    continue;
                }

                // Check for common backspace artifacts
                if input.chars().all(|c| c.is_whitespace() || c.is_control()) {
                    console.warn("检测到输入异常，已清理。请重新输入：");
                    continue;
                }

                match input {
                    "exit" | "quit" | "q" => {
                        console.info("Goodbye!");
                        break;
                    }
                    "help" | "h" => {
                        print_help(&console);
                    }
                    "config" => {
                        print_config(&console, &sdk);
                    }
                    "status" => {
                        print_status(&console, &sdk);
                    }
                    "clear" | "cls" => {
                        // Clear screen and reset display
                        print!("\x1B[2J\x1B[1;1H"); // Clear screen
                        print!("\x1B[3J");          // Clear scrollback buffer
                        console.success("Screen cleared!");
                    }
                    "reset" | "refresh" => {
                        // Force terminal reset to fix display issues
                        print!("\r\x1B[K");         // Clear current line
                        print!("\x1B[2J\x1B[1;1H"); // Clear screen
                        print!("\x1B[3J");          // Clear scrollback
                        console.success("Terminal display reset!");
                    }
                    "input-help" | "ih" => {
                        print_input_help(&console);
                    }
                    _ => {
                        // Execute task with proper error handling
                        match execute_task(&console, &sdk, input).await {
                            Ok(()) => {
                                // Task completed successfully
                            }
                            Err(e) => {
                                console.error(&format!("Task execution failed: {e}"));

                                // Check if this is a critical error that should break the loop
                                if is_critical_error(&e) {
                                    console.error("Critical error encountered. Exiting interactive mode.");
                                    break;
                                }

                                // For non-critical errors, continue the loop
                                console.info("You can try another task or type 'help' for available commands.");
                            }
                        }
                    }
                }
            }
            Err(e) => {
                console.error(&format!("Input error: {e}"));

                // Check if this is EOF or a critical input error
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    console.info("Goodbye!");
                    break;
                }

                // For other input errors, try to continue
                console.warn("Input error occurred. Please try again.");
                continue;
            }
        }

        console.print_separator();
    }
    
    Ok(())
}

/// Check if an error is critical and should terminate the interactive session
fn is_critical_error(error: &SageError) -> bool {
    match error {
        // Configuration errors are critical
        SageError::Config(_) => true,
        // LLM client errors might be temporary, so not critical
        SageError::Llm(_) => false,
        // Tool errors are usually not critical
        SageError::Tool { .. } => false,
        // Agent errors might be critical
        SageError::Agent(_) => false,
        // IO errors might be critical depending on the context
        SageError::Io(_) => false,
        // JSON errors are usually not critical
        SageError::Json(_) => false,
        // HTTP errors are usually temporary
        SageError::Http(_) => false,
        // Invalid input is not critical
        SageError::InvalidInput(_) => false,
        // Timeout is not critical
        SageError::Timeout { .. } => false,
        // Cancelled is not critical
        SageError::Cancelled => false,
        // Other errors are generally not critical
        _ => false,
    }
}

/// Print help information
fn print_help(console: &CLIConsole) {
    console.print_header("Available Commands");
    console.info("help, h          - Show this help message");
    console.info("config           - Show current configuration");
    console.info("status           - Show system status");
    console.info("clear, cls       - Clear the screen");
    console.info("reset, refresh   - Reset terminal display (fixes backspace issues)");
    console.info("input-help, ih   - Show input troubleshooting help");
    console.info("exit, quit, q    - Exit interactive mode");
    console.info("");
    console.info("Any other input will be treated as a task to execute.");
    console.info("Example: 'Create a hello world Python script'");
}

/// Print input troubleshooting help
fn print_input_help(console: &CLIConsole) {
    console.print_header("退格键问题解决方案");

    console.info("如果遇到退格键删除后仍显示字符的问题：");
    console.info("");
    console.info("立即解决方案：");
    console.info("  reset          - 重置终端显示（推荐）");
    console.info("  clear          - 清屏重新开始");
    console.info("  Ctrl+U         - 清除当前行");
    console.info("");
    console.info("常见问题和解决方法：");
    console.info("  • 中文输入残留:    输入 'reset' 重置显示");
    console.info("  • 退格键异常:      切换到英文输入法");
    console.info("  • 字符显示错乱:    使用 Ctrl+U 清除整行");
    console.info("  • 输入法问题:      重启输入法或切换输入法");
    console.info("");
    console.info("预防措施：");
    console.info("  • 输入命令时使用英文输入法");
    console.info("  • 避免在输入过程中频繁切换输入法");
    console.info("  • 使用支持中文较好的终端（如 iTerm2）");
    console.info("");
    console.info("终端快捷键：");
    console.info("  • Ctrl+U         - 清除当前行");
    console.info("  • Ctrl+A         - 移动到行首");
    console.info("  • Ctrl+E         - 移动到行尾");
    console.info("  • Ctrl+C         - 取消当前输入");
}

/// Print current configuration
fn print_config(console: &CLIConsole, sdk: &SageAgentSDK) {
    console.print_header("Current Configuration");
    let config = sdk.config();
    
    console.info(&format!("Provider: {}", config.default_provider));
    
    if let Ok(params) = config.default_model_parameters() {
        console.info(&format!("Model: {}", params.model));
    }
    
    console.info(&format!("Max Steps: {}", config.max_steps));
    
    if let Some(working_dir) = &config.working_directory {
        console.info(&format!("Working Directory: {}", working_dir.display()));
    }
    
    console.info(&format!("Tools Enabled: {}", config.tools.enabled_tools.len()));
}

/// Print system status
fn print_status(console: &CLIConsole, sdk: &SageAgentSDK) {
    console.print_header("Agent Status");

    let config = sdk.config();

    // Show key information like Python version
    console.info(&format!("Provider: {}", config.get_default_provider()));

    if let Ok(params) = config.default_model_parameters() {
        console.info(&format!("Model: {}", params.model));
    }

    console.info(&format!("Available Tools: {}", config.tools.enabled_tools.len()));
    console.info(&format!("Max Steps: {}", config.max_steps));

    // Check configuration validity
    match sdk.validate_config() {
        Ok(()) => console.success("Configuration is valid"),
        Err(e) => console.error(&format!("Configuration error: {e}")),
    }

    // Check API keys
    for (provider, params) in &config.model_providers {
        let has_key = params.get_api_key().is_some();
        let status = if has_key { "✓" } else { "✗" };
        console.info(&format!("{status} {provider}: API key {}", 
            if has_key { "configured" } else { "missing" }
        ));
    }
    
    // Check working directory
    if let Some(working_dir) = &config.working_directory {
        if working_dir.exists() {
            console.success(&format!("Working directory accessible: {}", working_dir.display()));
        } else {
            console.error(&format!("Working directory not found: {}", working_dir.display()));
        }
    } else {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        console.info(&format!("Using current directory: {}", current_dir.display()));
    }
}

/// Execute a task with improved error handling
async fn execute_task(console: &CLIConsole, sdk: &SageAgentSDK, task: &str) -> SageResult<()> {
    console.print_header("Task Execution");
    console.info(&format!("Task: {task}"));

    let start_time = std::time::Instant::now();

    // Show progress indicator
    console.info("🤔 Starting task...");

    // Enable info logging to show progress
    unsafe {
        std::env::set_var("RUST_LOG", "info");
    }

    let run_options = RunOptions::new()
        .with_trajectory(true); // Always enable trajectory in interactive mode

    // Execute task with timeout and error handling
    let result = match tokio::time::timeout(
        std::time::Duration::from_secs(300), // 5 minute timeout
        sdk.run_with_options(task, run_options)
    ).await {
        Ok(result) => result,
        Err(_) => {
            let duration = start_time.elapsed();
            console.error(&format!("Task timed out after {:.2}s", duration.as_secs_f64()));
            return Err(SageError::Timeout { seconds: 300 });
        }
    };

    match result {
        Ok(result) => {
            let duration = start_time.elapsed();

            if result.is_success() {
                console.success("✓ Task completed successfully!");
            } else {
                console.error("✗ Task execution failed!");
            }

            console.info(&format!("ℹ Execution time: {:.2}s", duration.as_secs_f64()));
            console.info(&format!("ℹ Steps: {}", result.execution.steps.len()));
            console.info(&format!("ℹ Tokens: {}", result.execution.total_usage.total_tokens));

            if let Some(final_result) = result.final_result() {
                console.print_header("Result");
                println!("{final_result}");
            }

            if let Some(trajectory_path) = result.trajectory_path() {
                console.info(&format!("ℹ Trajectory saved: {}", trajectory_path.display()));
            }

            Ok(())
        }
        Err(e) => {
            let duration = start_time.elapsed();

            // Provide more helpful error messages
            let error_msg = match &e {
                SageError::Llm(msg) if msg.contains("overloaded") => {
                    "The AI model is currently overloaded. Please try again in a few moments.".to_string()
                }
                SageError::Llm(msg) if msg.contains("rate limit") => {
                    "Rate limit exceeded. Please wait a moment before trying again.".to_string()
                }
                SageError::Http(_) => {
                    "Network error occurred. Please check your internet connection and try again.".to_string()
                }
                _ => format!("Task failed: {e}")
            };

            console.error(&format!("✗ Task execution failed!"));
            console.error(&format!("ℹ Execution time: {:.2}s", duration.as_secs_f64()));
            console.error(&format!("ℹ Error: {error_msg}"));

            Err(e)
        }
    }
}
