//! Permission handler trait and shared handler type

use async_trait::async_trait;
use std::sync::Arc;

use super::request::{PermissionDecision, PermissionRequest};

/// Handler for permission requests
///
/// Implement this trait to customize how permission requests are handled,
/// e.g., through a CLI prompt, GUI dialog, or automatic policy.
#[async_trait]
pub trait PermissionHandler: Send + Sync {
    /// Handle a permission request
    ///
    /// # Arguments
    /// * `request` - The permission request details
    ///
    /// # Returns
    /// The user's decision
    async fn handle_permission_request(&self, request: PermissionRequest) -> PermissionDecision;
}

/// Shared permission handler type
pub type SharedPermissionHandler = Arc<dyn PermissionHandler>;
