use serde_json::Value;

use crate::input::{InputRequestDto, InputRequestKindDto, InputResponseDto, InputResponseKindDto};

use super::RuntimeCorrelation;
use super::helpers::input_item_notification;
use super::ids::id_fragment;
use super::rules::{object_map, rule_from_suggestion};
use crate::runtime_protocol::envelope::{RuntimeEnvelope, RuntimeKind, RuntimeSource};
use crate::runtime_protocol::notification::{RuntimeNotification, RuntimeNotificationPayload};
use crate::runtime_protocol::permission::{
    RuntimePermissionDecision, RuntimePermissionRequestedPayload, RuntimePermissionRisk,
    RuntimeRule,
};
use crate::runtime_protocol::request::{
    RuntimeInputRespondPayload, RuntimePermissionRespondPayload, RuntimeRequest,
    RuntimeRequestPayload,
};

pub fn notification_from_input_request_dto(
    request: &InputRequestDto,
    correlation: &RuntimeCorrelation,
) -> RuntimeNotification {
    match &request.kind {
        InputRequestKindDto::Permission {
            tool_name,
            description,
            input: _,
            suggestions,
        } => RuntimeEnvelope::new(
            RuntimeKind::Notification,
            "permission.requested",
            format!("evt_permission_requested_{}", id_fragment(&request.id)),
            chrono::Utc::now(),
            RuntimeSource::Permission,
            RuntimeNotificationPayload::PermissionRequested(RuntimePermissionRequestedPayload {
                tool_name: tool_name.clone(),
                risk: RuntimePermissionRisk::High,
                reason: description.clone(),
                input_redacted: true,
                input: None,
                suggestions: suggestions.iter().map(rule_from_suggestion).collect(),
            }),
        )
        .with_thread_id(correlation.thread_id.clone())
        .with_turn_id(correlation.turn_id.clone())
        .with_item_id(format!("item_permission_{}", id_fragment(&request.id)))
        .with_request_id(request.id.clone())
        .with_sequence(correlation.sequence)
        .into(),
        InputRequestKindDto::Questions { questions } => input_item_notification(
            &request.id,
            correlation,
            format!("questions:{}", questions.len()),
        ),
        InputRequestKindDto::FreeText { prompt, .. } => {
            input_item_notification(&request.id, correlation, prompt.clone())
        }
        InputRequestKindDto::Simple { question, .. } => {
            input_item_notification(&request.id, correlation, question.clone())
        }
    }
}

pub fn request_from_input_response_dto(
    response: &InputResponseDto,
    correlation: &RuntimeCorrelation,
) -> RuntimeRequest {
    match &response.kind {
        InputResponseKindDto::PermissionGranted {
            modified_input,
            rules,
        } => permission_response_request(
            response,
            correlation,
            RuntimePermissionDecision::Allow,
            None,
            modified_input.clone(),
            rules.iter().map(rule_from_suggestion).collect(),
        ),
        InputResponseKindDto::PermissionDenied { reason } => permission_response_request(
            response,
            correlation,
            RuntimePermissionDecision::Deny,
            reason.clone(),
            None,
            Vec::new(),
        ),
        other => RuntimeRequest::from(
            RuntimeEnvelope::new(
                RuntimeKind::Request,
                "input.respond",
                format!("req_input_respond_{}", id_fragment(&response.request_id)),
                chrono::Utc::now(),
                RuntimeSource::Cli,
                RuntimeRequestPayload::InputRespond(input_response_payload(other)),
            )
            .with_thread_id(correlation.thread_id.clone())
            .with_turn_id(correlation.turn_id.clone())
            .with_request_id(response.request_id.clone()),
        ),
    }
}

fn permission_response_request(
    response: &InputResponseDto,
    correlation: &RuntimeCorrelation,
    decision: RuntimePermissionDecision,
    reason: Option<String>,
    modified_input: Option<Value>,
    rules: Vec<RuntimeRule>,
) -> RuntimeRequest {
    RuntimeRequest::from(
        RuntimeEnvelope::new(
            RuntimeKind::Request,
            "permission.respond",
            format!(
                "req_permission_respond_{}",
                id_fragment(&response.request_id)
            ),
            chrono::Utc::now(),
            RuntimeSource::Cli,
            RuntimeRequestPayload::PermissionRespond(RuntimePermissionRespondPayload {
                decision,
                reason,
                modified_input: modified_input.map(object_map),
                rules,
            }),
        )
        .with_thread_id(correlation.thread_id.clone())
        .with_turn_id(correlation.turn_id.clone())
        .with_request_id(response.request_id.clone()),
    )
}

fn input_response_payload(response: &InputResponseKindDto) -> RuntimeInputRespondPayload {
    match response {
        InputResponseKindDto::QuestionAnswers { answers } => RuntimeInputRespondPayload {
            answers: Some(
                answers
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            ),
            ..RuntimeInputRespondPayload::default()
        },
        InputResponseKindDto::FreeText { text } => RuntimeInputRespondPayload {
            text: Some(text.clone()),
            ..RuntimeInputRespondPayload::default()
        },
        InputResponseKindDto::Cancelled => RuntimeInputRespondPayload {
            cancelled: Some(true),
            ..RuntimeInputRespondPayload::default()
        },
        InputResponseKindDto::Simple { content, .. } => RuntimeInputRespondPayload {
            text: Some(content.clone()),
            ..RuntimeInputRespondPayload::default()
        },
        InputResponseKindDto::PermissionGranted { .. }
        | InputResponseKindDto::PermissionDenied { .. } => RuntimeInputRespondPayload::default(),
    }
}
