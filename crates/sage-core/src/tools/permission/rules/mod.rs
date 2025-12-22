//! Rule-based permission system

mod engine;
mod handler;
mod rule;

pub use engine::{PermissionEvaluation, PermissionRuleEngine, PermissionRulesConfig};
pub use handler::RuleBasedHandler;
pub use rule::PermissionRule;
