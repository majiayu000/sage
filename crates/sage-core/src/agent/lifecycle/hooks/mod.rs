//! Lifecycle hooks and registry

pub mod builtin;
mod registry;
mod traits;

pub use registry::LifecycleHookRegistry;
pub use traits::{AgentLifecycle, LifecycleHook};
