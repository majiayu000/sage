use crate::agent::ExecutionOutcome;
use crate::runtime::RuntimeStartRequest;
use crate::runtime_protocol::{
    RuntimeEnvelope, RuntimeKind, RuntimeNotification, RuntimeNotificationPayload, RuntimeSource,
    RuntimeTurnStatus, terminal_payload_from_execution_outcome,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeProtocolStream {
    enabled: bool,
}

impl RuntimeProtocolStream {
    pub fn enabled() -> Self {
        Self { enabled: true }
    }

    pub fn disabled() -> Self {
        Self { enabled: false }
    }

    pub fn is_enabled(self) -> bool {
        self.enabled
    }

    pub fn notifications_for_result(
        self,
        request: &RuntimeStartRequest,
        outcome: &ExecutionOutcome,
        source: RuntimeSource,
    ) -> Vec<RuntimeNotification> {
        if !self.enabled {
            return Vec::new();
        }

        let payload = terminal_payload_from_execution_outcome(outcome);
        let message_type = match payload.status {
            RuntimeTurnStatus::Completed => "turn.completed",
            RuntimeTurnStatus::Failed => "turn.failed",
            RuntimeTurnStatus::Interrupted => "turn.interrupted",
        };

        vec![
            RuntimeEnvelope::new(
                RuntimeKind::Notification,
                message_type,
                format!("evt_turn_terminal_{}", request.turn_id()),
                chrono::Utc::now(),
                source,
                RuntimeNotificationPayload::TurnTerminal(payload),
            )
            .with_turn_id(request.turn_id())
            .with_sequence(0)
            .into(),
        ]
    }
}

impl Default for RuntimeProtocolStream {
    fn default() -> Self {
        Self::disabled()
    }
}
