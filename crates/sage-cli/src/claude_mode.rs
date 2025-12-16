//! Claude Code Mode for Sage CLI
//!
//! This module implements the Claude Code execution flow for the Sage CLI,
//! providing a lightweight, responsive user experience.

use sage_core::{Config, ReactiveExecutionManager, ReactiveResponse, SageResult};
use std::io::{self, Write};
use std::time::Instant;

/// Claude Code style CLI mode
pub struct ClaudeMode {
    execution_manager: ReactiveExecutionManager,
    interactive: bool,
}

impl ClaudeMode {
    /// Create a new Claude Code mode CLI
    pub fn new(config: Config, interactive: bool) -> SageResult<Self> {
        let execution_manager = ReactiveExecutionManager::new(config)?;

        Ok(Self {
            execution_manager,
            interactive,
        })
    }

    /// Execute a single command in Claude Code style
    pub async fn execute_command(&mut self, command: &str) -> SageResult<()> {
        let start_time = Instant::now();
        print!("🤖 Thinking... ");
        io::stdout().flush().unwrap();

        let _response = match self.execution_manager.interactive_mode(command).await {
            Ok(response) => {
                print!("\r              \r"); // Clear thinking indicator
                self.display_response(&response);

                // Handle continuation if needed
                if !response.completed && response.continuation_prompt.is_some() {
                    if let Some(continuation) = &response.continuation_prompt {
                        self.handle_continuation(continuation).await?;
                    }
                }
                response
            }
            Err(e) => {
                print!("\r              \r"); // Clear thinking indicator
                eprintln!("❌ Error: {}", e);
                return Err(e);
            }
        };

        let total_duration = start_time.elapsed();
        if total_duration.as_millis() > 1000 {
            println!("⏱️  Completed in {:.2}s", total_duration.as_secs_f32());
        }

        Ok(())
    }

    /// Handle continuation in Claude Code style
    async fn handle_continuation(&mut self, continuation_prompt: &str) -> SageResult<()> {
        if self.interactive {
            // In interactive mode, show continuation prompt and wait for user input
            println!("\n{}", continuation_prompt);
            print!("Continue? (y/n): ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();

            if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
                print!("🤖 Continuing... ");
                io::stdout().flush().unwrap();
                match self
                    .execution_manager
                    .continue_interactive(continuation_prompt)
                    .await
                {
                    Ok(response) => {
                        print!("\r               \r"); // Clear indicator
                        self.display_response(&response);
                    }
                    Err(e) => {
                        print!("\r               \r"); // Clear indicator
                        eprintln!("❌ Error: {}", e);
                        return Err(e);
                    }
                }
            }
        } else {
            // In non-interactive mode, auto-continue
            print!("🤖 Continuing... ");
            io::stdout().flush().unwrap();
            match self
                .execution_manager
                .continue_interactive(continuation_prompt)
                .await
            {
                Ok(response) => {
                    print!("\r               \r"); // Clear indicator
                    self.display_response(&response);
                }
                Err(e) => {
                    print!("\r               \r"); // Clear indicator
                    eprintln!("❌ Error: {}", e);
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// Display response in Claude Code style
    fn display_response(&self, response: &ReactiveResponse) {
        // Show tool execution status concisely
        if !response.tool_calls.is_empty() {
            for (i, call) in response.tool_calls.iter().enumerate() {
                let result = response.tool_results.get(i);
                let status = match result {
                    Some(r) if r.success => "✅",
                    Some(_) => "❌",
                    None => "⏳",
                };
                println!("{} {}", status, call.name);
            }
        }

        // Display the main response
        if !response.content.trim().is_empty() {
            println!("{}", response.content.trim());
        }

        // Show completion status
        if response.completed {
            println!("✅ Task completed");
        }
    }

    /// Run interactive mode (Claude Code style REPL)
    pub async fn run_interactive(&mut self) -> SageResult<()> {
        println!("Sage Agent - Claude Code Style");
        println!("Type 'exit' to quit\n");

        loop {
            print!("sage> ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => {
                    let command = input.trim();

                    if command.is_empty() {
                        continue;
                    }

                    if command == "exit" || command == "quit" {
                        break;
                    }

                    if let Err(e) = self.execute_command(command).await {
                        eprintln!("❌ Error: {}", e);
                    }

                    println!(); // Add spacing between interactions
                }
                Err(e) => {
                    eprintln!("❌ Failed to read input: {}", e);
                    break;
                }
            }
        }

        println!("Goodbye!");
        Ok(())
    }

    /// Run in single command mode
    pub async fn run_single_command(&mut self, command: &str) -> SageResult<()> {
        self.execute_command(command).await
    }
}

/// Claude Code style configuration for the CLI
pub struct ClaudeModeConfig {
    pub max_response_length: usize,
    pub show_timing: bool,
    pub auto_continue: bool,
    pub truncate_long_output: bool,
}

impl Default for ClaudeModeConfig {
    fn default() -> Self {
        Self {
            max_response_length: 2000,
            show_timing: true,
            auto_continue: false,
            truncate_long_output: true,
        }
    }
}

/// Extensions for better Claude Code style experience
impl ClaudeMode {
    /// Set Claude mode configuration
    pub fn with_config(self, _config: ClaudeModeConfig) -> Self {
        // Configuration would be applied here
        // For now, just return self as the basic implementation
        // doesn't store the config
        self
    }

    /// Quick command execution (fire and forget style)
    pub async fn quick_execute(&mut self, command: &str) -> SageResult<String> {
        let response = self.execution_manager.interactive_mode(command).await?;

        let formatted = response.content.clone();

        Ok(formatted)
    }

    /// Check if the current session has any active operations
    pub fn has_active_operations(&self) -> bool {
        // In Claude Code style, operations are typically short-lived
        // This would be true only during actual execution
        false
    }
}

/// Run Claude Code style interactive mode
pub async fn run_claude_interactive(config_file: &str) -> SageResult<()> {
    println!("🚀 Sage Agent - Claude Code Style");
    println!("Loading configuration from: {}", config_file);

    // Load configuration
    let config = load_config_from_file(config_file).await?;

    // Create Claude mode CLI
    let mut claude_mode = ClaudeMode::new(config, true)?;

    // Run interactive mode
    claude_mode.run_interactive().await
}

/// Load configuration from file
async fn load_config_from_file(config_file: &str) -> SageResult<Config> {
    match std::fs::read_to_string(config_file) {
        Ok(content) => match serde_json::from_str::<Config>(&content) {
            Ok(config) => {
                println!("✅ Loaded configuration from {}", config_file);
                Ok(config)
            }
            Err(e) => {
                println!("⚠️  Config file invalid: {}", e);
                println!("📝 Using default configuration");
                Ok(Config::default())
            }
        },
        Err(_) => {
            println!("⚠️  {} not found", config_file);
            println!("📝 Using default configuration");
            Ok(Config::default())
        }
    }
}
