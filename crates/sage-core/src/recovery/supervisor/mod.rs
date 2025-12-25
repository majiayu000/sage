//! Supervision and error isolation for tasks
//!
//! Provides supervision strategies for managing task lifecycles and failures.

mod multi_supervisor;
mod recovery;
mod task_supervisor;
mod types;

#[cfg(test)]
mod tests;

// Re-export public types
pub use multi_supervisor::Supervisor;
pub use recovery::catch_panic;
pub use task_supervisor::TaskSupervisor;
pub use types::{SupervisionEvent, SupervisionPolicy, SupervisionResult};
