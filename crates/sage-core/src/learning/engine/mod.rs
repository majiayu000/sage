//! Learning engine module
//!
//! This module provides the core learning engine functionality split into focused submodules:
//! - `error`: Error types for learning operations
//! - `core`: Engine struct and basic operations
//! - `learning`: Pattern learning, reinforcement, and contradiction
//! - `retrieval`: Pattern retrieval and query operations
//! - `persistence`: Pattern persistence and lifecycle management

pub mod core;
pub mod error;
pub mod learning;
pub mod persistence;
pub mod retrieval;

#[cfg(test)]
mod tests;

// Re-export main types
pub use core::{
    LearningEngine, SharedLearningEngine, create_learning_engine,
    create_learning_engine_with_memory,
};
pub use error::LearningError;
