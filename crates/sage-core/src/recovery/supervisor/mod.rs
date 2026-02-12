//! Supervision and error isolation for tasks
//!
//! Provides supervision strategies for managing task lifecycles and failures.

mod task_supervisor;
mod types;

#[cfg(test)]
mod tests;

// Re-export public types
pub use task_supervisor::TaskSupervisor;
pub use types::{SupervisionPolicy, SupervisionResult};
