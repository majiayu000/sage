use crate::output::OutputEvent;

use super::RuntimeCorrelation;
use super::helpers::{
    error_reported_notification, item_notification, turn_completed_notification,
    with_legacy_session,
};
use super::ids::{legacy_item_id, tool_item_id};
use crate::runtime_protocol::envelope::RuntimeSource;
use crate::runtime_protocol::notification::{
    RuntimeItemPayload, RuntimeItemStatus, RuntimeItemType, RuntimeMessageRole, RuntimeNotification,
};

pub fn notifications_from_output_event(
    event: &OutputEvent,
    correlation: &RuntimeCorrelation,
) -> Vec<RuntimeNotification> {
    match event {
        OutputEvent::System(system) => vec![item_notification(
            "item.created",
            "evt_legacy_system",
            legacy_item_id("system", correlation.sequence),
            system.timestamp,
            RuntimeSource::Runtime,
            correlation,
            RuntimeItemPayload {
                item_type: RuntimeItemType::SystemMessage,
                content: Some(system.message.clone()),
                legacy_type: Some("system".to_string()),
                ..RuntimeItemPayload::new(RuntimeItemType::SystemMessage)
            },
            0,
        )],
        OutputEvent::Assistant(assistant) => vec![item_notification(
            "item.created",
            "evt_legacy_assistant",
            legacy_item_id("assistant", correlation.sequence),
            assistant.timestamp,
            RuntimeSource::Runtime,
            correlation,
            RuntimeItemPayload {
                item_type: RuntimeItemType::AssistantMessage,
                content: Some(assistant.content.clone()),
                legacy_type: Some("assistant".to_string()),
                ..RuntimeItemPayload::new(RuntimeItemType::AssistantMessage)
            },
            0,
        )],
        OutputEvent::ToolCallStart(tool) => vec![item_notification(
            "item.created",
            "evt_legacy_tool_start",
            tool_item_id(&tool.call_id),
            tool.timestamp,
            RuntimeSource::Tool,
            correlation,
            RuntimeItemPayload {
                item_type: RuntimeItemType::ToolCall,
                tool_name: Some(tool.tool_name.clone()),
                status: Some(RuntimeItemStatus::Started),
                arguments: None,
                redacted: Some(true),
                legacy_type: Some("tool_call_start".to_string()),
                ..RuntimeItemPayload::new(RuntimeItemType::ToolCall)
            },
            0,
        )],
        OutputEvent::ToolCallResult(tool) => vec![item_notification(
            "item.completed",
            "evt_legacy_tool_result",
            tool_item_id(&tool.call_id),
            tool.timestamp,
            RuntimeSource::Tool,
            correlation,
            RuntimeItemPayload {
                item_type: RuntimeItemType::ToolCall,
                tool_name: Some(tool.tool_name.clone()),
                status: Some(if tool.success {
                    RuntimeItemStatus::Completed
                } else {
                    RuntimeItemStatus::Failed
                }),
                success: Some(tool.success),
                duration_ms: Some(tool.duration_ms),
                output_preview: None,
                truncated: Some(tool.output.is_some() || tool.error.is_some()),
                redacted: Some(true),
                legacy_type: Some("tool_call_result".to_string()),
                ..RuntimeItemPayload::new(RuntimeItemType::ToolCall)
            },
            0,
        )],
        OutputEvent::UserPrompt(prompt) => vec![item_notification(
            "item.created",
            "evt_legacy_user",
            legacy_item_id("user", correlation.sequence),
            prompt.timestamp,
            RuntimeSource::Runtime,
            correlation,
            RuntimeItemPayload {
                item_type: RuntimeItemType::UserMessage,
                role: Some(RuntimeMessageRole::User),
                content: Some(prompt.content.clone()),
                legacy_type: Some("user_prompt".to_string()),
                ..RuntimeItemPayload::new(RuntimeItemType::UserMessage)
            },
            0,
        )],
        OutputEvent::Error(error) => vec![error_reported_notification(error, correlation)],
        OutputEvent::Result(result) => {
            let item = item_notification(
                "item.created",
                "evt_legacy_result_item",
                legacy_item_id("result", correlation.sequence),
                result.timestamp,
                RuntimeSource::Runtime,
                correlation,
                RuntimeItemPayload {
                    item_type: RuntimeItemType::Result,
                    status: Some(RuntimeItemStatus::Completed),
                    result: Some(result.content.clone()),
                    duration_ms: Some(result.duration_ms),
                    legacy_type: Some("result".to_string()),
                    ..RuntimeItemPayload::new(RuntimeItemType::Result)
                },
                0,
            );
            let turn = turn_completed_notification(result, correlation, 1);
            vec![
                with_legacy_session(item, result.session_id.as_deref()),
                turn,
            ]
        }
    }
}
