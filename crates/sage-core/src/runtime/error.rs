use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::runtime_protocol::{
    RuntimeEnvelope, RuntimeError, RuntimeErrorPayload, RuntimeErrorType, RuntimeKind,
    RuntimeSource,
};

pub type RuntimeControlResult<T> = Result<T, Box<RuntimeError>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeOperation {
    Start,
    Resume,
    Fork,
    Interrupt,
    Status,
}

impl RuntimeOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Resume => "resume",
            Self::Fork => "fork",
            Self::Interrupt => "interrupt",
            Self::Status => "status",
        }
    }
}

pub fn runtime_unsupported_error(
    operation: RuntimeOperation,
    source: RuntimeSource,
    message: impl Into<String>,
) -> RuntimeError {
    runtime_validation_error(operation, source, "unsupported_operation", message)
}

pub fn runtime_validation_error(
    operation: RuntimeOperation,
    source: RuntimeSource,
    code: impl Into<String>,
    message: impl Into<String>,
) -> RuntimeError {
    RuntimeEnvelope::new(
        RuntimeKind::Error,
        RuntimeErrorType::Validation.message_type(),
        format!("err_runtime_{}", operation.as_str()),
        chrono::Utc::now(),
        source,
        RuntimeErrorPayload {
            code: code.into(),
            message: message.into(),
            details: Some(json!({ "operation": operation.as_str() })),
            redacted: Some(false),
        },
    )
    .into()
}

pub fn boxed_runtime_unsupported_error(
    operation: RuntimeOperation,
    source: RuntimeSource,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    Box::new(runtime_unsupported_error(operation, source, message))
}

pub fn boxed_runtime_validation_error(
    operation: RuntimeOperation,
    source: RuntimeSource,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    Box::new(runtime_validation_error(operation, source, code, message))
}
