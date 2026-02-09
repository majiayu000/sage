//! Sage CLI UI Module
//!
//! Provides UI components for the CLI interface.
//! Uses the rnk App mode for CLI UI rendering.
//!
//! # Architecture
//!
//! - `adapters/` - Framework-specific implementations of sage-core UI traits
//! - `rnk_app/` - rnk-based terminal UI implementation

mod adapters;
mod rnk_app;

use crate::args::Cli;
use std::io;

pub async fn run_rnk_app_with_cli(cli: &Cli) -> io::Result<()> {
    rnk_app::run_rnk_app(cli).await
}
