//! Task definitions and loading for evaluation
//!
//! This module provides the core task types and loading functionality.

mod loader;
mod task;
mod verifier;

pub use loader::TaskLoader;
pub use task::{Difficulty, EvalTask, TaskCategory};
pub use verifier::{Verifier, VerifierResult};
