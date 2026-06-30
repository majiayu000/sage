use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeRuleBehavior {
    Allow,
    Deny,
    Ask,
    Passthrough,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeRuleSource {
    ProjectSettings,
    LocalSettings,
    UserSettings,
    SessionSettings,
    CliArg,
    Builtin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePermissionDecision {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePermissionRisk {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRule {
    pub behavior: RuntimeRuleBehavior,
    pub source: RuntimeRuleSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePermissionRequestedPayload {
    pub tool_name: String,
    pub risk: RuntimePermissionRisk,
    pub reason: String,
    pub input_redacted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<RuntimeRule>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePermissionResolvedPayload {
    pub decision: RuntimePermissionDecision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_input_applied: Option<bool>,
}
