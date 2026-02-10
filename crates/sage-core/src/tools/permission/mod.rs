//! Permission system for tool execution
//!
//! This module provides a permission checking framework for tools, including:
//! - Risk level assessment
//! - Permission decisions (allow, deny, ask)
//! - Permission handlers for user interaction
//! - Tool execution context
//! - Rule-based permission matching (OpenClaude compatible)

mod cache;
mod context;
mod handler;
mod handlers;
mod request;
mod rules;
#[cfg(test)]
mod tests;
mod types;

// Re-export all public types for backward compatibility
pub use cache::PermissionCache;
pub use context::ToolContext;
pub use handler::{PermissionHandler, SharedPermissionHandler};
pub use handlers::{AutoAllowHandler, AutoDenyHandler, PermissionPolicy, PolicyHandler};
pub use request::{PermissionDecision, PermissionRequest, ToolPermissionResult};
pub use rules::{
    PermissionEvaluation, PermissionRule, PermissionRuleEngine, PermissionRulesConfig,
    RuleBasedHandler,
};
pub use types::{PermissionBehavior, RiskLevel, RuleSource};
