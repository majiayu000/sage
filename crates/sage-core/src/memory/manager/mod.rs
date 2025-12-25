//! Memory manager module

mod config;
mod core;
pub(crate) mod helpers;
mod maintenance;
mod operations;

#[cfg(test)]
mod tests;

// Re-export public types and functions
pub use config::{MemoryConfig, MemoryStats};
pub use core::MemoryManager;
pub use helpers::{SharedMemoryManager, create_memory_manager};
