//! Permission system for tool execution
//!
//! This module provides a permission checking framework for tools, including:
//! - Risk level assessment
//! - Permission decisions (allow, deny, ask)
//! - Permission handlers for user interaction
//! - Tool execution context

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::ToolCall;

/// Risk level for tool operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Low risk - read-only operations, no side effects
    Low,
    /// Medium risk - local modifications, reversible
    Medium,
    /// High risk - significant changes, network access
    High,
    /// Critical risk - system modifications, irreversible operations
    Critical,
}

impl RiskLevel {
    /// Check if this risk level requires user confirmation by default
    pub fn requires_confirmation(&self) -> bool {
        matches!(self, RiskLevel::High | RiskLevel::Critical)
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            RiskLevel::Low => "Low risk - safe, read-only operation",
            RiskLevel::Medium => "Medium risk - local changes, reversible",
            RiskLevel::High => "High risk - significant changes",
            RiskLevel::Critical => "Critical risk - irreversible or system-wide",
        }
    }
}

impl Default for RiskLevel {
    fn default() -> Self {
        RiskLevel::Medium
    }
}

/// Permission check result from a tool
#[derive(Debug, Clone)]
pub enum PermissionResult {
    /// Allow execution to proceed
    Allow,

    /// Deny execution with reason
    Deny { reason: String },

    /// Ask user for permission
    Ask {
        /// Question to display to the user
        question: String,
        /// Default response if user doesn't respond
        default: bool,
        /// Risk level for this operation
        risk_level: RiskLevel,
    },

    /// Transform the input before execution
    Transform {
        /// Modified tool call
        new_call: ToolCall,
        /// Reason for transformation
        reason: String,
    },
}

impl PermissionResult {
    /// Create an allow result
    pub fn allow() -> Self {
        Self::Allow
    }

    /// Create a deny result
    pub fn deny(reason: impl Into<String>) -> Self {
        Self::Deny {
            reason: reason.into(),
        }
    }

    /// Create an ask result
    pub fn ask(question: impl Into<String>, default: bool, risk_level: RiskLevel) -> Self {
        Self::Ask {
            question: question.into(),
            default,
            risk_level,
        }
    }

    /// Create a transform result
    pub fn transform(new_call: ToolCall, reason: impl Into<String>) -> Self {
        Self::Transform {
            new_call,
            reason: reason.into(),
        }
    }

    /// Check if this result allows execution
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow | Self::Transform { .. })
    }
}

/// Permission request details sent to the permission handler
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    /// The tool being called
    pub tool_name: String,
    /// The tool call details
    pub call: ToolCall,
    /// Reason for the permission check
    pub reason: String,
    /// Risk level assessment
    pub risk_level: RiskLevel,
    /// Additional context
    pub context: HashMap<String, Value>,
}

impl PermissionRequest {
    /// Create a new permission request
    pub fn new(
        tool_name: impl Into<String>,
        call: ToolCall,
        reason: impl Into<String>,
        risk_level: RiskLevel,
    ) -> Self {
        Self {
            tool_name: tool_name.into(),
            call,
            reason: reason.into(),
            risk_level,
            context: HashMap::new(),
        }
    }

    /// Add context information
    pub fn with_context(mut self, key: impl Into<String>, value: Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }
}

/// User's decision on a permission request
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    /// Allow the operation
    Allow,
    /// Allow this and future similar operations
    AllowAlways,
    /// Deny the operation
    Deny,
    /// Deny this and future similar operations
    DenyAlways,
    /// Modify the operation
    Modify { new_call: ToolCall },
}

impl PermissionDecision {
    /// Check if this decision allows execution
    pub fn is_allowed(&self) -> bool {
        matches!(
            self,
            Self::Allow | Self::AllowAlways | Self::Modify { .. }
        )
    }
}

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
    async fn handle_permission_request(
        &self,
        request: PermissionRequest,
    ) -> PermissionDecision;
}

/// Execution context for tool permission checking
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Current working directory
    pub working_directory: PathBuf,
    /// Session ID
    pub session_id: Option<String>,
    /// Agent ID
    pub agent_id: Option<String>,
    /// User ID (if authenticated)
    pub user_id: Option<String>,
    /// Whether running in sandbox mode
    pub sandboxed: bool,
    /// Allowed paths for file operations
    pub allowed_paths: Vec<PathBuf>,
    /// Denied paths for file operations
    pub denied_paths: Vec<PathBuf>,
    /// Custom permissions
    pub custom_permissions: HashMap<String, bool>,
    /// Additional context data
    pub metadata: HashMap<String, Value>,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_default(),
            session_id: None,
            agent_id: None,
            user_id: None,
            sandboxed: false,
            allowed_paths: Vec::new(),
            denied_paths: Vec::new(),
            custom_permissions: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
}

impl ToolContext {
    /// Create a new tool context
    pub fn new(working_directory: PathBuf) -> Self {
        Self {
            working_directory,
            ..Default::default()
        }
    }

    /// Set session ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set agent ID
    pub fn with_agent_id(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Enable sandbox mode
    pub fn sandboxed(mut self) -> Self {
        self.sandboxed = true;
        self
    }

    /// Add allowed path
    pub fn allow_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.allowed_paths.push(path.into());
        self
    }

    /// Add denied path
    pub fn deny_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.denied_paths.push(path.into());
        self
    }

    /// Check if a path is allowed
    pub fn is_path_allowed(&self, path: &std::path::Path) -> bool {
        // Check denied paths first
        for denied in &self.denied_paths {
            if path.starts_with(denied) {
                return false;
            }
        }

        // If no allowed paths specified, allow all (except denied)
        if self.allowed_paths.is_empty() {
            return true;
        }

        // Check allowed paths
        for allowed in &self.allowed_paths {
            if path.starts_with(allowed) {
                return true;
            }
        }

        false
    }

    /// Set a custom permission
    pub fn set_permission(mut self, key: impl Into<String>, allowed: bool) -> Self {
        self.custom_permissions.insert(key.into(), allowed);
        self
    }

    /// Check a custom permission
    pub fn has_permission(&self, key: &str) -> Option<bool> {
        self.custom_permissions.get(key).copied()
    }
}

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

/// Policy-based permission handler
pub struct PolicyHandler {
    /// Policies for different tools/operations
    policies: HashMap<String, PermissionPolicy>,
    /// Default policy
    default_policy: PermissionPolicy,
}

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

/// Permission cache for "always allow" / "always deny" decisions
#[derive(Debug, Default)]
pub struct PermissionCache {
    allowed: RwLock<HashMap<String, bool>>,
}

impl PermissionCache {
    /// Create a new permission cache
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate cache key for a tool call
    pub fn cache_key(tool_name: &str, call: &ToolCall) -> String {
        // Simple key based on tool name and argument keys
        let arg_keys: Vec<_> = call.arguments.keys().collect();
        format!("{}:{:?}", tool_name, arg_keys)
    }

    /// Check if there's a cached decision
    pub async fn get(&self, key: &str) -> Option<bool> {
        self.allowed.read().await.get(key).copied()
    }

    /// Cache a decision
    pub async fn set(&self, key: String, allowed: bool) {
        self.allowed.write().await.insert(key, allowed);
    }

    /// Clear the cache
    pub async fn clear(&self) {
        self.allowed.write().await.clear();
    }
}

/// Shared permission handler type
pub type SharedPermissionHandler = Arc<dyn PermissionHandler>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }

    #[test]
    fn test_risk_level_confirmation() {
        assert!(!RiskLevel::Low.requires_confirmation());
        assert!(!RiskLevel::Medium.requires_confirmation());
        assert!(RiskLevel::High.requires_confirmation());
        assert!(RiskLevel::Critical.requires_confirmation());
    }

    #[test]
    fn test_permission_result() {
        assert!(PermissionResult::allow().is_allowed());
        assert!(!PermissionResult::deny("test").is_allowed());
    }

    #[test]
    fn test_tool_context_path_checking() {
        let ctx = ToolContext::new(PathBuf::from("/home/user"))
            .allow_path("/home/user/projects")
            .deny_path("/home/user/.ssh");

        assert!(ctx.is_path_allowed(std::path::Path::new("/home/user/projects/code.rs")));
        assert!(!ctx.is_path_allowed(std::path::Path::new("/home/user/.ssh/id_rsa")));
    }

    #[tokio::test]
    async fn test_auto_allow_handler() {
        let handler = AutoAllowHandler;
        let call = ToolCall::new("1", "test_tool", HashMap::new());
        let request = PermissionRequest::new("test_tool", call, "test", RiskLevel::High);

        let decision = handler.handle_permission_request(request).await;
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_policy_handler() {
        let policy = PermissionPolicy {
            allow_by_default: true,
            max_auto_allow_risk: RiskLevel::Medium,
            tool_overrides: HashMap::new(),
        };

        let handler = PolicyHandler::new(policy);

        // Low risk should be allowed
        let call = ToolCall::new("1", "read_file", HashMap::new());
        let request = PermissionRequest::new("read_file", call, "test", RiskLevel::Low);
        assert!(handler.handle_permission_request(request).await.is_allowed());

        // High risk should also be allowed due to allow_by_default
        let call = ToolCall::new("2", "delete_file", HashMap::new());
        let request = PermissionRequest::new("delete_file", call, "test", RiskLevel::High);
        assert!(handler.handle_permission_request(request).await.is_allowed());
    }

    #[tokio::test]
    async fn test_permission_cache() {
        let cache = PermissionCache::new();

        let call = ToolCall::new("1", "test", HashMap::new());
        let key = PermissionCache::cache_key("test", &call);

        assert!(cache.get(&key).await.is_none());

        cache.set(key.clone(), true).await;
        assert_eq!(cache.get(&key).await, Some(true));

        cache.clear().await;
        assert!(cache.get(&key).await.is_none());
    }
}
