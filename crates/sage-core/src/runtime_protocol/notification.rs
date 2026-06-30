use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::envelope::RuntimeEnvelope;
use super::permission::{
    RuntimePermissionDecision, RuntimePermissionRequestedPayload, RuntimePermissionResolvedPayload,
};

pub type RuntimeNotification = RuntimeEnvelope<RuntimeNotificationPayload>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuntimeNotificationPayload {
    TurnStarted(RuntimeTurnStartedPayload),
    TurnTerminal(RuntimeTurnTerminalPayload),
    Item(RuntimeItemPayload),
    PermissionRequested(RuntimePermissionRequestedPayload),
    PermissionResolved(RuntimePermissionResolvedPayload),
    ErrorReported(RuntimeErrorReportedPayload),
    ThreadLifecycle(RuntimeThreadLifecyclePayload),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTurnStatus {
    Completed,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeItemType {
    Message,
    SystemMessage,
    AssistantMessage,
    UserMessage,
    ToolCall,
    Permission,
    Error,
    Result,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeMessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeItemStatus {
    Started,
    Completed,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RuntimeThreadLifecyclePayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTurnStartedPayload {
    pub input_item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTurnTerminalPayload {
    pub status: RuntimeTurnStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeItemPayload {
    pub item_type: RuntimeItemType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<RuntimeMessageRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<RuntimeItemStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted: Option<bool>,
}

impl RuntimeItemPayload {
    pub fn new(item_type: RuntimeItemType) -> Self {
        Self {
            item_type,
            role: None,
            content: None,
            tool_name: None,
            status: None,
            arguments: None,
            success: None,
            output_preview: None,
            result: None,
            duration_ms: None,
            truncated: None,
            code: None,
            message: None,
            legacy_type: None,
            redacted: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeErrorReportedPayload {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted: Option<bool>,
}

impl From<RuntimePermissionDecision> for RuntimeTurnStatus {
    fn from(decision: RuntimePermissionDecision) -> Self {
        match decision {
            RuntimePermissionDecision::Allow => Self::Completed,
            RuntimePermissionDecision::Deny => Self::Failed,
        }
    }
}
