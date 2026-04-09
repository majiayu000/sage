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

#[derive(Debug, Clone, Copy)]
enum LogFormat {
    Json,
    Pretty,
    Compact,
}

fn resolve_log_format(cli: &Cli) -> LogFormat {
    let config = if cli.config_file == args::DEFAULT_CONFIG_FILE {
        load_config().ok()
    } else {
        load_config_from_file(&cli.config_file).ok()
    };

    match config.as_ref().map(|cfg| cfg.logging.format.as_str()) {
        Some("json") => LogFormat::Json,
        Some("pretty") => LogFormat::Pretty,
        _ => LogFormat::Compact,
    }
}

fn init_tracing(cli: &Cli) {
    // Keep logging disabled by default to avoid corrupting TUI output on stderr.
    // When RUST_LOG is not set, skip subscriber initialization entirely.
    if std::env::var_os("RUST_LOG").is_none() {
        return;
    }

    let env_filter = tracing_subscriber::EnvFilter::from_default_env();
    match resolve_log_format(cli) {
        LogFormat::Json => {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(env_filter)
                .with_writer(std::io::stderr)
                .init();
        }
        LogFormat::Pretty => {
            tracing_subscriber::fmt()
                .pretty()
                .with_env_filter(env_filter)
                .with_writer(std::io::stderr)
                .init();
        }
        LogFormat::Compact => {
            tracing_subscriber::fmt()
                .compact()
                .with_env_filter(env_filter)
                .with_writer(std::io::stderr)
                .init();
        }
    }
}

#[tokio::main]
async fn main() -> SageResult<()> {
    let cli = Cli::parse();
    init_tracing(&cli);

    router::route(cli).await
}
