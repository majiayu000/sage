use std::sync::Arc;

use crate::agent::ExecutionOptions;
use crate::config::Config;
use crate::runtime::{
    Runtime, RuntimeForkRequest, RuntimeInterruptRequest, RuntimeOperation, RuntimeResumeRequest,
    RuntimeStateMode,
};
use crate::runtime_protocol::{RuntimeKind, RuntimeRequestPayload, RuntimeSource};
use crate::thread_store::{SqliteThreadStore, ThreadItemInput, ThreadRecord, ThreadStore};

#[test]
fn runtime_start_request_uses_protocol_turn_start() {
    let runtime = Runtime::new(Config::default(), ExecutionOptions::default())
        .with_source(RuntimeSource::Cli);
    let request = runtime.start_request("write tests", "/tmp/project");

    assert_eq!(request.operation(), RuntimeOperation::Start);
    assert_eq!(request.protocol_request.kind, RuntimeKind::Request);
    assert_eq!(request.protocol_request.message_type, "turn.start");
    assert_eq!(request.protocol_request.source, RuntimeSource::Cli);
    assert_eq!(request.task.description, "write tests");
    assert_eq!(request.task.working_dir, "/tmp/project");
    match &request.protocol_request.payload {
        RuntimeRequestPayload::TurnStart(payload) => {
            assert_eq!(payload.input, "write tests");
        }
        other => panic!("unexpected payload: {other:?}"),
    }
}

#[test]
fn runtime_unsupported_operations_return_structured_errors() {
    let runtime = Runtime::new(Config::default(), ExecutionOptions::default())
        .with_source(RuntimeSource::Sdk);

    let fork_error = runtime
        .fork(RuntimeForkRequest {
            parent_thread_id: "thread-a".to_string(),
            parent_turn_id: None,
            parent_item_id: None,
            fork_mode: None,
        })
        .unwrap_err();
    assert_eq!(fork_error.kind, RuntimeKind::Error);
    assert_eq!(fork_error.message_type, "error.validation");
    assert_eq!(fork_error.payload.code, "unsupported_operation");

    let interrupt_error = runtime
        .interrupt(RuntimeInterruptRequest {
            thread_id: "thread-a".to_string(),
            turn_id: Some("turn-a".to_string()),
            reason: Some("test".to_string()),
        })
        .unwrap_err();
    assert_eq!(interrupt_error.kind, RuntimeKind::Error);
    assert_eq!(interrupt_error.message_type, "error.validation");
    assert_eq!(interrupt_error.payload.code, "unsupported_operation");
}

#[test]
fn runtime_resume_fails_closed_without_fake_success() {
    let runtime = Runtime::new(Config::default(), ExecutionOptions::default())
        .with_source(RuntimeSource::Sdk);

    let err = runtime
        .resume(RuntimeResumeRequest {
            thread_id: "missing-thread".to_string(),
            restore_latest: false,
        })
        .unwrap_err();

    assert_eq!(err.kind, RuntimeKind::Error);
    assert_eq!(err.message_type, "error.validation");
    assert_eq!(err.payload.code, "unsupported_operation");
    assert!(err.payload.message.contains("runtime resume"));
    assert!(err.payload.message.contains("missing-thread"));
}

#[tokio::test]
async fn runtime_status_without_thread_store_fails_closed() {
    let runtime = Runtime::new(Config::default(), ExecutionOptions::default());
    assert_eq!(
        runtime.state_capabilities().mode,
        RuntimeStateMode::Ephemeral
    );

    let err = runtime.status("thread-a").await.unwrap_err();
    assert_eq!(err.kind, RuntimeKind::Error);
    assert_eq!(err.message_type, "error.validation");
    assert_eq!(err.payload.code, "unsupported_operation");
}

#[tokio::test]
async fn runtime_status_missing_thread_is_not_reported_as_unsupported() {
    let store = Arc::new(SqliteThreadStore::in_memory().unwrap());
    let runtime = Runtime::new(Config::default(), ExecutionOptions::default())
        .with_thread_store(store as Arc<dyn ThreadStore>);

    let err = runtime.status("missing-thread").await.unwrap_err();

    assert_eq!(err.kind, RuntimeKind::Error);
    assert_eq!(err.message_type, "error.validation");
    assert_eq!(err.payload.code, "thread_not_found");
    assert!(err.payload.message.contains("missing-thread"));
}

#[tokio::test]
async fn runtime_status_reads_thread_store_when_configured() {
    let store = Arc::new(SqliteThreadStore::in_memory().unwrap());
    store
        .create_thread(ThreadRecord::new("thread-a"))
        .await
        .unwrap();
    store
        .append_event("thread-a", Some("turn-a"), ThreadItemInput::new("message"))
        .await
        .unwrap();

    let runtime = Runtime::new(Config::default(), ExecutionOptions::default())
        .with_thread_store(store as Arc<dyn ThreadStore>);
    let status = runtime.status("thread-a").await.unwrap();

    assert_eq!(status.thread_id, "thread-a");
    assert_eq!(status.state.mode, RuntimeStateMode::ThreadStore);
    assert_eq!(status.turn_count, 1);
    assert_eq!(status.item_count, 1);
}
