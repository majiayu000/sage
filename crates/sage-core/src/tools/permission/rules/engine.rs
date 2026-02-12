//! Permission rule engine for evaluating rules

use serde::{Deserialize, Serialize};

use super::rule::PermissionRule;
use crate::tools::permission::request::{PermissionRequest, ToolPermissionResult};
use crate::tools::permission::types::{PermissionBehavior, RiskLevel};

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
        let path = request
            .call
            .arguments
            .get("file_path")
            .or_else(|| request.call.arguments.get("path"))
            .and_then(|v| v.as_str());

        // Extract command from bash tool arguments
        let command = request
            .call
            .arguments
            .get("command")
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
    /// Convert to ToolPermissionResult
    pub fn to_result(&self, risk_level: RiskLevel) -> ToolPermissionResult {
        match self.behavior {
            PermissionBehavior::Allow => ToolPermissionResult::Allow,
            PermissionBehavior::Deny => ToolPermissionResult::Deny {
                reason: self
                    .reason
                    .clone()
                    .unwrap_or_else(|| "Denied by rule".to_string()),
            },
            PermissionBehavior::Ask | PermissionBehavior::Passthrough => {
                ToolPermissionResult::Ask {
                    question: self
                        .reason
                        .clone()
                        .unwrap_or_else(|| "Permission required".to_string()),
                    default: false,
                    risk_level,
                }
            }
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
