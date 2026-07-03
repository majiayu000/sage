//! Permission system for tool execution
//!
//! This module provides a permission checking framework for tools, including:
//! - Risk level assessment
//! - Permission decisions (allow, deny, ask)
//! - Permission handlers for user interaction
//! - Tool execution context

mod cache;
mod context;
mod handler;
mod handlers;
mod request;
#[cfg(test)]
mod rules;
#[cfg(test)]
mod tests;
mod types;

// Re-export all public types for backward compatibility
pub use cache::PermissionCache;
pub use context::ToolContext;
pub use handler::{PermissionHandler, SharedPermissionHandler};
pub use handlers::{AutoAllowHandler, AutoDenyHandler};
#[cfg(test)]
pub use handlers::{PermissionPolicy, PolicyHandler};
pub use request::{PermissionDecision, PermissionRequest, ToolPermissionResult};
#[cfg(test)]
pub use rules::{
    PermissionEvaluation, PermissionRule, PermissionRuleEngine, PermissionRulesConfig,
    RuleBasedHandler,
};
pub use types::{PermissionBehavior, RiskLevel, RuleSource};
