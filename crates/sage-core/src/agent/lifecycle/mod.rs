//! Agent lifecycle hooks
//!
//! Provides lifecycle management for agents with hooks at various points
//! in the agent execution flow.

mod context;
mod error;
pub mod hooks;
mod manager;
mod phase;
#[cfg(test)]
mod tests;

// Re-export main types for backward compatibility
pub use context::{HookResult, LifecycleContext};
pub use error::{LifecycleError, LifecycleResult};
pub use hooks::builtin::{LoggingHook, MetricsHook};
pub use hooks::{AgentLifecycle, LifecycleHook, LifecycleHookRegistry};
pub use manager::LifecycleManager;
pub use phase::LifecyclePhase;
