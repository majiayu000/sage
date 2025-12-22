//! Rule-based permission handler

use async_trait::async_trait;
use std::sync::Arc;

use super::engine::PermissionRuleEngine;
use crate::tools::permission::handler::PermissionHandler;
use crate::tools::permission::request::{PermissionDecision, PermissionRequest};
use crate::tools::permission::types::PermissionBehavior;

/// Rule-based permission handler
pub struct RuleBasedHandler {
    engine: PermissionRuleEngine,
    fallback: Option<Arc<dyn PermissionHandler>>,
}

impl RuleBasedHandler {
    /// Create a new rule-based handler
    pub fn new(engine: PermissionRuleEngine) -> Self {
        Self {
            engine,
            fallback: None,
        }
    }

    /// Set a fallback handler for Ask behavior
    pub fn with_fallback(mut self, handler: Arc<dyn PermissionHandler>) -> Self {
        self.fallback = Some(handler);
        self
    }
}

#[async_trait]
impl PermissionHandler for RuleBasedHandler {
    async fn handle_permission_request(&self, request: PermissionRequest) -> PermissionDecision {
        let evaluation = self.engine.evaluate_request(&request);

        match evaluation.behavior {
            PermissionBehavior::Allow => PermissionDecision::Allow,
            PermissionBehavior::Deny => PermissionDecision::Deny,
            PermissionBehavior::Ask | PermissionBehavior::Passthrough => {
                // Delegate to fallback if available
                if let Some(ref fallback) = self.fallback {
                    fallback.handle_permission_request(request).await
                } else {
                    // Default to deny if no fallback
                    PermissionDecision::Deny
                }
            }
        }
    }
}
