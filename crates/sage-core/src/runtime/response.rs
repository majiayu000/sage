use crate::agent::ExecutionOutcome;
use crate::runtime::{RuntimeStartRequest, RuntimeStateCapabilities};
use crate::runtime_protocol::{
    RuntimeEnvelope, RuntimeKind, RuntimeNotification, RuntimeResponse, RuntimeResponsePayload,
    RuntimeSource, RuntimeTurnResponsePayload, RuntimeTurnStatus,
    terminal_payload_from_execution_outcome,
};

pub struct RuntimeRunResult {
    pub outcome: ExecutionOutcome,
    pub request: RuntimeStartRequest,
    pub response: RuntimeResponse,
    pub protocol_notifications: Vec<RuntimeNotification>,
    pub state: RuntimeStateCapabilities,
}

impl RuntimeRunResult {
    pub fn new(
        request: RuntimeStartRequest,
        outcome: ExecutionOutcome,
        protocol_notifications: Vec<RuntimeNotification>,
        state: RuntimeStateCapabilities,
        source: RuntimeSource,
    ) -> Self {
        let terminal = terminal_payload_from_execution_outcome(&outcome);
        let status = match terminal.status {
            RuntimeTurnStatus::Completed => "completed",
            RuntimeTurnStatus::Failed => "failed",
            RuntimeTurnStatus::Interrupted => "interrupted",
        };
        let response = RuntimeEnvelope::new(
            RuntimeKind::Response,
            "turn.start.result",
            format!("res_turn_start_{}", request.turn_id()),
            chrono::Utc::now(),
            source,
            RuntimeResponsePayload::Turn(RuntimeTurnResponsePayload {
                turn_id: request.turn_id(),
                status: Some(status.to_string()),
            }),
        )
        .with_request_id(request.protocol_request.id.clone())
        .with_turn_id(request.turn_id())
        .into();

        Self {
            outcome,
            request,
            response,
            protocol_notifications,
            state,
        }
    }
}
