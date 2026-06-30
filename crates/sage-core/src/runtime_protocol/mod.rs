//! Sage runtime protocol v0 DTOs and compatibility mappings.
//!
//! This module defines the stable `sage.runtime.v0` envelope used by future
//! ThreadStore, runtime facade, and subagent graph work. It does not change the
//! existing `--stream-json` wire format.

pub mod envelope;
pub mod error;
pub mod notification;
pub mod permission;
pub mod request;
pub mod response;
pub mod stream_mapping;

#[cfg(test)]
mod tests;

pub use envelope::{
    RuntimeEnvelope, RuntimeKind, RuntimeMessage, RuntimeMetadata, RuntimeProtocolVersion,
    RuntimeSource,
};
pub use error::{RuntimeError, RuntimeErrorPayload, RuntimeErrorType};
pub use notification::{
    RuntimeErrorReportedPayload, RuntimeItemPayload, RuntimeItemStatus, RuntimeItemType,
    RuntimeMessageRole, RuntimeNotification, RuntimeNotificationPayload,
    RuntimeThreadLifecyclePayload, RuntimeTurnStartedPayload, RuntimeTurnStatus,
    RuntimeTurnTerminalPayload,
};
pub use permission::{
    RuntimePermissionDecision, RuntimePermissionRequestedPayload, RuntimePermissionResolvedPayload,
    RuntimePermissionRisk, RuntimeRule, RuntimeRuleBehavior, RuntimeRuleSource,
};
pub use request::{
    RuntimeExecutionMode, RuntimeForkMode, RuntimeInputRespondPayload,
    RuntimePermissionRespondPayload, RuntimeRequest, RuntimeRequestPayload,
    RuntimeThreadForkPayload, RuntimeThreadResumePayload, RuntimeThreadStartPayload,
    RuntimeTurnInterruptPayload, RuntimeTurnStartPayload, RuntimeTurnSteerPayload,
};
pub use response::{
    RuntimeAckResponsePayload, RuntimeResponse, RuntimeResponsePayload,
    RuntimeThreadResponsePayload, RuntimeTurnResponsePayload,
};
pub use stream_mapping::{
    RuntimeCorrelation, error_type_for_execution_error_kind, notification_from_agent_event,
    notification_from_input_request_dto, notifications_from_output_event,
    request_from_input_response_dto, terminal_payload_from_execution_outcome,
};

pub const RUNTIME_PROTOCOL_VERSION: &str = "sage.runtime.v0";
