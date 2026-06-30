mod agent_event;
mod helpers;
mod input;
mod outcome;
mod output;

pub use agent_event::notification_from_agent_event;
pub use input::{notification_from_input_request_dto, request_from_input_response_dto};
pub use outcome::{error_type_for_execution_error_kind, terminal_payload_from_execution_outcome};
pub use output::notifications_from_output_event;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCorrelation {
    pub thread_id: String,
    pub turn_id: String,
    pub sequence: u64,
}

impl RuntimeCorrelation {
    pub fn new(thread_id: impl Into<String>, turn_id: impl Into<String>, sequence: u64) -> Self {
        Self {
            thread_id: thread_id.into(),
            turn_id: turn_id.into(),
            sequence,
        }
    }

    pub(super) fn sequence_at(&self, offset: u64) -> u64 {
        self.sequence.saturating_add(offset)
    }
}
