//! Checkpoint manager
//!
//! This module provides the high-level checkpoint management API,
//! orchestrating checkpoint creation, restoration, and listing.

mod core;
mod operations;
#[cfg(test)]
mod tests;
mod types;

pub use types::CheckpointManager;
