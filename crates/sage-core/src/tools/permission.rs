//! Permission system for tool execution
//!
//! This module provides a permission checking framework for tools, including:
//! - Risk level assessment
//! - Permission decisions (allow, deny, ask)
//! - Permission handlers for user interaction
//! - Tool execution context
//! - Rule-based permission matching (OpenClaude compatible)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::ToolCall;
use crate::hooks::matcher::matches as pattern_matches;

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
        matches!(self, Self::Allow | Self::AllowAlways | Self::Modify { .. })
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
    async fn handle_permission_request(&self, request: PermissionRequest) -> PermissionDecision;
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

// ============================================================================
// Rule-based Permission System (OpenClaude compatible)
// ============================================================================

/// Source of a permission rule
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleSource {
    /// Project settings (.sage/settings.json or .claude/settings.json)
    ProjectSettings,
    /// Local project settings (.sage/settings.local.json)
    LocalSettings,
    /// User-level settings (~/.config/sage/settings.json)
    UserSettings,
    /// Session-level settings (runtime)
    SessionSettings,
    /// Command line argument
    CliArg,
    /// Builtin default rules
    Builtin,
}

impl RuleSource {
    /// Get the priority of this rule source (lower = higher priority)
    pub fn priority(&self) -> u8 {
        match self {
            RuleSource::CliArg => 0,         // Highest priority
            RuleSource::SessionSettings => 1,
            RuleSource::LocalSettings => 2,
            RuleSource::ProjectSettings => 3,
            RuleSource::UserSettings => 4,
            RuleSource::Builtin => 5,        // Lowest priority
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            RuleSource::ProjectSettings => "project settings",
            RuleSource::LocalSettings => "local settings",
            RuleSource::UserSettings => "user settings",
            RuleSource::SessionSettings => "session settings",
            RuleSource::CliArg => "command line",
            RuleSource::Builtin => "builtin",
        }
    }
}

impl fmt::Display for RuleSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Default for RuleSource {
    fn default() -> Self {
        RuleSource::Builtin
    }
}

/// Permission behavior for a rule
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionBehavior {
    /// Allow the operation
    Allow,
    /// Deny the operation
    Deny,
    /// Ask the user
    Ask,
    /// Pass through to next rule (no decision)
    Passthrough,
}

impl fmt::Display for PermissionBehavior {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PermissionBehavior::Allow => write!(f, "allow"),
            PermissionBehavior::Deny => write!(f, "deny"),
            PermissionBehavior::Ask => write!(f, "ask"),
            PermissionBehavior::Passthrough => write!(f, "passthrough"),
        }
    }
}

impl Default for PermissionBehavior {
    fn default() -> Self {
        PermissionBehavior::Ask
    }
}

/// A permission rule with pattern matchers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    /// Source of this rule
    #[serde(default)]
    pub source: RuleSource,
    /// Tool name pattern (e.g., "bash", "edit|write", "^file_.*")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_pattern: Option<String>,
    /// File path pattern (for file tools like read, write, edit)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_pattern: Option<String>,
    /// Command pattern (for bash tool)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_pattern: Option<String>,
    /// The permission behavior
    pub behavior: PermissionBehavior,
    /// Optional reason for this rule
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Whether this rule is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl PermissionRule {
    /// Create a new permission rule
    pub fn new(behavior: PermissionBehavior) -> Self {
        Self {
            source: RuleSource::default(),
            tool_pattern: None,
            path_pattern: None,
            command_pattern: None,
            behavior,
            reason: None,
            enabled: true,
        }
    }

    /// Set the rule source
    pub fn with_source(mut self, source: RuleSource) -> Self {
        self.source = source;
        self
    }

    /// Set the tool pattern
    pub fn with_tool_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.tool_pattern = Some(pattern.into());
        self
    }

    /// Set the path pattern
    pub fn with_path_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.path_pattern = Some(pattern.into());
        self
    }

    /// Set the command pattern
    pub fn with_command_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.command_pattern = Some(pattern.into());
        self
    }

    /// Set the reason
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Check if this rule matches a given tool call
    pub fn matches(&self, tool_name: &str, path: Option<&str>, command: Option<&str>) -> bool {
        if !self.enabled {
            return false;
        }

        // Tool name must match if pattern is specified
        if !pattern_matches(self.tool_pattern.as_deref(), tool_name) {
            return false;
        }

        // Path must match if pattern is specified and path is provided
        if self.path_pattern.is_some() {
            match path {
                Some(p) => {
                    if !pattern_matches(self.path_pattern.as_deref(), p) {
                        return false;
                    }
                }
                None => return false, // Path pattern specified but no path provided
            }
        }

        // Command must match if pattern is specified and command is provided
        if self.command_pattern.is_some() {
            match command {
                Some(c) => {
                    if !pattern_matches(self.command_pattern.as_deref(), c) {
                        return false;
                    }
                }
                None => return false, // Command pattern specified but no command provided
            }
        }

        true
    }
}

impl fmt::Display for PermissionRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (source: {})", self.behavior, self.source)?;
        if let Some(ref tool) = self.tool_pattern {
            write!(f, " tool={}", tool)?;
        }
        if let Some(ref path) = self.path_pattern {
            write!(f, " path={}", path)?;
        }
        if let Some(ref cmd) = self.command_pattern {
            write!(f, " cmd={}", cmd)?;
        }
        Ok(())
    }
}

/// Permission rule engine for evaluating rules
#[derive(Debug, Clone, Default)]
pub struct PermissionRuleEngine {
    /// Ordered rules (evaluated in order, first match wins)
    rules: Vec<PermissionRule>,
    /// Default behavior when no rule matches
    default_behavior: PermissionBehavior,
}

impl PermissionRuleEngine {
    /// Create a new permission rule engine
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            default_behavior: PermissionBehavior::Ask,
        }
    }

    /// Create with a custom default behavior
    pub fn with_default(default_behavior: PermissionBehavior) -> Self {
        Self {
            rules: Vec::new(),
            default_behavior,
        }
    }

    /// Add a rule to the engine
    pub fn add_rule(&mut self, rule: PermissionRule) {
        self.rules.push(rule);
    }

    /// Add multiple rules
    pub fn add_rules(&mut self, rules: impl IntoIterator<Item = PermissionRule>) {
        self.rules.extend(rules);
    }

    /// Sort rules by source priority (lower priority number = higher priority)
    pub fn sort_by_priority(&mut self) {
        self.rules.sort_by_key(|r| r.source.priority());
    }

    /// Evaluate permission for a tool call
    pub fn evaluate(
        &self,
        tool_name: &str,
        path: Option<&str>,
        command: Option<&str>,
    ) -> PermissionEvaluation {
        for rule in &self.rules {
            if rule.matches(tool_name, path, command) {
                match rule.behavior {
                    PermissionBehavior::Passthrough => continue,
                    behavior => {
                        return PermissionEvaluation {
                            behavior,
                            matched_rule: Some(rule.clone()),
                            reason: rule.reason.clone(),
                        };
                    }
                }
            }
        }

        // No matching rule, use default
        PermissionEvaluation {
            behavior: self.default_behavior,
            matched_rule: None,
            reason: Some("No matching rule found, using default".to_string()),
        }
    }

    /// Evaluate permission for a PermissionRequest
    pub fn evaluate_request(&self, request: &PermissionRequest) -> PermissionEvaluation {
        // Extract path from common tool arguments
        let path = request.call.arguments.get("file_path")
            .or_else(|| request.call.arguments.get("path"))
            .and_then(|v| v.as_str());

        // Extract command from bash tool arguments
        let command = request.call.arguments.get("command")
            .and_then(|v| v.as_str());

        self.evaluate(&request.tool_name, path, command)
    }

    /// Get all rules
    pub fn rules(&self) -> &[PermissionRule] {
        &self.rules
    }

    /// Get the number of rules
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Check if the engine has no rules
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Clear all rules
    pub fn clear(&mut self) {
        self.rules.clear();
    }

    /// Load rules from configuration
    pub fn from_config(config: &PermissionRulesConfig) -> Self {
        let mut engine = Self::new();
        engine.add_rules(config.rules.iter().cloned());
        engine.sort_by_priority();
        engine
    }
}

/// Result of permission evaluation
#[derive(Debug, Clone)]
pub struct PermissionEvaluation {
    /// The determined behavior
    pub behavior: PermissionBehavior,
    /// The rule that matched (if any)
    pub matched_rule: Option<PermissionRule>,
    /// Reason for this evaluation
    pub reason: Option<String>,
}

impl PermissionEvaluation {
    /// Convert to PermissionResult
    pub fn to_result(&self, risk_level: RiskLevel) -> PermissionResult {
        match self.behavior {
            PermissionBehavior::Allow => PermissionResult::Allow,
            PermissionBehavior::Deny => PermissionResult::Deny {
                reason: self.reason.clone().unwrap_or_else(|| "Denied by rule".to_string()),
            },
            PermissionBehavior::Ask | PermissionBehavior::Passthrough => PermissionResult::Ask {
                question: self.reason.clone().unwrap_or_else(|| "Permission required".to_string()),
                default: false,
                risk_level,
            },
        }
    }

    /// Check if the evaluation allows the operation
    pub fn is_allowed(&self) -> bool {
        self.behavior == PermissionBehavior::Allow
    }
}

/// Configuration for permission rules
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionRulesConfig {
    /// List of permission rules
    #[serde(default)]
    pub rules: Vec<PermissionRule>,
}

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
        assert!(
            handler
                .handle_permission_request(request)
                .await
                .is_allowed()
        );

        // High risk should also be allowed due to allow_by_default
        let call = ToolCall::new("2", "delete_file", HashMap::new());
        let request = PermissionRequest::new("delete_file", call, "test", RiskLevel::High);
        assert!(
            handler
                .handle_permission_request(request)
                .await
                .is_allowed()
        );
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

    // ===== Rule-based Permission Tests =====

    #[test]
    fn test_rule_source_priority() {
        assert!(RuleSource::CliArg.priority() < RuleSource::SessionSettings.priority());
        assert!(RuleSource::SessionSettings.priority() < RuleSource::LocalSettings.priority());
        assert!(RuleSource::LocalSettings.priority() < RuleSource::ProjectSettings.priority());
        assert!(RuleSource::ProjectSettings.priority() < RuleSource::UserSettings.priority());
        assert!(RuleSource::UserSettings.priority() < RuleSource::Builtin.priority());
    }

    #[test]
    fn test_rule_source_display() {
        assert_eq!(format!("{}", RuleSource::CliArg), "command line");
        assert_eq!(format!("{}", RuleSource::ProjectSettings), "project settings");
    }

    #[test]
    fn test_permission_behavior_display() {
        assert_eq!(format!("{}", PermissionBehavior::Allow), "allow");
        assert_eq!(format!("{}", PermissionBehavior::Deny), "deny");
        assert_eq!(format!("{}", PermissionBehavior::Ask), "ask");
        assert_eq!(format!("{}", PermissionBehavior::Passthrough), "passthrough");
    }

    #[test]
    fn test_permission_rule_matches_tool() {
        let rule = PermissionRule::new(PermissionBehavior::Allow)
            .with_tool_pattern("bash");

        assert!(rule.matches("bash", None, None));
        assert!(!rule.matches("edit", None, None));
    }

    #[test]
    fn test_permission_rule_matches_tool_pattern() {
        let rule = PermissionRule::new(PermissionBehavior::Allow)
            .with_tool_pattern("edit|write|read");

        assert!(rule.matches("edit", None, None));
        assert!(rule.matches("write", None, None));
        assert!(rule.matches("read", None, None));
        assert!(!rule.matches("bash", None, None));
    }

    #[test]
    fn test_permission_rule_matches_path() {
        let rule = PermissionRule::new(PermissionBehavior::Deny)
            .with_tool_pattern("edit|write")
            .with_path_pattern(".*\\.env.*");

        // Should match .env files
        assert!(rule.matches("edit", Some("/path/to/.env"), None));
        assert!(rule.matches("write", Some("/path/to/.env.local"), None));

        // Should not match regular files
        assert!(!rule.matches("edit", Some("/path/to/code.rs"), None));

        // Should not match if no path provided when path pattern exists
        assert!(!rule.matches("edit", None, None));
    }

    #[test]
    fn test_permission_rule_matches_command() {
        let rule = PermissionRule::new(PermissionBehavior::Deny)
            .with_tool_pattern("bash")
            .with_command_pattern(".*rm.*-rf.*");

        // Should match dangerous commands
        assert!(rule.matches("bash", None, Some("rm -rf /")));
        assert!(rule.matches("bash", None, Some("sudo rm -rf /tmp")));

        // Should not match safe commands
        assert!(!rule.matches("bash", None, Some("ls -la")));
        assert!(!rule.matches("bash", None, Some("cat file.txt")));
    }

    #[test]
    fn test_permission_rule_disabled() {
        let mut rule = PermissionRule::new(PermissionBehavior::Allow)
            .with_tool_pattern("bash");
        rule.enabled = false;

        assert!(!rule.matches("bash", None, None));
    }

    #[test]
    fn test_permission_rule_engine_evaluate() {
        let mut engine = PermissionRuleEngine::new();

        // Add rules
        engine.add_rule(
            PermissionRule::new(PermissionBehavior::Deny)
                .with_tool_pattern("bash")
                .with_command_pattern(".*rm.*-rf.*")
                .with_reason("Dangerous command")
        );
        engine.add_rule(
            PermissionRule::new(PermissionBehavior::Allow)
                .with_tool_pattern("read|glob|grep")
        );

        // Dangerous bash command should be denied
        let eval = engine.evaluate("bash", None, Some("rm -rf /"));
        assert_eq!(eval.behavior, PermissionBehavior::Deny);
        assert!(eval.matched_rule.is_some());

        // Safe read should be allowed
        let eval = engine.evaluate("read", None, None);
        assert_eq!(eval.behavior, PermissionBehavior::Allow);

        // Unknown tool should get default behavior
        let eval = engine.evaluate("unknown_tool", None, None);
        assert_eq!(eval.behavior, PermissionBehavior::Ask); // Default
    }

    #[test]
    fn test_permission_rule_engine_sort_by_priority() {
        let mut engine = PermissionRuleEngine::new();

        engine.add_rule(
            PermissionRule::new(PermissionBehavior::Allow)
                .with_source(RuleSource::Builtin)
                .with_tool_pattern("bash")
        );
        engine.add_rule(
            PermissionRule::new(PermissionBehavior::Deny)
                .with_source(RuleSource::CliArg)
                .with_tool_pattern("bash")
        );

        engine.sort_by_priority();

        // CLI arg should be first (higher priority)
        assert_eq!(engine.rules()[0].source, RuleSource::CliArg);
        assert_eq!(engine.rules()[1].source, RuleSource::Builtin);

        // CLI arg rule should be matched first
        let eval = engine.evaluate("bash", None, None);
        assert_eq!(eval.behavior, PermissionBehavior::Deny);
    }

    #[test]
    fn test_permission_rule_engine_passthrough() {
        let mut engine = PermissionRuleEngine::new();

        // Passthrough rule should be skipped
        engine.add_rule(
            PermissionRule::new(PermissionBehavior::Passthrough)
                .with_tool_pattern("bash")
        );
        engine.add_rule(
            PermissionRule::new(PermissionBehavior::Allow)
                .with_tool_pattern("bash")
        );

        let eval = engine.evaluate("bash", None, None);
        assert_eq!(eval.behavior, PermissionBehavior::Allow);
    }

    #[test]
    fn test_permission_evaluation_to_result() {
        let eval = PermissionEvaluation {
            behavior: PermissionBehavior::Allow,
            matched_rule: None,
            reason: None,
        };
        let result = eval.to_result(RiskLevel::Low);
        assert!(matches!(result, PermissionResult::Allow));

        let eval = PermissionEvaluation {
            behavior: PermissionBehavior::Deny,
            matched_rule: None,
            reason: Some("Test reason".to_string()),
        };
        let result = eval.to_result(RiskLevel::High);
        assert!(matches!(result, PermissionResult::Deny { reason } if reason == "Test reason"));
    }

    #[tokio::test]
    async fn test_rule_based_handler() {
        let mut engine = PermissionRuleEngine::new();
        engine.add_rule(
            PermissionRule::new(PermissionBehavior::Allow)
                .with_tool_pattern("read|glob")
        );
        engine.add_rule(
            PermissionRule::new(PermissionBehavior::Deny)
                .with_tool_pattern("bash")
        );

        let handler = RuleBasedHandler::new(engine);

        // Read should be allowed
        let call = ToolCall::new("1", "read", HashMap::new());
        let request = PermissionRequest::new("read", call, "test", RiskLevel::Low);
        let decision = handler.handle_permission_request(request).await;
        assert_eq!(decision, PermissionDecision::Allow);

        // Bash should be denied
        let call = ToolCall::new("2", "bash", HashMap::new());
        let request = PermissionRequest::new("bash", call, "test", RiskLevel::Medium);
        let decision = handler.handle_permission_request(request).await;
        assert_eq!(decision, PermissionDecision::Deny);
    }

    #[test]
    fn test_permission_rules_config_serialization() {
        let config = PermissionRulesConfig {
            rules: vec![
                PermissionRule::new(PermissionBehavior::Allow)
                    .with_tool_pattern("read|glob|grep")
                    .with_source(RuleSource::Builtin),
                PermissionRule::new(PermissionBehavior::Deny)
                    .with_tool_pattern("bash")
                    .with_command_pattern(".*rm.*-rf.*")
                    .with_reason("Dangerous command"),
            ],
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: PermissionRulesConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.rules.len(), 2);
        assert_eq!(deserialized.rules[0].behavior, PermissionBehavior::Allow);
        assert_eq!(deserialized.rules[1].behavior, PermissionBehavior::Deny);
    }

    #[test]
    fn test_permission_rule_display() {
        let rule = PermissionRule::new(PermissionBehavior::Allow)
            .with_tool_pattern("bash")
            .with_command_pattern("ls.*")
            .with_source(RuleSource::ProjectSettings);

        let display = format!("{}", rule);
        assert!(display.contains("allow"));
        assert!(display.contains("project settings"));
        assert!(display.contains("tool=bash"));
        assert!(display.contains("cmd=ls.*"));
    }
}
