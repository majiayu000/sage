//! CLI commands

pub mod config;
pub mod diagnostics;
pub mod interactive;
pub mod tools;
pub mod trajectory;
pub mod unified;

pub use unified::{UnifiedArgs, execute as unified_execute};
