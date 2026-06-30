use crate::runtime::RuntimeOperation;
use crate::runtime_protocol::{
    RuntimeEnvelope, RuntimeForkMode, RuntimeKind, RuntimeRequest, RuntimeRequestPayload,
    RuntimeSource, RuntimeThreadForkPayload, RuntimeThreadResumePayload,
    RuntimeTurnInterruptPayload, RuntimeTurnStartPayload,
};
use crate::types::TaskMetadata;

#[derive(Debug, Clone)]
pub struct RuntimeStartRequest {
    pub task: TaskMetadata,
    pub protocol_request: RuntimeRequest,
}

impl RuntimeStartRequest {
    pub fn new(task: TaskMetadata, source: RuntimeSource) -> Self {
        let request_id = format!("req_turn_start_{}", task.id);
        let protocol_request = RuntimeEnvelope::new(
            RuntimeKind::Request,
            "turn.start",
            request_id.clone(),
            chrono::Utc::now(),
            source,
            RuntimeRequestPayload::TurnStart(RuntimeTurnStartPayload {
                input: task.description.clone(),
                input_item_id: None,
            }),
        )
        .with_request_id(request_id)
        .with_turn_id(task.id.to_string())
        .into();

        Self {
            task,
            protocol_request,
        }
    }

    pub fn operation(&self) -> RuntimeOperation {
        RuntimeOperation::Start
    }

    pub fn turn_id(&self) -> String {
        self.task.id.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeResumeRequest {
    pub thread_id: String,
    pub restore_latest: bool,
}

impl RuntimeResumeRequest {
    pub fn protocol_request(&self, source: RuntimeSource) -> RuntimeRequest {
        RuntimeEnvelope::new(
            RuntimeKind::Request,
            "thread.resume",
            format!("req_thread_resume_{}", self.thread_id),
            chrono::Utc::now(),
            source,
            RuntimeRequestPayload::ThreadResume(RuntimeThreadResumePayload {
                thread_id: self.thread_id.clone(),
                restore_latest: Some(self.restore_latest),
            }),
        )
        .with_thread_id(self.thread_id.clone())
        .into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeForkRequest {
    pub parent_thread_id: String,
    pub parent_turn_id: Option<String>,
    pub parent_item_id: Option<String>,
    pub fork_mode: Option<RuntimeForkMode>,
}

impl RuntimeForkRequest {
    pub fn protocol_request(&self, source: RuntimeSource) -> RuntimeRequest {
        RuntimeEnvelope::new(
            RuntimeKind::Request,
            "thread.fork",
            format!("req_thread_fork_{}", self.parent_thread_id),
            chrono::Utc::now(),
            source,
            RuntimeRequestPayload::ThreadFork(RuntimeThreadForkPayload {
                parent_thread_id: self.parent_thread_id.clone(),
                parent_turn_id: self.parent_turn_id.clone(),
                parent_item_id: self.parent_item_id.clone(),
                fork_mode: self.fork_mode,
            }),
        )
        .with_thread_id(self.parent_thread_id.clone())
        .into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeInterruptRequest {
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub reason: Option<String>,
}

impl RuntimeInterruptRequest {
    pub fn protocol_request(&self, source: RuntimeSource) -> RuntimeRequest {
        let mut envelope = RuntimeEnvelope::new(
            RuntimeKind::Request,
            "turn.interrupt",
            format!("req_turn_interrupt_{}", self.thread_id),
            chrono::Utc::now(),
            source,
            RuntimeRequestPayload::TurnInterrupt(RuntimeTurnInterruptPayload {
                reason: self.reason.clone(),
            }),
        )
        .with_thread_id(self.thread_id.clone());

        if let Some(turn_id) = &self.turn_id {
            envelope = envelope.with_turn_id(turn_id.clone());
        }

        envelope.into()
    }
}
