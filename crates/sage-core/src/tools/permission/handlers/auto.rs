//! Auto-allow and auto-deny permission handlers

use async_trait::async_trait;

use crate::tools::permission::handler::PermissionHandler;
use crate::tools::permission::request::{PermissionDecision, PermissionRequest};

/// Auto-allow permission handler (for non-interactive use)
pub struct AutoAllowHandler;

#[async_trait]
impl PermissionHandler for AutoAllowHandler {
    async fn handle_permission_request(&self, _request: PermissionRequest) -> PermissionDecision {
        PermissionDecision::Allow
    }
}

/// Auto-deny permission handler (for restricted environments)
pub struct AutoDenyHandler {
    pub reason: String,
}

impl AutoDenyHandler {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl PermissionHandler for AutoDenyHandler {
    async fn handle_permission_request(&self, _request: PermissionRequest) -> PermissionDecision {
        PermissionDecision::Deny
    }
}
