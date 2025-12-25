//! Parallel tool executor implementation
//!
//! This module contains the core parallel executor split into focused submodules:
//! - `types`: Internal type definitions (PermitGuard)
//! - `executor`: Main ParallelToolExecutor struct and public API
//! - `scheduler`: Permit acquisition and result ordering logic
//! - `permission`: Permission checking logic

mod executor;
mod permission;
mod scheduler;
mod types;

// Re-export public items
pub use executor::ParallelToolExecutor;
