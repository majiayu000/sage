//! Policy-based permission handler

use async_trait::async_trait;
use std::collections::HashMap;

use crate::tools::permission::handler::PermissionHandler;
use crate::tools::permission::request::{PermissionDecision, PermissionRequest};
use crate::tools::permission::types::RiskLevel;

/// Permission policy
#[derive(Debug, Clone)]
pub struct PermissionPolicy {
    /// Whether to allow by default
    pub allow_by_default: bool,
    /// Maximum risk level to auto-allow
    pub max_auto_allow_risk: RiskLevel,
    /// Specific tool overrides
    pub tool_overrides: HashMap<String, bool>,
}

impl Default for PermissionPolicy {
    fn default() -> Self {
        Self {
            allow_by_default: true,
            max_auto_allow_risk: RiskLevel::Medium,
            tool_overrides: HashMap::new(),
        }
    }
}

/// Policy-based permission handler
pub struct PolicyHandler {
    /// Policies for different tools/operations
    policies: HashMap<String, PermissionPolicy>,
    /// Default policy
    default_policy: PermissionPolicy,
}

impl PolicyHandler {
    /// Create a new policy handler with default policy
    pub fn new(default_policy: PermissionPolicy) -> Self {
        Self {
            policies: HashMap::new(),
            default_policy,
        }
    }

    /// Add a policy for a specific context (e.g., session, agent)
    pub fn with_policy(mut self, key: impl Into<String>, policy: PermissionPolicy) -> Self {
        self.policies.insert(key.into(), policy);
        self
    }
}

#[async_trait]
impl PermissionHandler for PolicyHandler {
    async fn handle_permission_request(&self, request: PermissionRequest) -> PermissionDecision {
        let policy = &self.default_policy;

        // Check tool-specific override
        if let Some(&allowed) = policy.tool_overrides.get(&request.tool_name) {
            return if allowed {
                PermissionDecision::Allow
            } else {
                PermissionDecision::Deny
            };
        }

        // Check risk level
        if request.risk_level <= policy.max_auto_allow_risk {
            return PermissionDecision::Allow;
        }

        // Fall back to default
        if policy.allow_by_default {
            PermissionDecision::Allow
        } else {
            PermissionDecision::Deny
        }
    }
}
