//! Sage Agent CLI application
//!
//! A powerful command-line interface for interacting with AI agents.
//!
//! # Installation
//!
//! ```bash
//! cargo install --path crates/sage-cli
//! ```
//!
//! # CLI Modes Overview
//!
//! This CLI provides multiple execution modes for different use cases:
//!
//! ## 1. Interactive Mode (Default)
//! Start a conversation loop where you can have multi-turn conversations with the AI.
//! The AI remembers context across messages within the same conversation.
//!
//! - **Use when:** You want to have a back-and-forth conversation, iterating on tasks
//! - **Command:** `sage` or `sage interactive`
//! - **Example:** Ask the AI to create a file, then ask it to modify that file
//!
//! ## 2. Run Mode (One-shot)
//! Execute a single task and exit. Best for automation and scripting.
//! The AI completes the task and returns immediately.
//!
//! - **Use when:** You have a single, well-defined task to complete
//! - **Command:** `sage run "<task>"`
//! - **Example:** `sage run "Create a Python hello world script"`
//!
//! ## 3. Unified Mode (Advanced)
//! New execution model with inline user input blocking. Supports both interactive
//! and non-interactive modes via flag.
//!
//! - **Use when:** You need fine-grained control over execution behavior
//! - **Command:** `sage unified "<task>"`
//! - **Example:** `sage unified --non-interactive "Run tests"`
//!
//! ## 4. Utility Commands
//! Additional commands for configuration, trajectory analysis, and tool inspection.
//! See `sage --help` for full list.

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

mod api_types;
mod args;
mod commands;
mod console;
mod ipc;
mod progress;
mod router;
mod signal_handler;
mod ui;
mod ui_backend;
mod ui_launcher;

use clap::Parser;
use sage_core::error::SageResult;

// Re-export for external use
pub use args::{Cli, Commands, ConfigAction, TrajectoryAction};

#[tokio::main]
async fn main() -> SageResult<()> {
    // Initialize logging with environment-based filtering
    // Set RUST_LOG=debug for verbose logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Initialize icon mode from environment (SAGE_NERD_FONTS=false to disable)
    sage_core::ui::init_icons();

    let cli = Cli::parse();
    router::route(cli).await
}
