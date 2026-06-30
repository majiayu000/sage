use crate::agent::{ExecutionErrorKind, ExecutionOutcome};

use crate::runtime_protocol::error::RuntimeErrorType;
use crate::runtime_protocol::notification::{RuntimeTurnStatus, RuntimeTurnTerminalPayload};

pub fn terminal_payload_from_execution_outcome(
    outcome: &ExecutionOutcome,
) -> RuntimeTurnTerminalPayload {
    match outcome {
        ExecutionOutcome::Success(execution) => RuntimeTurnTerminalPayload {
            status: RuntimeTurnStatus::Completed,
            reason: None,
            result: execution.final_result.clone(),
            duration_ms: None,
            legacy_type: None,
        },
        ExecutionOutcome::Failed { error, .. } => RuntimeTurnTerminalPayload {
            status: RuntimeTurnStatus::Failed,
            reason: Some(
                error_type_for_execution_error_kind(&error.kind)
                    .code()
                    .to_string(),
            ),
            result: Some(error.message.clone()),
            duration_ms: None,
            legacy_type: None,
        },
        ExecutionOutcome::Interrupted { .. } => RuntimeTurnTerminalPayload {
            status: RuntimeTurnStatus::Interrupted,
            reason: Some(RuntimeErrorType::Interrupted.code().to_string()),
            result: None,
            duration_ms: None,
            legacy_type: None,
        },
        ExecutionOutcome::MaxStepsReached { .. } => RuntimeTurnTerminalPayload {
            status: RuntimeTurnStatus::Failed,
            reason: Some(RuntimeErrorType::MaxSteps.code().to_string()),
            result: None,
            duration_ms: None,
            legacy_type: None,
        },
        ExecutionOutcome::UserCancelled {
            pending_question, ..
        } => RuntimeTurnTerminalPayload {
            status: RuntimeTurnStatus::Interrupted,
            reason: Some("user_cancelled".to_string()),
            result: pending_question.clone(),
            duration_ms: None,
            legacy_type: None,
        },
        ExecutionOutcome::NeedsUserInput { last_response, .. } => RuntimeTurnTerminalPayload {
            status: RuntimeTurnStatus::Interrupted,
            reason: Some("needs_user_input".to_string()),
            result: Some(last_response.clone()),
            duration_ms: None,
            legacy_type: None,
        },
    }
}

pub fn error_type_for_execution_error_kind(kind: &ExecutionErrorKind) -> RuntimeErrorType {
    match kind {
        ExecutionErrorKind::InvalidRequest | ExecutionErrorKind::Configuration => {
            RuntimeErrorType::Validation
        }
        ExecutionErrorKind::ToolExecution { .. } => RuntimeErrorType::ToolFailed,
        ExecutionErrorKind::Authentication
        | ExecutionErrorKind::RateLimit
        | ExecutionErrorKind::ServiceUnavailable
        | ExecutionErrorKind::Network
        | ExecutionErrorKind::Timeout => RuntimeErrorType::ModelFailed,
        ExecutionErrorKind::Other => RuntimeErrorType::Internal,
    }
}
