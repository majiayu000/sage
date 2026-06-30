use serde_json::Value;

use crate::ui::AgentEvent;

use super::RuntimeCorrelation;
use super::helpers::{
    assistant_message_notification, item_notification, system_message_notification,
    user_message_notification,
};
use super::ids::{id_fragment, tool_item_id};
use crate::runtime_protocol::envelope::{RuntimeEnvelope, RuntimeKind, RuntimeSource};
use crate::runtime_protocol::notification::{
    RuntimeErrorReportedPayload, RuntimeItemPayload, RuntimeItemStatus, RuntimeItemType,
    RuntimeNotification, RuntimeNotificationPayload, RuntimeThreadLifecyclePayload,
};

pub fn notification_from_agent_event(
    event: &AgentEvent,
    correlation: &RuntimeCorrelation,
) -> RuntimeNotification {
    match event {
        AgentEvent::SessionStarted {
            session_id,
            model,
            provider,
        } => RuntimeEnvelope::new(
            RuntimeKind::Notification,
            "thread.started",
            format!("evt_agent_session_started_{}", id_fragment(session_id)),
            chrono::Utc::now(),
            RuntimeSource::Runtime,
            RuntimeNotificationPayload::ThreadLifecycle(RuntimeThreadLifecyclePayload {
                persistent: Some(true),
                legacy_session_id: Some(session_id.clone()),
                ..RuntimeThreadLifecyclePayload::default()
            }),
        )
        .with_thread_id(session_id.clone())
        .with_sequence(correlation.sequence)
        .with_metadata("model", Value::String(model.clone()))
        .with_metadata("provider", Value::String(provider.clone()))
        .into(),
        AgentEvent::SessionEnded { session_id } => RuntimeEnvelope::new(
            RuntimeKind::Notification,
            "thread.ended",
            format!("evt_agent_session_ended_{}", id_fragment(session_id)),
            chrono::Utc::now(),
            RuntimeSource::Runtime,
            RuntimeNotificationPayload::ThreadLifecycle(RuntimeThreadLifecyclePayload {
                status: Some("ended".to_string()),
                legacy_session_id: Some(session_id.clone()),
                ..RuntimeThreadLifecyclePayload::default()
            }),
        )
        .with_thread_id(session_id.clone())
        .with_sequence(correlation.sequence)
        .into(),
        AgentEvent::ToolExecutionStarted {
            tool_name,
            tool_id,
            description: _,
        } => item_notification(
            "item.created",
            "evt_agent_tool_started",
            tool_item_id(tool_id),
            chrono::Utc::now(),
            RuntimeSource::Tool,
            correlation,
            RuntimeItemPayload {
                item_type: RuntimeItemType::ToolCall,
                tool_name: Some(tool_name.clone()),
                status: Some(RuntimeItemStatus::Started),
                redacted: Some(true),
                legacy_type: Some("agent_tool_execution_started".to_string()),
                ..RuntimeItemPayload::new(RuntimeItemType::ToolCall)
            },
            0,
        ),
        AgentEvent::ToolExecutionCompleted {
            tool_name,
            tool_id,
            success,
            duration_ms,
            result_preview,
        } => item_notification(
            "item.completed",
            "evt_agent_tool_completed",
            tool_item_id(tool_id),
            chrono::Utc::now(),
            RuntimeSource::Tool,
            correlation,
            RuntimeItemPayload {
                item_type: RuntimeItemType::ToolCall,
                tool_name: Some(tool_name.clone()),
                status: Some(if *success {
                    RuntimeItemStatus::Completed
                } else {
                    RuntimeItemStatus::Failed
                }),
                success: Some(*success),
                duration_ms: Some(*duration_ms),
                output_preview: None,
                truncated: Some(result_preview.is_some()),
                redacted: Some(true),
                legacy_type: Some("agent_tool_execution_completed".to_string()),
                ..RuntimeItemPayload::new(RuntimeItemType::ToolCall)
            },
            0,
        ),
        AgentEvent::ErrorOccurred {
            error_type,
            message,
        } => RuntimeEnvelope::new(
            RuntimeKind::Notification,
            "error.reported",
            format!(
                "evt_agent_error_{:03}",
                correlation.sequence.saturating_add(1)
            ),
            chrono::Utc::now(),
            RuntimeSource::Runtime,
            RuntimeNotificationPayload::ErrorReported(RuntimeErrorReportedPayload {
                code: error_type.clone(),
                message: message.clone(),
                details: None,
                legacy_type: Some("agent_error_occurred".to_string()),
                redacted: Some(true),
            }),
        )
        .with_thread_id(correlation.thread_id.clone())
        .with_turn_id(correlation.turn_id.clone())
        .with_sequence(correlation.sequence)
        .into(),
        AgentEvent::UserInputReceived { input } => {
            user_message_notification(input.clone(), "agent_user_input_received", correlation)
        }
        AgentEvent::ContentChunk { chunk } => assistant_message_notification(
            chunk.clone(),
            "item.updated",
            "agent_content_chunk",
            correlation,
        ),
        other => system_message_notification(agent_event_summary(other), correlation),
    }
}

fn agent_event_summary(event: &AgentEvent) -> String {
    match event {
        AgentEvent::ModelSwitched {
            old_model,
            new_model,
        } => {
            format!("model switched from {old_model} to {new_model}")
        }
        AgentEvent::StepStarted { step_number } => format!("step {step_number} started"),
        AgentEvent::ThinkingStarted => "thinking started".to_string(),
        AgentEvent::ThinkingStopped => "thinking stopped".to_string(),
        AgentEvent::ContentStreamStarted => "content stream started".to_string(),
        AgentEvent::ContentStreamEnded => "content stream ended".to_string(),
        AgentEvent::UserInputRequested { prompt } => format!("user input requested: {prompt}"),
        AgentEvent::GitBranchChanged { branch } => format!("git branch changed: {branch}"),
        AgentEvent::WorkingDirectoryChanged { path } => {
            format!("working directory changed: {path}")
        }
        AgentEvent::SessionStarted { .. }
        | AgentEvent::SessionEnded { .. }
        | AgentEvent::ContentChunk { .. }
        | AgentEvent::ToolExecutionStarted { .. }
        | AgentEvent::ToolExecutionCompleted { .. }
        | AgentEvent::ErrorOccurred { .. }
        | AgentEvent::UserInputReceived { .. } => "agent event".to_string(),
    }
}
