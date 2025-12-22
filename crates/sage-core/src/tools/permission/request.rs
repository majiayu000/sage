//! Permission request and response types

use serde_json::Value;
use std::collections::HashMap;

use super::types::RiskLevel;
use crate::tools::types::ToolCall;

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
