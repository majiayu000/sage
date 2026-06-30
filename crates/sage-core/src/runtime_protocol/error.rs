use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::envelope::RuntimeEnvelope;

pub type RuntimeError = RuntimeEnvelope<RuntimeErrorPayload>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeErrorType {
    Validation,
    PermissionDenied,
    ToolFailed,
    ModelFailed,
    Interrupted,
    MaxSteps,
    Internal,
}

impl RuntimeErrorType {
    pub fn message_type(self) -> &'static str {
        match self {
            Self::Validation => "error.validation",
            Self::PermissionDenied => "error.permission_denied",
            Self::ToolFailed => "error.tool_failed",
            Self::ModelFailed => "error.model_failed",
            Self::Interrupted => "error.interrupted",
            Self::MaxSteps => "error.max_steps",
            Self::Internal => "error.internal",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Validation => "invalid_request",
            Self::PermissionDenied => "permission_denied",
            Self::ToolFailed => "tool_failed",
            Self::ModelFailed => "model_failed",
            Self::Interrupted => "interrupted",
            Self::MaxSteps => "max_steps",
            Self::Internal => "internal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeErrorPayload {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted: Option<bool>,
}
