//! Contract-focused tests for external GUI integration.

use sage_core::input::{InputChannel, InputRequest, InputResponse};
use sage_core::ui::bridge::{AgentEvent, AppStateDto, EventAdapter};

#[tokio::test]
async fn input_channel_roundtrip_preserves_request_id() {
    let (mut channel, mut handle) = InputChannel::new(1);
    let request = InputRequest::free_text("Continue?", "last response");
    let expected_id = request.id;

    let responder = tokio::spawn(async move {
        if let Some(incoming) = handle.request_rx.recv().await {
            let response = InputResponse::free_text(incoming.id, "continue");
            let _ = handle.respond(response).await;
        }
    });

    let response = channel
        .request_input(request)
        .await
        .expect("request_input should succeed");
    responder.await.expect("responder join");

    assert_eq!(response.request_id, expected_id);
    assert_eq!(response.get_text(), Some("continue"));
}

#[test]
fn app_state_dto_snapshot_is_serializable() {
    let adapter = EventAdapter::with_default_state();

    adapter.handle_event(AgentEvent::session_started("s1", "model-a", "provider-a"));
    adapter.handle_event(AgentEvent::UserInputReceived {
        input: "hello".to_string(),
    });
    adapter.handle_event(AgentEvent::ContentStreamStarted);
    adapter.handle_event(AgentEvent::chunk("world"));

    let dto = AppStateDto::from(adapter.get_state());
    let json = serde_json::to_string(&dto).expect("serialize AppStateDto");
    assert!(json.contains("model-a"));
    assert!(json.contains("status_text"));
}
