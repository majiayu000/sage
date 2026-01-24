//! Sage CLI UI Module
//!
//! Provides UI components for the CLI interface.
//! Uses the rnk App mode for CLI UI rendering.
//!
//! # Architecture
//!
//! - `adapters/` - Framework-specific implementations of sage-core UI traits
//! - `rnk_app/` - rnk-based terminal UI implementation

pub mod adapters;
mod rnk_app;

pub use adapters::RnkEventSink;
pub use rnk_app::run_rnk_app;

