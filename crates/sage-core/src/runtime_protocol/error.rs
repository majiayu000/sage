use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ops::Deref;
use thiserror::Error;

use super::envelope::{RuntimeEnvelope, RuntimeKind};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct RuntimeError(pub RuntimeEnvelope<RuntimeErrorPayload>);

impl From<RuntimeEnvelope<RuntimeErrorPayload>> for RuntimeError {
    fn from(envelope: RuntimeEnvelope<RuntimeErrorPayload>) -> Self {
        Self(envelope)
    }
}

impl Deref for RuntimeError {
    type Target = RuntimeEnvelope<RuntimeErrorPayload>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for RuntimeError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RuntimeEnvelope::<RuntimeErrorPayload>::deserialize(deserializer)?;
        if raw.kind != RuntimeKind::Error {
            return Err(serde::de::Error::custom("runtime error kind mismatch"));
        }
        RuntimeErrorType::from_message_type(&raw.message_type)
            .ok_or_else(|| RuntimeErrorDecodeError::UnsupportedType(raw.message_type.clone()))
            .map_err(serde::de::Error::custom)?;
        Ok(Self(raw))
    }
}

#[derive(Debug, Error)]
enum RuntimeErrorDecodeError {
    #[error("unsupported runtime error type {0}")]
    UnsupportedType(String),
}

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
    pub fn from_message_type(message_type: &str) -> Option<Self> {
        match message_type {
            "error.validation" => Some(Self::Validation),
            "error.permission_denied" => Some(Self::PermissionDenied),
            "error.tool_failed" => Some(Self::ToolFailed),
            "error.model_failed" => Some(Self::ModelFailed),
            "error.interrupted" => Some(Self::Interrupted),
            "error.max_steps" => Some(Self::MaxSteps),
            "error.internal" => Some(Self::Internal),
            _ => None,
        }
    }

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
