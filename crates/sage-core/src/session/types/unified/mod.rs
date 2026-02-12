//! Unified Session Data Model
//!
//! This module provides a unified data model for session management,
//! following Claude Code's design patterns:
//! - uuid + parentUuid message chains
//! - Sidechain branching support
//! - Real-time JSONL persistence
//! - SessionRecord for append-only storage

mod header;
mod message;
mod message_types;
mod record;
mod tool_types;
mod wire_token;

// Re-export all public types so external imports remain unchanged
pub use header::{BranchId, MessageId, Session, SessionHeader, SessionId};
pub use message::SessionMessage;
pub use message_types::{MessageContent, SessionMessageType};
pub use record::{SessionMetadataPatch, SessionRecord, SessionRecordPayload};
pub use tool_types::{UnifiedToolCall, UnifiedToolResult};
pub use wire_token::WireTokenUsage;

// Re-export SessionState from base module
pub use super::base::SessionState;

// Re-export canonical context types from enhanced module
pub use super::super::enhanced::context::{
    SessionContext, ThinkingLevel, ThinkingMetadata, TodoItem, TodoStatus,
};

// Re-export file tracking types
pub use super::super::file_tracking::{
    FileBackupInfo, FileHistorySnapshot, TrackedFileState, TrackedFilesSnapshot,
};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;

    #[test]
    fn test_session_header_new() {
        let header = SessionHeader::new("test-id", PathBuf::from("/tmp"));
        assert_eq!(header.id, "test-id");
        assert_eq!(header.state, SessionState::Active);
        assert!(!header.is_sidechain);
    }

    #[test]
    fn test_session_message_user() {
        let ctx = SessionContext::new(PathBuf::from("/tmp"));
        let msg = SessionMessage::user("Hello", "session-1", ctx);
        assert_eq!(msg.message_type, SessionMessageType::User);
        assert_eq!(msg.message.role, crate::types::MessageRole::User);
        assert_eq!(msg.message.content, "Hello");
    }

    #[test]
    fn test_session_message_chain() {
        let ctx = SessionContext::new(PathBuf::from("/tmp"));
        let user_msg = SessionMessage::user("Hello", "session-1", ctx.clone());
        let user_uuid = user_msg.uuid.clone();

        let assistant_msg =
            SessionMessage::assistant("Hi!", "session-1", ctx, Some(user_uuid.clone()));
        assert_eq!(assistant_msg.parent_uuid, Some(user_uuid));
    }

    #[test]
    fn test_token_usage_add() {
        let mut usage1 = WireTokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            ..Default::default()
        };
        let usage2 = WireTokenUsage {
            input_tokens: 200,
            output_tokens: 100,
            ..Default::default()
        };
        usage1.add(&usage2);
        assert_eq!(usage1.input_tokens, 300);
        assert_eq!(usage1.output_tokens, 150);
    }

    #[test]
    fn test_session_record_serialization() {
        let ctx = SessionContext::new(PathBuf::from("/tmp"));
        let msg = SessionMessage::user("Test", "session-1", ctx);
        let record = SessionRecord {
            seq: 1,
            timestamp: Utc::now(),
            session_id: "session-1".to_string(),
            payload: SessionRecordPayload::Message(Box::new(msg)),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("recordType"));
        assert!(json.contains("message"));
    }
}
