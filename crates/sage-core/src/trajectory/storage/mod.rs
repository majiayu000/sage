//! Trajectory storage implementations
//!
//! This module provides various storage backends for trajectory records,
//! including file-based and in-memory storage options.

// Module declarations
mod compression;
mod file_ops;
mod file_storage;
mod file_storage_impl;
mod memory_storage;
mod rotation;
mod trait_def;
mod types;

// Tests module
#[cfg(test)]
mod tests;

// Public re-exports to maintain backward compatibility
pub use file_storage::FileStorage;
pub use memory_storage::MemoryStorage;
pub use trait_def::TrajectoryStorage;
pub use types::{RotationConfig, StorageStatistics};
