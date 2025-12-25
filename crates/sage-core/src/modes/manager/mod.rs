//! Mode manager
//!
//! This module provides the mode manager for controlling agent operational modes.

mod core;
mod handlers;
mod transitions;
mod types;

#[cfg(test)]
mod tests;

// Re-export public items
pub use core::ModeManager;
pub use types::{ModeExitResult, PlanModeContext};
