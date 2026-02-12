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

mod args;
mod commands;
mod console;
mod router;
mod signal_handler;
mod ui;

use clap::Parser;
use sage_core::config::{load_config, load_config_from_file};
use sage_core::error::SageResult;

// Re-export for external use
pub use args::{Cli, Commands, ConfigAction, DEFAULT_CONFIG_FILE};

#[tokio::main]
async fn main() -> SageResult<()> {
    let cli = Cli::parse();

    let config = if cli.config_file == args::DEFAULT_CONFIG_FILE {
        load_config().ok()
    } else {
        load_config_from_file(&cli.config_file).ok()
    };

    if let Some(config) = config {
        let env_filter = if std::env::var_os("RUST_LOG").is_some() {
            tracing_subscriber::EnvFilter::from_default_env()
        } else {
            tracing_subscriber::EnvFilter::new(config.logging.level)
        };

        match config.logging.format.as_str() {
            "json" => {
                tracing_subscriber::fmt()
                    .json()
                    .with_env_filter(env_filter)
                    .init();
            }
            "pretty" => {
                tracing_subscriber::fmt()
                    .pretty()
                    .with_env_filter(env_filter)
                    .init();
            }
            _ => {
                tracing_subscriber::fmt()
                    .compact()
                    .with_env_filter(env_filter)
                    .init();
            }
        }
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    }

    // Initialize icon mode from environment (SAGE_NERD_FONTS=false to disable)
    sage_core::ui::init_icons();

    router::route(cli).await
}
