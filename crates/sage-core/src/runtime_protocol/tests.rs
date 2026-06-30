use std::collections::BTreeMap;
use std::collections::HashMap;

use super::*;
use crate::agent::{AgentExecution, ExecutionError, ExecutionErrorKind, ExecutionOutcome};
use crate::input::{
    InputContext, InputOption, InputRequestDto, InputRequestKindDto, InputResponseDto,
    InputResponseKindDto, Question, QuestionOption,
};
use crate::output::OutputEvent;
use crate::tools::permission::PermissionBehavior;
use crate::types::TaskMetadata;
use crate::ui::AgentEvent;

const STREAM_FIXTURE: &str =
    include_str!("../../../../specs/GH81/fixtures/runtime_protocol_v0_stream.jsonl");
const LEGACY_MAPPING_FIXTURE: &str =
    include_str!("../../../../specs/GH81/fixtures/runtime_protocol_v0_legacy_stream_mapping.jsonl");
const PERMISSION_FIXTURE: &str =
    include_str!("../../../../specs/GH81/fixtures/runtime_protocol_v0_permission_roundtrip.jsonl");
const STRUCTURED_ERROR_FIXTURE: &str =
    include_str!("../../../../specs/GH81/fixtures/runtime_protocol_v0_structured_error.jsonl");
const SCHEMA_FIXTURE: &str =
    include_str!("../../../../specs/GH81/fixtures/runtime_protocol_v0.schema.json");

#[test]
fn runtime_protocol_fixtures_deserialize() {
    for line in STREAM_FIXTURE
        .lines()
        .chain(PERMISSION_FIXTURE.lines())
        .chain(STRUCTURED_ERROR_FIXTURE.lines())
    {
        let message: RuntimeMessage = serde_json::from_str(line).expect("valid runtime message");
        let encoded = serde_json::to_string(&message).expect("runtime message serializes");
        assert!(encoded.contains(RUNTIME_PROTOCOL_VERSION));
    }
}

#[test]
fn schema_fixture_declares_runtime_protocol_v0() {
    let schema: serde_json::Value =
        serde_json::from_str(SCHEMA_FIXTURE).expect("schema fixture is valid JSON");
    assert_eq!(schema["title"], "Sage Runtime Protocol v0");
    assert_eq!(
        schema["$defs"]["protocol_version"]["const"],
        RUNTIME_PROTOCOL_VERSION
    );
}

#[test]
fn runtime_fixture_payload_keys_match_schema() {
    let schema: serde_json::Value =
        serde_json::from_str(SCHEMA_FIXTURE).expect("schema fixture is valid JSON");
    let mut messages = STREAM_FIXTURE
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    messages.extend(LEGACY_MAPPING_FIXTURE.lines().map(|line| {
        serde_json::from_str::<serde_json::Value>(line)
            .expect("mapping row")["protocol_notification"]
            .to_string()
    }));
    messages.extend(PERMISSION_FIXTURE.lines().map(str::to_string));
    messages.extend(STRUCTURED_ERROR_FIXTURE.lines().map(str::to_string));

    for line in messages {
        let message: serde_json::Value = serde_json::from_str(&line).expect("runtime JSON");
        let payload = message["payload"].as_object().expect("payload object");
        let payload_schema = payload_schema_def(&schema, &message);
        let allowed = payload_schema_keys(&schema, &message);

        if let Some(required) = payload_schema["required"].as_array() {
            for field in required {
                let field_name = field.as_str().expect("required field name");
                assert!(
                    payload.contains_key(field_name),
                    "fixture for {} {} misses required payload key {}",
                    message["kind"],
                    message["type"],
                    field_name
                );
            }
        }

        for key in payload.keys() {
            assert!(
                allowed.contains_key(key),
                "schema for {} {} does not declare payload key {}",
                message["kind"],
                message["type"],
                key
            );
            assert_schema_value_type(
                &schema,
                &allowed[key],
                &payload[key],
                &format!("{} {} payload.{key}", message["kind"], message["type"]),
            );
        }
    }
}

#[test]
fn runtime_protocol_rejects_type_payload_mismatches() {
    let wrong_notification = runtime_message_json(
        "notification",
        "permission.requested",
        serde_json::json!({
            "status": "completed",
            "reason": "done"
        }),
    );
    assert!(serde_json::from_value::<RuntimeMessage>(wrong_notification).is_err());

    let wrong_response = runtime_message_json(
        "response",
        "thread.start.result",
        serde_json::json!({
            "accepted": true
        }),
    );
    assert!(serde_json::from_value::<RuntimeMessage>(wrong_response).is_err());

    let wrong_request = runtime_message_json(
        "request",
        "permission.respond",
        serde_json::json!({
            "decision": "allow",
            "modified_input": "raw command"
        }),
    );
    assert!(serde_json::from_value::<RuntimeMessage>(wrong_request).is_err());
}

#[test]
fn legacy_stream_mapping_fixture_matches_output_event_mapper() {
    let mut seen = Vec::new();
    let mut grouped = BTreeMap::<String, (serde_json::Value, Vec<RuntimeNotification>)>::new();

    for line in LEGACY_MAPPING_FIXTURE.lines() {
        let row: serde_json::Value = serde_json::from_str(line).expect("valid mapping row");
        let expected: RuntimeNotification =
            serde_json::from_value(row["protocol_notification"].clone()).expect("notification");
        let key = serde_json::to_string(&row["legacy_output_event"]).expect("legacy key");
        grouped
            .entry(key)
            .or_insert_with(|| (row["legacy_output_event"].clone(), Vec::new()))
            .1
            .push(expected);
    }

    for (_, (legacy_value, expected)) in grouped {
        let legacy: OutputEvent = serde_json::from_value(legacy_value).expect("legacy event");
        let first_sequence = expected
            .iter()
            .filter_map(|message| message.sequence)
            .min()
            .expect("expected sequence");
        let correlation = RuntimeCorrelation::new(
            expected[0].thread_id.clone().expect("thread id"),
            expected[0].turn_id.clone().expect("turn id"),
            first_sequence,
        );

        let actual = notifications_from_output_event(&legacy, &correlation);
        assert_eq!(
            serde_json::to_value(&actual).expect("actual JSON"),
            serde_json::to_value(&expected).expect("expected JSON")
        );
        seen.push(legacy.event_type());
    }

    seen.sort_unstable();
    seen.dedup();
    assert_eq!(
        seen,
        vec![
            "assistant",
            "error",
            "result",
            "system",
            "tool_call_result",
            "tool_call_start",
            "user_prompt"
        ]
    );
}

#[test]
fn agent_event_mapping_covers_current_variants() {
    let correlation = RuntimeCorrelation::new("thread_001", "turn_001", 0);
    let events = vec![
        AgentEvent::session_started("sess_001", "model", "provider"),
        AgentEvent::SessionEnded {
            session_id: "sess_001".to_string(),
        },
        AgentEvent::model_switched("old", "new"),
        AgentEvent::StepStarted { step_number: 1 },
        AgentEvent::ThinkingStarted,
        AgentEvent::ThinkingStopped,
        AgentEvent::ContentStreamStarted,
        AgentEvent::chunk("chunk"),
        AgentEvent::ContentStreamEnded,
        AgentEvent::tool_started("bash", "tool_001", "run command"),
        AgentEvent::tool_completed("bash", "tool_001", true, 12, Some("ok".to_string())),
        AgentEvent::error("tool_failed", "failed"),
        AgentEvent::UserInputRequested {
            prompt: "continue?".to_string(),
        },
        AgentEvent::UserInputReceived {
            input: "yes".to_string(),
        },
        AgentEvent::GitBranchChanged {
            branch: "main".to_string(),
        },
        AgentEvent::WorkingDirectoryChanged {
            path: "/workspace".to_string(),
        },
    ];

    for event in events {
        let notification = notification_from_agent_event(&event, &correlation);
        assert_eq!(notification.kind, RuntimeKind::Notification);
        assert!(notification.message_type.contains('.'));
    }
}

#[test]
fn input_request_and_response_dtos_map_to_protocol_messages() {
    let correlation = RuntimeCorrelation::new("thread_001", "turn_001", 0);
    let permission_request = InputRequestDto {
        id: "req_permission_001".to_string(),
        kind: InputRequestKindDto::Permission {
            tool_name: "bash".to_string(),
            description: "run command".to_string(),
            input: serde_json::json!({"cmd":"cargo test"}),
            suggestions: vec![crate::input::PermissionSuggestion {
                suggestion_type: crate::input::SuggestionType::AddRule,
                tool_name: "bash".to_string(),
                rule_content: "bash:*".to_string(),
                behavior: PermissionBehavior::Ask,
                destination: crate::input::RuleDestination::Session,
            }],
        },
        timeout_ms: None,
    };
    assert_eq!(
        notification_from_input_request_dto(&permission_request, &correlation).message_type,
        "permission.requested"
    );
    let permission_notification =
        notification_from_input_request_dto(&permission_request, &correlation);
    match &permission_notification.payload {
        RuntimeNotificationPayload::PermissionRequested(payload) => {
            assert!(payload.input_redacted);
            assert!(payload.input.is_none());
        }
        other => panic!("unexpected permission payload: {other:?}"),
    }

    let question_request = InputRequestDto {
        id: "req_questions_001".to_string(),
        kind: InputRequestKindDto::Questions {
            questions: vec![Question::new(
                "Pick one",
                "Pick",
                vec![
                    QuestionOption::new("A", "first"),
                    QuestionOption::new("B", "second"),
                ],
            )],
        },
        timeout_ms: None,
    };
    assert_eq!(
        notification_from_input_request_dto(&question_request, &correlation).message_type,
        "item.created"
    );

    let simple_request = InputRequestDto {
        id: "req_simple_001".to_string(),
        kind: InputRequestKindDto::Simple {
            question: "Proceed?".to_string(),
            options: Some(vec![InputOption::new("Yes", "continue")]),
            multi_select: false,
            context: InputContext::Confirmation,
        },
        timeout_ms: None,
    };
    assert_eq!(
        notification_from_input_request_dto(&simple_request, &correlation).message_type,
        "item.created"
    );

    let mut answers = HashMap::new();
    answers.insert("Pick one".to_string(), "A".to_string());
    let responses = vec![
        InputResponseDto {
            request_id: "req_permission_001".to_string(),
            kind: InputResponseKindDto::PermissionDenied {
                reason: Some("read-only".to_string()),
            },
        },
        InputResponseDto {
            request_id: "req_permission_002".to_string(),
            kind: InputResponseKindDto::PermissionGranted {
                modified_input: None,
                rules: Vec::new(),
            },
        },
        InputResponseDto {
            request_id: "req_questions_001".to_string(),
            kind: InputResponseKindDto::QuestionAnswers { answers },
        },
        InputResponseDto {
            request_id: "req_text_001".to_string(),
            kind: InputResponseKindDto::FreeText {
                text: "continue".to_string(),
            },
        },
        InputResponseDto {
            request_id: "req_cancel_001".to_string(),
            kind: InputResponseKindDto::Cancelled,
        },
        InputResponseDto {
            request_id: "req_simple_001".to_string(),
            kind: InputResponseKindDto::Simple {
                content: "yes".to_string(),
                selected_indices: Some(vec![0]),
            },
        },
    ];

    for response in responses {
        let request = request_from_input_response_dto(&response, &correlation);
        assert_eq!(request.kind, RuntimeKind::Request);
        assert!(request.message_type.ends_with(".respond"));
    }
}

#[test]
fn execution_outcome_terminal_variants_have_stable_statuses() {
    let execution = AgentExecution::new(TaskMetadata::new("task", "/workspace"));
    let success = ExecutionOutcome::Success(execution.clone());
    assert_eq!(
        terminal_payload_from_execution_outcome(&success).status,
        RuntimeTurnStatus::Completed
    );

    let failed = ExecutionOutcome::Failed {
        execution: execution.clone(),
        error: ExecutionError::new(
            ExecutionErrorKind::ToolExecution {
                tool_name: "bash".to_string(),
            },
            "tool failed",
        ),
    };
    assert_eq!(
        terminal_payload_from_execution_outcome(&failed)
            .reason
            .as_deref(),
        Some("tool_failed")
    );

    let interrupted = ExecutionOutcome::Interrupted {
        execution: execution.clone(),
    };
    assert_eq!(
        terminal_payload_from_execution_outcome(&interrupted).status,
        RuntimeTurnStatus::Interrupted
    );

    let max_steps = ExecutionOutcome::MaxStepsReached {
        execution: execution.clone(),
    };
    assert_eq!(
        terminal_payload_from_execution_outcome(&max_steps)
            .reason
            .as_deref(),
        Some("max_steps")
    );

    let cancelled = ExecutionOutcome::UserCancelled {
        execution: execution.clone(),
        pending_question: Some("continue?".to_string()),
    };
    assert_eq!(
        terminal_payload_from_execution_outcome(&cancelled)
            .reason
            .as_deref(),
        Some("user_cancelled")
    );

    let needs_input = ExecutionOutcome::NeedsUserInput {
        execution,
        last_response: "Need more context".to_string(),
    };
    assert_eq!(
        terminal_payload_from_execution_outcome(&needs_input)
            .reason
            .as_deref(),
        Some("needs_user_input")
    );
}

#[test]
fn structured_error_fixture_uses_stable_codes_and_redaction_flags() {
    for line in STRUCTURED_ERROR_FIXTURE.lines() {
        let message: RuntimeMessage = serde_json::from_str(line).expect("valid error message");
        match message {
            RuntimeMessage::Error(error) => {
                assert!(!error.payload.code.is_empty());
                assert!(!error.payload.message.is_empty());
                assert!(error.payload.redacted.is_some());
            }
            other => panic!("expected error message, got {other:?}"),
        }
    }
}

fn runtime_message_json(
    kind: &str,
    message_type: &str,
    payload: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "protocol_version": RUNTIME_PROTOCOL_VERSION,
        "kind": kind,
        "type": message_type,
        "id": "evt_test_001",
        "thread_id": "thread_test",
        "turn_id": "turn_test",
        "timestamp": "2026-01-01T00:00:00Z",
        "source": "runtime",
        "payload": payload
    })
}

fn payload_schema_keys<'a>(
    schema: &'a serde_json::Value,
    message: &serde_json::Value,
) -> &'a serde_json::Map<String, serde_json::Value> {
    payload_schema_def(schema, message)["properties"]
        .as_object()
        .expect("payload properties")
}

fn payload_schema_def<'a>(
    schema: &'a serde_json::Value,
    message: &serde_json::Value,
) -> &'a serde_json::Value {
    let defs = &schema["$defs"];
    let kind = message["kind"].as_str().expect("kind");
    let message_type = message["type"].as_str().expect("type");
    let payload_def = match (kind, message_type) {
        ("request", "thread.start") => "thread_start_payload",
        ("request", "thread.resume") => "thread_resume_payload",
        ("request", "thread.fork") => "thread_fork_payload",
        ("request", "turn.start") => "turn_start_payload",
        ("request", "turn.steer") => "turn_steer_payload",
        ("request", "turn.interrupt") => "turn_interrupt_payload",
        ("request", "permission.respond") => "permission_respond_payload",
        ("request", "input.respond") => "input_respond_payload",
        ("notification", "thread.started" | "thread.ended") => "thread_lifecycle_payload",
        ("notification", "turn.started") => "turn_started_payload",
        ("notification", "turn.completed" | "turn.interrupted") => "turn_terminal_payload",
        ("notification", "item.created" | "item.updated" | "item.completed") => "item_payload",
        ("notification", "permission.requested") => "permission_requested_payload",
        ("notification", "permission.resolved") => "permission_resolved_payload",
        ("notification", "error.reported") => "error_reported_payload",
        ("response", "thread.start.result" | "thread.resume.result" | "thread.fork.result") => {
            "thread_response_payload"
        }
        ("response", "turn.start.result" | "turn.steer.result" | "turn.interrupt.result") => {
            "turn_response_payload"
        }
        ("response", "permission.respond.result" | "input.respond.result") => {
            "ack_response_payload"
        }
        ("error", _) => {
            return &defs["error_envelope"]["properties"]["payload"];
        }
        _ => panic!("unmapped runtime message type {kind} {message_type}"),
    };

    &defs[payload_def]
}

fn assert_schema_value_type(
    schema: &serde_json::Value,
    property_schema: &serde_json::Value,
    value: &serde_json::Value,
    context: &str,
) {
    if let Some(ref_name) = property_schema["$ref"].as_str() {
        let def_name = ref_name.trim_start_matches("#/$defs/");
        assert_schema_value_type(schema, &schema["$defs"][def_name], value, context);
        return;
    }

    let Some(expected_type) = property_schema["type"].as_str() else {
        return;
    };
    let matches = match expected_type {
        "array" => value.is_array(),
        "boolean" => value.is_boolean(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "number" => value.is_number(),
        "object" => value.is_object(),
        "string" => value.is_string(),
        _ => true,
    };

    assert!(
        matches,
        "{context} has JSON value {value:?} that does not match schema type {expected_type}"
    );
}
