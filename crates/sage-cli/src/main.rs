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
//! This CLI uses a single unified execution architecture.
//!
//! - `sage`                     # Start interactive mode (TTY)
//! - `sage "task"`              # Execute a task
//! - `sage -p "task"`           # Non-interactive one-shot mode
//! - `sage -c`                  # Resume most recent session
//! - `sage -r <id>`             # Resume specific session
//! - `sage <utility command>`   # Config/diagnostics/tools commands

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
mod router;
mod signal_handler;
mod ui;

use clap::Parser;
use sage_core::error::SageResult;

// Re-export for external use
pub use args::{Cli, Commands, ConfigAction};

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
