// Allow common clippy lints that are stylistic preferences
#![allow(clippy::collapsible_if)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::manual_range_patterns)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::single_char_add_str)]
#![allow(clippy::option_map_or_none)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::filter_map_identity)]

//! Sage Agent CLI application
//!
//! This CLI provides multiple execution modes for different use cases:
//!
//! # CLI Modes Overview
//!
//! ## 1. Interactive Mode (Default)
//! Start a conversation loop where you can have multi-turn conversations with the AI.
//! The AI remembers context across messages within the same conversation.
//!
//! **Use when:** You want to have a back-and-forth conversation, iterating on tasks
//! **Command:** `sage` or `sage interactive`
//! **Example:** Ask the AI to create a file, then ask it to modify that file
//!
//! ## 2. Run Mode (One-shot)
//! Execute a single task and exit. Best for automation and scripting.
//! The AI completes the task and returns immediately.
//!
//! **Use when:** You have a single, well-defined task to complete
//! **Command:** `sage run "<task>"`
//! **Example:** `sage run "Create a Python hello world script"`
//!
//! ## 3. Unified Mode (Advanced)
//! New execution model with inline user input blocking. Supports both interactive
//! and non-interactive modes via flag.
//!
//! **Use when:** You need fine-grained control over execution behavior
//! **Command:** `sage unified "<task>"`
//! **Example:** `sage unified --non-interactive "Run tests"`
//!
//! ## 4. Utility Commands
//! Additional commands for configuration, trajectory analysis, and tool inspection.
//! See `sage --help` for full list.

mod claude_mode;
mod commands;
mod console;
mod progress;
mod signal_handler;
mod ui_backend;
mod ui_launcher;

use clap::{Parser, Subcommand};
use sage_core::error::SageResult;
use std::path::PathBuf;

/// CLI Mode enum for documentation and type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliMode {
    /// Interactive conversation mode with multi-turn context
    Interactive,
    /// One-shot task execution mode
    Run,
    /// Unified execution mode (Claude Code style)
    Unified,
    /// Configuration management
    Config,
    /// Trajectory analysis and management
    Trajectory,
    /// Tool inspection
    Tools,
}

impl CliMode {
    /// Get a human-readable description of the mode
    pub fn description(&self) -> &'static str {
        match self {
            CliMode::Interactive => "Multi-turn conversation mode with context retention",
            CliMode::Run => "Single task execution mode (fire-and-forget)",
            CliMode::Unified => "Advanced execution mode with inline input blocking",
            CliMode::Config => "Configuration file management",
            CliMode::Trajectory => "Execution trajectory analysis",
            CliMode::Tools => "Tool discovery and inspection",
        }
    }

    /// Get usage examples for the mode
    pub fn examples(&self) -> Vec<&'static str> {
        match self {
            CliMode::Interactive => vec![
                "sage interactive",
                "sage interactive --modern-ui",
                "sage interactive --claude-style",
            ],
            CliMode::Run => vec![
                "sage run \"Create a Python hello world\"",
                "sage run \"Fix the bug in main.rs\" --provider anthropic",
                "sage run --task-file task.txt",
            ],
            CliMode::Unified => vec![
                "sage unified \"Create a test suite\"",
                "sage unified --non-interactive \"Run tests\"",
                "sage unified --max-steps 10 \"Refactor code\"",
            ],
            CliMode::Config => vec![
                "sage config show",
                "sage config validate",
                "sage config init",
            ],
            CliMode::Trajectory => vec![
                "sage trajectory list",
                "sage trajectory show <file>",
                "sage trajectory analyze <file>",
            ],
            CliMode::Tools => vec!["sage tools"],
        }
    }
}

#[derive(Parser)]
#[command(name = "sage")]
#[command(about = "Sage Agent - LLM-based agent for software engineering tasks")]
#[command(long_about = r#"Sage Agent - LLM-based agent for software engineering tasks

MODES:
  Interactive (default) - Multi-turn conversation with context
  Run                  - One-shot task execution
  Unified              - Advanced execution with inline input

QUICK START:
  sage                           # Start interactive mode
  sage run "your task"           # Execute a single task
  sage interactive --modern-ui   # Use modern UI
  sage config init               # Create config file

For detailed help on any command, use: sage <command> --help"#)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to configuration file (used when no subcommand is provided)
    #[arg(long, default_value = "sage_config.json")]
    config_file: String,

    /// Path to save trajectory file (used when no subcommand is provided)
    #[arg(long)]
    trajectory_file: Option<PathBuf>,

    /// Working directory for the agent (used when no subcommand is provided)
    #[arg(long)]
    working_dir: Option<PathBuf>,

    /// Enable verbose output (used when no subcommand is provided)
    #[arg(long, short)]
    verbose: bool,

    /// Use modern UI (Ink + React) instead of traditional CLI
    #[arg(long)]
    modern_ui: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a single task and exit (non-interactive mode)
    ///
    /// This mode executes one task and exits. Best for automation, scripting,
    /// or when you have a single well-defined task to complete.
    ///
    /// The task can be provided as a string or loaded from a file.
    ///
    /// Examples:
    ///   sage run "Create a Python script that prints hello world"
    ///   sage run "Fix the bug in src/main.rs" --provider anthropic --model claude-3-opus
    ///   sage run --task task.txt --working-dir /path/to/project
    #[command(verbatim_doc_comment)]
    Run {
        /// The task description or path to a file containing the task
        task: String,

        /// LLM provider to use (openai, anthropic, google, ollama)
        #[arg(long)]
        provider: Option<String>,

        /// Model to use
        #[arg(long)]
        model: Option<String>,

        /// Base URL for the model API
        #[arg(long)]
        model_base_url: Option<String>,

        /// API key for the provider
        #[arg(long)]
        api_key: Option<String>,

        /// Maximum number of execution steps
        #[arg(long)]
        max_steps: Option<u32>,

        /// Working directory for the agent
        #[arg(long)]
        working_dir: Option<PathBuf>,

        /// Path to configuration file
        #[arg(long, default_value = "sage_config.json")]
        config_file: String,

        /// Path to save trajectory file
        #[arg(long)]
        trajectory_file: Option<PathBuf>,

        /// Path to patch file
        #[arg(long)]
        patch_path: Option<PathBuf>,

        /// Whether to create a patch
        #[arg(long)]
        must_patch: bool,

        /// Enable verbose output
        #[arg(long, short)]
        verbose: bool,

        /// Use modern UI (Ink + React)
        #[arg(long)]
        modern_ui: bool,
    },

    /// Start interactive conversation mode (multi-turn with context)
    ///
    /// This mode starts a conversation loop where you can have multiple exchanges
    /// with the AI. The AI remembers previous messages and maintains context within
    /// the same conversation session.
    ///
    /// Perfect for iterative development where you want to:
    /// - Create something, then refine it
    /// - Ask follow-up questions
    /// - Build on previous responses
    ///
    /// Special commands available in interactive mode:
    /// - help, h          - Show available commands
    /// - new, new-task    - Start a new conversation (clears context)
    /// - exit, quit, q    - Exit interactive mode
    /// - config           - Show current configuration
    /// - status           - Show system status
    ///
    /// Examples:
    ///   sage interactive
    ///   sage interactive --modern-ui
    ///   sage interactive --claude-style
    ///   sage interactive --config my_config.json
    #[command(verbatim_doc_comment)]
    Interactive {
        /// Path to configuration file
        #[arg(long, default_value = "sage_config.json")]
        config_file: String,

        /// Path to save trajectory file
        #[arg(long)]
        trajectory_file: Option<PathBuf>,

        /// Working directory for the agent
        #[arg(long)]
        working_dir: Option<PathBuf>,

        /// Enable verbose output
        #[arg(long, short)]
        verbose: bool,

        /// Use modern UI (Ink + React)
        #[arg(long)]
        modern_ui: bool,

        /// Use Claude Code style execution (lightweight, fast responses)
        #[arg(long)]
        claude_style: bool,
    },

    /// Manage configuration files
    ///
    /// Use this command to create, validate, or inspect configuration files.
    /// Configuration files control LLM provider settings, API keys, tool
    /// configurations, and execution parameters.
    ///
    /// Examples:
    ///   sage config init                    # Create a new config file
    ///   sage config show                    # Display current configuration
    ///   sage config validate                # Check config for errors
    ///   sage config init --force            # Overwrite existing config
    #[command(verbatim_doc_comment)]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Analyze and manage trajectory files
    ///
    /// Trajectories are execution recordings that capture all steps, tool calls,
    /// LLM interactions, and results during agent execution. Use this command to
    /// review past executions, analyze performance, or debug issues.
    ///
    /// Examples:
    ///   sage trajectory list                # List all trajectory files
    ///   sage trajectory show file.json      # Display trajectory details
    ///   sage trajectory stats file.json     # Show statistics
    ///   sage trajectory analyze .           # Analyze all in directory
    #[command(verbatim_doc_comment)]
    Trajectory {
        #[command(subcommand)]
        action: TrajectoryAction,
    },

    /// List all available tools and their descriptions
    ///
    /// Shows all registered tools that the agent can use, including:
    /// - File operations (read, write, edit)
    /// - Process execution (bash, command)
    /// - Code analysis (search, grep)
    /// - Task management
    /// - And more
    ///
    /// Examples:
    ///   sage tools                          # List all tools
    #[command(verbatim_doc_comment)]
    Tools,

    /// Run task with unified execution loop (Claude Code style) [ADVANCED]
    ///
    /// This is an advanced execution mode that uses a unified loop inspired by
    /// Claude Code. It supports both interactive and non-interactive execution
    /// with inline user input blocking.
    ///
    /// Key differences from 'run' and 'interactive':
    /// - Uses inline blocking for user input (InputChannel)
    /// - Never exits for user questions in interactive mode
    /// - More fine-grained control over execution behavior
    /// - Cleaner execution model with fewer edge cases
    ///
    /// When to use:
    /// - You want the newer, more robust execution model
    /// - You need non-interactive mode with auto-responses
    /// - You're integrating Sage into automated workflows
    ///
    /// Examples:
    ///   sage unified "Create a test suite"
    ///   sage unified --non-interactive "Run tests and report results"
    ///   sage unified --max-steps 10 "Refactor the code"
    ///   sage unified --working-dir /path/to/project "Fix the bug"
    #[command(verbatim_doc_comment)]
    Unified {
        /// The task description (omit for interactive prompt)
        task: Option<String>,

        /// Path to configuration file
        #[arg(long, default_value = "sage_config.json")]
        config_file: String,

        /// Path to save trajectory file
        #[arg(long)]
        trajectory_file: Option<PathBuf>,

        /// Working directory for the agent
        #[arg(long)]
        working_dir: Option<PathBuf>,

        /// Maximum number of execution steps
        #[arg(long)]
        max_steps: Option<u32>,

        /// Enable verbose output
        #[arg(long, short)]
        verbose: bool,

        /// Non-interactive mode (auto-respond to user questions)
        /// When enabled, the agent will not wait for user input and will
        /// make decisions autonomously
        #[arg(long)]
        non_interactive: bool,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Display current configuration settings
    ///
    /// Shows the complete configuration including provider settings,
    /// API keys (masked), tool configurations, and execution parameters.
    Show {
        /// Path to configuration file
        #[arg(long, default_value = "sage_config.json")]
        config_file: String,
    },

    /// Validate configuration file for errors
    ///
    /// Checks the configuration file for syntax errors, missing required
    /// fields, invalid provider settings, and other configuration issues.
    Validate {
        /// Path to configuration file
        #[arg(long, default_value = "sage_config.json")]
        config_file: String,
    },

    /// Create a new configuration file with defaults
    ///
    /// Generates a sample configuration file with placeholders for
    /// API keys and sensible defaults for all settings.
    Init {
        /// Path for the new configuration file
        #[arg(long, default_value = "sage_config.json")]
        config_file: String,

        /// Overwrite existing file without prompting
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum TrajectoryAction {
    /// List all trajectory files in a directory
    ///
    /// Scans the specified directory for trajectory JSON files and
    /// displays them with metadata (date, task, status, etc.).
    List {
        /// Directory to search for trajectories
        #[arg(long, default_value = ".")]
        directory: PathBuf,
    },

    /// Display detailed information about a trajectory
    ///
    /// Shows complete execution details including all steps, tool calls,
    /// LLM interactions, token usage, and final results.
    Show {
        /// Path to trajectory file
        trajectory_file: PathBuf,
    },

    /// Calculate statistics for trajectory file(s)
    ///
    /// Computes aggregated statistics like total steps, token usage,
    /// execution time, tool usage frequency, and success rate.
    Stats {
        /// Path to trajectory file or directory
        path: PathBuf,
    },

    /// Analyze execution patterns and performance
    ///
    /// Performs deep analysis of trajectory patterns including common
    /// failure points, bottlenecks, tool usage patterns, and optimization
    /// opportunities.
    Analyze {
        /// Path to trajectory file or directory
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> SageResult<()> {
    // Initialize logging with environment-based filtering
    // Set RUST_LOG=debug for verbose logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        // DEFAULT BEHAVIOR: If no subcommand is provided, start interactive mode
        // This makes `sage` equivalent to `sage interactive` for convenience
        // Users who want one-shot execution should use `sage run "<task>"`
        None => {
            if cli.modern_ui {
                ui_launcher::launch_modern_ui(
                    &cli.config_file,
                    cli.trajectory_file.as_ref().and_then(|p| p.to_str()),
                    cli.working_dir.as_ref().and_then(|p| p.to_str()),
                )
                .await
            } else {
                commands::interactive::execute(commands::interactive::InteractiveArgs {
                    config_file: cli.config_file,
                    trajectory_file: cli.trajectory_file,
                    working_dir: cli.working_dir,
                })
                .await
            }
        }
        Some(Commands::Run {
            task,
            provider,
            model,
            model_base_url,
            api_key,
            max_steps,
            working_dir,
            config_file,
            trajectory_file,
            patch_path,
            must_patch,
            verbose,
            modern_ui: _,
        }) => {
            // For now, Run command doesn't use modern UI
            // TODO: Add modern UI support for run command
            commands::run::execute(commands::run::RunArgs {
                task,
                provider,
                model,
                model_base_url,
                api_key,
                max_steps,
                working_dir,
                config_file,
                trajectory_file,
                patch_path,
                must_patch,
                verbose,
            })
            .await
        }

        Some(Commands::Interactive {
            config_file,
            trajectory_file,
            working_dir,
            verbose: _,
            modern_ui,
            claude_style,
        }) => {
            if claude_style {
                claude_mode::run_claude_interactive(&config_file).await
            } else if modern_ui {
                ui_launcher::launch_modern_ui(
                    &config_file,
                    trajectory_file.as_ref().and_then(|p| p.to_str()),
                    working_dir.as_ref().and_then(|p| p.to_str()),
                )
                .await
            } else {
                commands::interactive::execute(commands::interactive::InteractiveArgs {
                    config_file,
                    trajectory_file,
                    working_dir,
                })
                .await
            }
        }

        Some(Commands::Config { action }) => match action {
            ConfigAction::Show { config_file } => commands::config::show(&config_file).await,
            ConfigAction::Validate { config_file } => {
                commands::config::validate(&config_file).await
            }
            ConfigAction::Init { config_file, force } => {
                commands::config::init(&config_file, force).await
            }
        },

        Some(Commands::Trajectory { action }) => match action {
            TrajectoryAction::List { directory } => commands::trajectory::list(&directory).await,
            TrajectoryAction::Show { trajectory_file } => {
                commands::trajectory::show(&trajectory_file).await
            }
            TrajectoryAction::Stats { path } => commands::trajectory::stats(&path).await,
            TrajectoryAction::Analyze { path } => commands::trajectory::analyze(&path).await,
        },

        Some(Commands::Tools) => commands::tools::show_tools().await,

        Some(Commands::Unified {
            task,
            config_file,
            trajectory_file,
            working_dir,
            max_steps,
            verbose,
            non_interactive,
        }) => {
            commands::unified_execute(commands::UnifiedArgs {
                task,
                config_file,
                trajectory_file,
                working_dir,
                max_steps,
                verbose,
                non_interactive,
            })
            .await
        }
    }
}
