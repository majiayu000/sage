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
fn legacy_stream_mapping_fixture_covers_every_output_event() {
    let mut seen = Vec::new();

    for line in LEGACY_MAPPING_FIXTURE.lines() {
        let row: serde_json::Value = serde_json::from_str(line).expect("valid mapping row");
        let legacy: OutputEvent =
            serde_json::from_value(row["legacy_output_event"].clone()).expect("legacy event");
        let expected: RuntimeNotification =
            serde_json::from_value(row["protocol_notification"].clone()).expect("notification");
        let correlation = RuntimeCorrelation::new(
            expected.thread_id.clone().expect("thread id"),
            expected.turn_id.clone().expect("turn id"),
            expected.sequence.expect("sequence"),
        );

        let actual = notifications_from_output_event(&legacy, &correlation);
        assert!(
            actual
                .iter()
                .any(|message| message.message_type == expected.message_type),
            "missing mapped message type {}",
            expected.message_type
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
