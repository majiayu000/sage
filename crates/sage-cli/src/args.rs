//! CLI argument definitions using clap
//!
//! Unified CLI structure inspired by Claude Code:
//! - sage                     # Interactive mode (default)
//! - sage "task"              # Execute task interactively
//! - sage -p "task"           # Print mode (non-interactive, one-shot)
//! - sage -c                  # Resume most recent session
//! - sage -r <id>             # Resume specific session
//! - sage config/tools        # Utility commands

use crate::commands::unified::OutputModeArg;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Default configuration file name used across all CLI commands.
pub const DEFAULT_CONFIG_FILE: &str = "sage_config.json";

#[derive(Parser)]
#[command(name = "sage")]
#[command(about = "Sage Agent - LLM-based agent for software engineering tasks")]
#[command(
    long_about = r#"Sage Agent - LLM-based agent for software engineering tasks

USAGE:
  sage                           # Start interactive mode
  sage "your task"               # Execute task (interactive)
  sage -p "your task"            # Print mode (non-interactive)
  sage -c                        # Resume most recent session
  sage -r <session_id>           # Resume specific session

UTILITY COMMANDS:
  sage config init               # Create config file
  sage config show               # Show current config
  sage tools                     # List available tools

For detailed help: sage --help"#
)]
#[command(version)]
pub struct Cli {
    /// Task description to execute (omit for interactive prompt)
    pub task: Option<String>,

    /// Print mode - execute task and exit without interaction
    #[arg(short = 'p', long = "print")]
    pub print_mode: bool,

    /// Resume the most recent session
    #[arg(short = 'c', long = "continue", conflicts_with = "resume_session")]
    pub continue_session: bool,

    /// Resume a specific session by ID
    #[arg(short = 'r', long = "resume", conflicts_with = "continue_session")]
    pub resume_session: Option<String>,

    /// Maximum number of execution steps (unlimited if not specified)
    #[arg(long)]
    pub max_steps: Option<u32>,

    /// Path to configuration file
    #[arg(long, default_value = DEFAULT_CONFIG_FILE)]
    pub config_file: String,

    /// Working directory for the agent
    #[arg(long)]
    pub working_dir: Option<PathBuf>,

    /// Enable verbose output
    #[arg(long, short)]
    pub verbose: bool,

    /// Output in streaming JSON format (for SDK/programmatic use)
    #[arg(long)]
    pub stream_json: bool,

    /// Output mode: streaming (real-time), batch (collect then display), or silent
    #[arg(long, value_enum, default_value = "streaming")]
    pub output_mode: OutputModeArg,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage configuration files
    #[command(verbatim_doc_comment)]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// List all available tools and their descriptions
    #[command(verbatim_doc_comment)]
    Tools,

    /// Check system health and diagnose issues (like Claude Code's /doctor)
    #[command(verbatim_doc_comment)]
    Doctor {
        /// Path to configuration file
        #[arg(long, default_value = DEFAULT_CONFIG_FILE)]
        config_file: String,
    },

    /// Show current status and environment info
    #[command(verbatim_doc_comment)]
    Status {
        /// Path to configuration file
        #[arg(long, default_value = DEFAULT_CONFIG_FILE)]
        config_file: String,
    },

    /// Show token usage statistics for sessions
    #[command(verbatim_doc_comment)]
    Usage {
        /// Path to session directory
        #[arg(long)]
        session_dir: Option<PathBuf>,

        /// Show detailed breakdown by session
        #[arg(long, short)]
        detailed: bool,
    },
}

#[derive(Subcommand, Clone)]
pub enum ConfigAction {
    /// Display current configuration settings
    Show {
        /// Path to configuration file
        #[arg(long, default_value = DEFAULT_CONFIG_FILE)]
        config_file: String,
    },

    /// Validate configuration file for errors
    Validate {
        /// Path to configuration file
        #[arg(long, default_value = DEFAULT_CONFIG_FILE)]
        config_file: String,
    },

    /// Create a new configuration file with defaults
    Init {
        /// Path for the new configuration file
        #[arg(long, default_value = DEFAULT_CONFIG_FILE)]
        config_file: String,

        /// Overwrite existing file without prompting
        #[arg(long)]
        force: bool,
    },
}
