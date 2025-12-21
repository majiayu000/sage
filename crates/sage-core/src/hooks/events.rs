//! Hook event types
//!
//! Defines all the events that can trigger hook execution.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Hook events that can trigger hook execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEvent {
    /// Before tool execution
    PreToolUse,
    /// After successful tool execution
    PostToolUse,
    /// After failed tool execution
    PostToolUseFailure,
    /// User submits a prompt
    UserPromptSubmit,
    /// Session starts
    SessionStart,
    /// Session ends
    SessionEnd,
    /// Sub-agent starts
    SubagentStart,
    /// Sub-agent stops
    SubagentStop,
    /// Permission request
    PermissionRequest,
    /// Before context compaction
    PreCompact,
    /// Notification event
    Notification,
    /// Agent is stopping (main agent)
    Stop,
    /// Status line update
    StatusLine,
}

impl HookEvent {
    /// Get the match query field for this event type
    pub fn match_field(&self) -> &'static str {
        match self {
            HookEvent::PreToolUse | HookEvent::PostToolUse | HookEvent::PostToolUseFailure => {
                "tool_name"
            }
            HookEvent::SessionStart => "source",
            HookEvent::SessionEnd => "reason",
            HookEvent::SubagentStart | HookEvent::SubagentStop => "agent_type",
            HookEvent::UserPromptSubmit => "prompt",
            HookEvent::PermissionRequest => "tool_name",
            HookEvent::PreCompact => "trigger",
            HookEvent::Notification => "notification_type",
            HookEvent::Stop => "stop_reason",
            HookEvent::StatusLine => "status",
        }
    }

    /// Get a human-readable description of this event
    pub fn description(&self) -> &'static str {
        match self {
            HookEvent::PreToolUse => "Before tool execution",
            HookEvent::PostToolUse => "After successful tool execution",
            HookEvent::PostToolUseFailure => "After failed tool execution",
            HookEvent::UserPromptSubmit => "User submits a prompt",
            HookEvent::SessionStart => "Session starts",
            HookEvent::SessionEnd => "Session ends",
            HookEvent::SubagentStart => "Sub-agent starts",
            HookEvent::SubagentStop => "Sub-agent stops",
            HookEvent::PermissionRequest => "Permission request",
            HookEvent::PreCompact => "Before context compaction",
            HookEvent::Notification => "Notification event",
            HookEvent::Stop => "Agent is stopping",
            HookEvent::StatusLine => "Status line update",
        }
    }

    /// Returns all possible hook events
    pub fn all() -> &'static [HookEvent] {
        &[
            HookEvent::PreToolUse,
            HookEvent::PostToolUse,
            HookEvent::PostToolUseFailure,
            HookEvent::UserPromptSubmit,
            HookEvent::SessionStart,
            HookEvent::SessionEnd,
            HookEvent::SubagentStart,
            HookEvent::SubagentStop,
            HookEvent::PermissionRequest,
            HookEvent::PreCompact,
            HookEvent::Notification,
            HookEvent::Stop,
            HookEvent::StatusLine,
        ]
    }
}

impl fmt::Display for HookEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HookEvent::PreToolUse => write!(f, "PreToolUse"),
            HookEvent::PostToolUse => write!(f, "PostToolUse"),
            HookEvent::PostToolUseFailure => write!(f, "PostToolUseFailure"),
            HookEvent::UserPromptSubmit => write!(f, "UserPromptSubmit"),
            HookEvent::SessionStart => write!(f, "SessionStart"),
            HookEvent::SessionEnd => write!(f, "SessionEnd"),
            HookEvent::SubagentStart => write!(f, "SubagentStart"),
            HookEvent::SubagentStop => write!(f, "SubagentStop"),
            HookEvent::PermissionRequest => write!(f, "PermissionRequest"),
            HookEvent::PreCompact => write!(f, "PreCompact"),
            HookEvent::Notification => write!(f, "Notification"),
            HookEvent::Stop => write!(f, "Stop"),
            HookEvent::StatusLine => write!(f, "StatusLine"),
        }
    }
}

impl Default for HookEvent {
    fn default() -> Self {
        HookEvent::SessionStart
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_match_field() {
        assert_eq!(HookEvent::PreToolUse.match_field(), "tool_name");
        assert_eq!(HookEvent::PostToolUse.match_field(), "tool_name");
        assert_eq!(HookEvent::PostToolUseFailure.match_field(), "tool_name");
        assert_eq!(HookEvent::UserPromptSubmit.match_field(), "prompt");
        assert_eq!(HookEvent::SessionStart.match_field(), "source");
        assert_eq!(HookEvent::SessionEnd.match_field(), "reason");
        assert_eq!(HookEvent::SubagentStart.match_field(), "agent_type");
        assert_eq!(HookEvent::SubagentStop.match_field(), "agent_type");
        assert_eq!(HookEvent::PermissionRequest.match_field(), "tool_name");
        assert_eq!(HookEvent::PreCompact.match_field(), "trigger");
        assert_eq!(HookEvent::Notification.match_field(), "notification_type");
        assert_eq!(HookEvent::Stop.match_field(), "stop_reason");
        assert_eq!(HookEvent::StatusLine.match_field(), "status");
    }

    #[test]
    fn test_hook_event_description() {
        assert_eq!(HookEvent::PreToolUse.description(), "Before tool execution");
        assert_eq!(
            HookEvent::PostToolUse.description(),
            "After successful tool execution"
        );
        assert_eq!(
            HookEvent::PostToolUseFailure.description(),
            "After failed tool execution"
        );
        assert_eq!(
            HookEvent::UserPromptSubmit.description(),
            "User submits a prompt"
        );
        assert_eq!(HookEvent::SessionStart.description(), "Session starts");
        assert_eq!(HookEvent::SessionEnd.description(), "Session ends");
        assert_eq!(HookEvent::SubagentStart.description(), "Sub-agent starts");
        assert_eq!(HookEvent::SubagentStop.description(), "Sub-agent stops");
        assert_eq!(
            HookEvent::PermissionRequest.description(),
            "Permission request"
        );
        assert_eq!(
            HookEvent::PreCompact.description(),
            "Before context compaction"
        );
        assert_eq!(HookEvent::Notification.description(), "Notification event");
        assert_eq!(HookEvent::Stop.description(), "Agent is stopping");
        assert_eq!(HookEvent::StatusLine.description(), "Status line update");
    }

    #[test]
    fn test_hook_event_all() {
        let all_events = HookEvent::all();
        assert_eq!(all_events.len(), 13);
        assert!(all_events.contains(&HookEvent::PreToolUse));
        assert!(all_events.contains(&HookEvent::PostToolUse));
        assert!(all_events.contains(&HookEvent::PostToolUseFailure));
        assert!(all_events.contains(&HookEvent::UserPromptSubmit));
        assert!(all_events.contains(&HookEvent::SessionStart));
        assert!(all_events.contains(&HookEvent::SessionEnd));
        assert!(all_events.contains(&HookEvent::SubagentStart));
        assert!(all_events.contains(&HookEvent::SubagentStop));
        assert!(all_events.contains(&HookEvent::PermissionRequest));
        assert!(all_events.contains(&HookEvent::PreCompact));
        assert!(all_events.contains(&HookEvent::Notification));
        assert!(all_events.contains(&HookEvent::Stop));
        assert!(all_events.contains(&HookEvent::StatusLine));
    }

    #[test]
    fn test_hook_event_display() {
        assert_eq!(format!("{}", HookEvent::PreToolUse), "PreToolUse");
        assert_eq!(format!("{}", HookEvent::PostToolUse), "PostToolUse");
        assert_eq!(
            format!("{}", HookEvent::PostToolUseFailure),
            "PostToolUseFailure"
        );
        assert_eq!(
            format!("{}", HookEvent::UserPromptSubmit),
            "UserPromptSubmit"
        );
        assert_eq!(format!("{}", HookEvent::SessionStart), "SessionStart");
        assert_eq!(format!("{}", HookEvent::SessionEnd), "SessionEnd");
        assert_eq!(format!("{}", HookEvent::SubagentStart), "SubagentStart");
        assert_eq!(format!("{}", HookEvent::SubagentStop), "SubagentStop");
        assert_eq!(
            format!("{}", HookEvent::PermissionRequest),
            "PermissionRequest"
        );
        assert_eq!(format!("{}", HookEvent::PreCompact), "PreCompact");
        assert_eq!(format!("{}", HookEvent::Notification), "Notification");
        assert_eq!(format!("{}", HookEvent::Stop), "Stop");
        assert_eq!(format!("{}", HookEvent::StatusLine), "StatusLine");
    }

    #[test]
    fn test_hook_event_default() {
        assert_eq!(HookEvent::default(), HookEvent::SessionStart);
    }

    #[test]
    fn test_hook_event_equality() {
        assert_eq!(HookEvent::PreToolUse, HookEvent::PreToolUse);
        assert_ne!(HookEvent::PreToolUse, HookEvent::PostToolUse);
    }

    #[test]
    fn test_hook_event_serialization() {
        let event = HookEvent::PreToolUse;
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: HookEvent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_hook_event_clone() {
        let event = HookEvent::SessionStart;
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn test_hook_event_debug() {
        let event = HookEvent::PreToolUse;
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("PreToolUse"));
    }
}
