use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ops::Deref;
use thiserror::Error;

use super::envelope::{RuntimeEnvelope, RuntimeKind};
use super::permission::{
    RuntimePermissionDecision, RuntimePermissionRequestedPayload, RuntimePermissionResolvedPayload,
};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct RuntimeNotification(pub RuntimeEnvelope<RuntimeNotificationPayload>);

impl From<RuntimeEnvelope<RuntimeNotificationPayload>> for RuntimeNotification {
    fn from(envelope: RuntimeEnvelope<RuntimeNotificationPayload>) -> Self {
        Self(envelope)
    }
}

impl Deref for RuntimeNotification {
    type Target = RuntimeEnvelope<RuntimeNotificationPayload>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RuntimeNotification {
    pub fn with_metadata(self, key: impl Into<String>, value: Value) -> Self {
        self.0.with_metadata(key, value).into()
    }

    pub fn with_request_id(self, request_id: impl Into<String>) -> Self {
        self.0.with_request_id(request_id).into()
    }
}

impl<'de> Deserialize<'de> for RuntimeNotification {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RuntimeEnvelope::<Value>::deserialize(deserializer)?;
        if raw.kind != RuntimeKind::Notification {
            return Err(serde::de::Error::custom(
                "runtime notification kind mismatch",
            ));
        }

        let payload = notification_payload_from_type(&raw.message_type, raw.payload)
            .map_err(serde::de::Error::custom)?;

        Ok(RuntimeNotification(RuntimeEnvelope {
            protocol_version: raw.protocol_version,
            kind: raw.kind,
            message_type: raw.message_type,
            id: raw.id,
            thread_id: raw.thread_id,
            turn_id: raw.turn_id,
            item_id: raw.item_id,
            request_id: raw.request_id,
            timestamp: raw.timestamp,
            sequence: raw.sequence,
            source: raw.source,
            payload,
            metadata: raw.metadata,
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum RuntimeNotificationPayload {
    TurnStarted(RuntimeTurnStartedPayload),
    TurnTerminal(RuntimeTurnTerminalPayload),
    Item(RuntimeItemPayload),
    PermissionRequested(RuntimePermissionRequestedPayload),
    PermissionResolved(RuntimePermissionResolvedPayload),
    ErrorReported(RuntimeErrorReportedPayload),
    ThreadLifecycle(RuntimeThreadLifecyclePayload),
}

#[derive(Debug, Error)]
enum RuntimeNotificationPayloadDecodeError {
    #[error("unsupported runtime notification type {0}")]
    UnsupportedType(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

fn notification_payload_from_type(
    message_type: &str,
    payload: Value,
) -> Result<RuntimeNotificationPayload, RuntimeNotificationPayloadDecodeError> {
    match message_type {
        "thread.started" | "thread.ended" => {
            decode_payload(payload, RuntimeNotificationPayload::ThreadLifecycle)
        }
        "turn.started" => decode_payload(payload, RuntimeNotificationPayload::TurnStarted),
        "turn.completed" | "turn.interrupted" => {
            decode_payload(payload, RuntimeNotificationPayload::TurnTerminal)
        }
        "item.created" | "item.updated" | "item.completed" => {
            decode_payload(payload, RuntimeNotificationPayload::Item)
        }
        "permission.requested" => {
            decode_payload(payload, RuntimeNotificationPayload::PermissionRequested)
        }
        "permission.resolved" => {
            decode_payload(payload, RuntimeNotificationPayload::PermissionResolved)
        }
        "error.reported" => decode_payload(payload, RuntimeNotificationPayload::ErrorReported),
        _ => Err(RuntimeNotificationPayloadDecodeError::UnsupportedType(
            message_type.to_string(),
        )),
    }
}

fn decode_payload<T>(
    payload: Value,
    wrap: impl FnOnce(T) -> RuntimeNotificationPayload,
) -> Result<RuntimeNotificationPayload, RuntimeNotificationPayloadDecodeError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(payload)
        .map(wrap)
        .map_err(Into::into)
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
