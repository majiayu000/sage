use serde_json::{Map, Value};

use crate::input::{PermissionSuggestion, RuleDestination};
use crate::tools::permission::PermissionBehavior;

use super::RuntimeCorrelation;
use crate::runtime_protocol::envelope::{RuntimeEnvelope, RuntimeKind, RuntimeSource};
use crate::runtime_protocol::notification::{
    RuntimeErrorReportedPayload, RuntimeItemPayload, RuntimeItemType, RuntimeMessageRole,
    RuntimeNotification, RuntimeNotificationPayload, RuntimeTurnStatus, RuntimeTurnTerminalPayload,
};
use crate::runtime_protocol::permission::{RuntimeRule, RuntimeRuleBehavior, RuntimeRuleSource};

pub(super) fn item_notification(
    message_type: &str,
    id_prefix: &str,
    item_id: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    source: RuntimeSource,
    correlation: &RuntimeCorrelation,
    payload: RuntimeItemPayload,
    sequence_offset: u64,
) -> RuntimeNotification {
    RuntimeEnvelope::new(
        RuntimeKind::Notification,
        message_type,
        format!(
            "{}_{:03}",
            id_prefix,
            correlation.sequence_at(sequence_offset).saturating_add(1)
        ),
        timestamp,
        source,
        RuntimeNotificationPayload::Item(payload),
    )
    .with_thread_id(correlation.thread_id.clone())
    .with_turn_id(correlation.turn_id.clone())
    .with_item_id(item_id)
    .with_sequence(correlation.sequence_at(sequence_offset))
}

pub(super) fn input_item_notification(
    request_id: &str,
    correlation: &RuntimeCorrelation,
    content: String,
) -> RuntimeNotification {
    item_notification(
        "item.created",
        "evt_input_request",
        format!("item_input_{}", id_fragment(request_id)),
        chrono::Utc::now(),
        RuntimeSource::Runtime,
        correlation,
        RuntimeItemPayload {
            item_type: RuntimeItemType::Message,
            role: Some(RuntimeMessageRole::System),
            content: Some(content),
            legacy_type: Some("input_request".to_string()),
            ..RuntimeItemPayload::new(RuntimeItemType::Message)
        },
        0,
    )
    .with_request_id(request_id.to_string())
}

pub(super) fn error_reported_notification(
    error: &crate::output::ErrorEvent,
    correlation: &RuntimeCorrelation,
) -> RuntimeNotification {
    RuntimeEnvelope::new(
        RuntimeKind::Notification,
        "error.reported",
        format!(
            "evt_legacy_error_{:03}",
            correlation.sequence.saturating_add(1)
        ),
        error.timestamp,
        RuntimeSource::Runtime,
        RuntimeNotificationPayload::ErrorReported(RuntimeErrorReportedPayload {
            code: error.code.clone().unwrap_or_else(|| "internal".to_string()),
            message: error.message.clone(),
            details: error.details.clone().map(object_value),
            legacy_type: Some("error".to_string()),
            redacted: Some(true),
        }),
    )
    .with_thread_id(correlation.thread_id.clone())
    .with_turn_id(correlation.turn_id.clone())
    .with_item_id(legacy_item_id("error", correlation.sequence))
    .with_sequence(correlation.sequence)
}

pub(super) fn turn_completed_notification(
    result: &crate::output::ResultEvent,
    correlation: &RuntimeCorrelation,
    sequence_offset: u64,
) -> RuntimeNotification {
    let notification = RuntimeEnvelope::new(
        RuntimeKind::Notification,
        "turn.completed",
        format!(
            "evt_legacy_result_turn_{:03}",
            correlation.sequence_at(sequence_offset).saturating_add(1)
        ),
        result.timestamp,
        RuntimeSource::Runtime,
        RuntimeNotificationPayload::TurnTerminal(RuntimeTurnTerminalPayload {
            status: RuntimeTurnStatus::Completed,
            reason: None,
            result: Some(result.content.clone()),
            duration_ms: Some(result.duration_ms),
            legacy_type: Some("result".to_string()),
        }),
    )
    .with_thread_id(correlation.thread_id.clone())
    .with_turn_id(correlation.turn_id.clone())
    .with_sequence(correlation.sequence_at(sequence_offset));

    with_legacy_session(notification, result.session_id.as_deref())
}

pub(super) fn user_message_notification(
    content: String,
    legacy_type: &str,
    correlation: &RuntimeCorrelation,
) -> RuntimeNotification {
    item_notification(
        "item.created",
        "evt_agent_user_message",
        legacy_item_id("agent_user", correlation.sequence),
        chrono::Utc::now(),
        RuntimeSource::Runtime,
        correlation,
        RuntimeItemPayload {
            item_type: RuntimeItemType::UserMessage,
            role: Some(RuntimeMessageRole::User),
            content: Some(content),
            legacy_type: Some(legacy_type.to_string()),
            ..RuntimeItemPayload::new(RuntimeItemType::UserMessage)
        },
        0,
    )
}

pub(super) fn assistant_message_notification(
    content: String,
    message_type: &str,
    legacy_type: &str,
    correlation: &RuntimeCorrelation,
) -> RuntimeNotification {
    item_notification(
        message_type,
        "evt_agent_assistant_message",
        legacy_item_id("agent_assistant", correlation.sequence),
        chrono::Utc::now(),
        RuntimeSource::Runtime,
        correlation,
        RuntimeItemPayload {
            item_type: RuntimeItemType::AssistantMessage,
            role: Some(RuntimeMessageRole::Assistant),
            content: Some(content),
            legacy_type: Some(legacy_type.to_string()),
            ..RuntimeItemPayload::new(RuntimeItemType::AssistantMessage)
        },
        0,
    )
}

pub(super) fn system_message_notification(
    content: String,
    correlation: &RuntimeCorrelation,
) -> RuntimeNotification {
    item_notification(
        "item.created",
        "evt_agent_system_message",
        legacy_item_id("agent_system", correlation.sequence),
        chrono::Utc::now(),
        RuntimeSource::Runtime,
        correlation,
        RuntimeItemPayload {
            item_type: RuntimeItemType::SystemMessage,
            content: Some(content),
            legacy_type: Some("agent_event".to_string()),
            ..RuntimeItemPayload::new(RuntimeItemType::SystemMessage)
        },
        0,
    )
}

pub(super) fn with_legacy_session(
    notification: RuntimeNotification,
    legacy_session_id: Option<&str>,
) -> RuntimeNotification {
    match legacy_session_id {
        Some(session_id) => {
            notification.with_metadata("legacy_session_id", Value::String(session_id.to_string()))
        }
        None => notification,
    }
}

pub(super) fn object_value(value: Value) -> Value {
    if value.is_object() {
        value
    } else {
        let mut map = Map::new();
        map.insert("value".to_string(), value);
        Value::Object(map)
    }
}

pub(super) fn rule_from_suggestion(suggestion: &PermissionSuggestion) -> RuntimeRule {
    RuntimeRule {
        behavior: match suggestion.behavior {
            PermissionBehavior::Allow => RuntimeRuleBehavior::Allow,
            PermissionBehavior::Deny => RuntimeRuleBehavior::Deny,
            PermissionBehavior::Ask => RuntimeRuleBehavior::Ask,
            PermissionBehavior::Passthrough => RuntimeRuleBehavior::Passthrough,
        },
        source: match suggestion.destination {
            RuleDestination::Session => RuntimeRuleSource::SessionSettings,
            RuleDestination::LocalSettings => RuntimeRuleSource::LocalSettings,
            RuleDestination::UserSettings => RuntimeRuleSource::UserSettings,
            RuleDestination::ProjectSettings => RuntimeRuleSource::ProjectSettings,
        },
    }
}

pub(super) fn legacy_item_id(kind: &str, sequence: u64) -> String {
    format!("item_legacy_{}_{:03}", kind, sequence.saturating_add(1))
}

pub(super) fn tool_item_id(call_id: &str) -> String {
    if call_id.starts_with("item_") {
        call_id.to_string()
    } else {
        format!("item_{}", id_fragment(call_id))
    }
}

pub(super) fn id_fragment(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect()
}
