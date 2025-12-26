//! CLI argument definitions using clap

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sage")]
#[command(about = "Sage Agent - LLM-based agent for software engineering tasks")]
#[command(
    long_about = r#"Sage Agent - LLM-based agent for software engineering tasks

MODES:
  Interactive (default) - Multi-turn conversation with context
  Run                  - One-shot task execution
  Unified              - Advanced execution with inline input

QUICK START:
  sage                           # Start interactive mode
  sage run "your task"           # Execute a single task
  sage interactive --modern-ui   # Use modern UI
  sage config init               # Create config file

For detailed help on any command, use: sage <command> --help"#
)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to configuration file (used when no subcommand is provided)
    #[arg(long, default_value = "sage_config.json")]
    pub config_file: String,

    /// Path to save trajectory file (used when no subcommand is provided)
    #[arg(long)]
    pub trajectory_file: Option<PathBuf>,

    /// Working directory for the agent (used when no subcommand is provided)
    #[arg(long)]
    pub working_dir: Option<PathBuf>,

    /// Enable verbose output (used when no subcommand is provided)
    #[arg(long, short)]
    pub verbose: bool,

    /// Use modern UI (Ink + React) instead of traditional CLI
    #[arg(long)]
    pub modern_ui: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run a single task and exit (non-interactive mode)
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
    #[command(verbatim_doc_comment)]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Analyze and manage trajectory files
    #[command(verbatim_doc_comment)]
    Trajectory {
        #[command(subcommand)]
        action: TrajectoryAction,
    },

    /// List all available tools and their descriptions
    #[command(verbatim_doc_comment)]
    Tools,

    /// Run task with unified execution loop (Claude Code style) - Advanced
    #[command(verbatim_doc_comment)]
    Unified {
        /// The task description (omit for interactive prompt)
        task: Option<String>,

        /// Path to configuration file
        #[arg(long, default_value = "sage_config.json")]
        config_file: String,

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
        #[arg(long)]
        non_interactive: bool,
    },

    /// Run as IPC backend for Modern UI (internal use)
    #[command(verbatim_doc_comment)]
    Ipc {
        /// Path to configuration file
        #[arg(long, default_value = "sage_config.json")]
        config_file: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Display current configuration settings
    Show {
        /// Path to configuration file
        #[arg(long, default_value = "sage_config.json")]
        config_file: String,
    },

    /// Validate configuration file for errors
    Validate {
        /// Path to configuration file
        #[arg(long, default_value = "sage_config.json")]
        config_file: String,
    },

    /// Create a new configuration file with defaults
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
pub enum TrajectoryAction {
    /// List all trajectory files in a directory
    List {
        /// Directory to search for trajectories
        #[arg(long, default_value = ".")]
        directory: PathBuf,
    },

    /// Display detailed information about a trajectory
    Show {
        /// Path to trajectory file
        trajectory_file: PathBuf,
    },

    /// Calculate statistics for trajectory file(s)
    Stats {
        /// Path to trajectory file or directory
        path: PathBuf,
    },

    /// Analyze execution patterns and performance
    Analyze {
        /// Path to trajectory file or directory
        path: PathBuf,
    },
}
