//! Sage CLI UI Module
//!
//! Provides UI components for the CLI interface.
//! Includes both legacy streaming mode and new rnk App mode.

mod indicators;
mod rnk_app;
mod streaming;

pub use rnk_app::run_rnk_app;

