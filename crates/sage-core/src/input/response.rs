//! Input response types from user interface

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::permission::PermissionSuggestion;

/// Response kind for different request types
#[derive(Debug, Clone)]
pub enum InputResponseKind {
    /// Answers to structured questions
    QuestionAnswers {
        /// Answers keyed by question text
        answers: HashMap<String, String>,
    },
    /// Permission granted
    PermissionGranted {
        /// Modified input (if user changed it)
        modified_input: Option<serde_json::Value>,
        /// Permission rules to apply
        rules: Vec<PermissionSuggestion>,
    },
    /// Permission denied
    PermissionDenied {
        /// Reason for denial
        reason: Option<String>,
    },
    /// Free text response
    FreeText {
        /// User's text
        text: String,
    },
    /// User cancelled
    Cancelled,
    /// Legacy: simple text/selection response
    Simple {
        /// Text content
        content: String,
        /// Selected indices
        selected_indices: Option<Vec<usize>>,
    },
}

/// User's response to an input request
#[derive(Debug, Clone)]
pub struct InputResponse {
    /// ID matching the request
    pub request_id: Uuid,
    /// The response kind
    pub kind: InputResponseKind,
}

impl InputResponse {
    /// Create a new response
    pub fn new(request_id: Uuid, kind: InputResponseKind) -> Self {
        Self { request_id, kind }
    }

    /// Create a question answers response
    pub fn question_answers(request_id: Uuid, answers: HashMap<String, String>) -> Self {
        Self::new(request_id, InputResponseKind::QuestionAnswers { answers })
    }

    /// Create a permission granted response
    pub fn permission_granted(request_id: Uuid) -> Self {
        Self::new(
            request_id,
            InputResponseKind::PermissionGranted {
                modified_input: None,
                rules: vec![],
            },
        )
    }

    /// Create a permission granted response with modified input
    pub fn permission_granted_with_input(
        request_id: Uuid,
        modified_input: serde_json::Value,
    ) -> Self {
        Self::new(
            request_id,
            InputResponseKind::PermissionGranted {
                modified_input: Some(modified_input),
                rules: vec![],
            },
        )
    }

    /// Create a permission denied response
    pub fn permission_denied(request_id: Uuid, reason: Option<String>) -> Self {
        Self::new(request_id, InputResponseKind::PermissionDenied { reason })
    }

    /// Create a free text response
    pub fn free_text(request_id: Uuid, text: impl Into<String>) -> Self {
        Self::new(
            request_id,
            InputResponseKind::FreeText { text: text.into() },
        )
    }

    /// Create a cancelled response
    pub fn cancelled(request_id: Uuid) -> Self {
        Self::new(request_id, InputResponseKind::Cancelled)
    }

    /// Create a simple text response (legacy compatibility)
    pub fn text(request_id: Uuid, content: impl Into<String>) -> Self {
        Self::new(
            request_id,
            InputResponseKind::Simple {
                content: content.into(),
                selected_indices: None,
            },
        )
    }

    /// Create a response with selected option index (legacy compatibility)
    pub fn selected(request_id: Uuid, index: usize, content: impl Into<String>) -> Self {
        Self::new(
            request_id,
            InputResponseKind::Simple {
                content: content.into(),
                selected_indices: Some(vec![index]),
            },
        )
    }

    /// Create a response with multiple selected indices (legacy compatibility)
    pub fn multi_selected(
        request_id: Uuid,
        indices: Vec<usize>,
        content: impl Into<String>,
    ) -> Self {
        Self::new(
            request_id,
            InputResponseKind::Simple {
                content: content.into(),
                selected_indices: Some(indices),
            },
        )
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        matches!(self.kind, InputResponseKind::Cancelled)
    }

    /// Check if permission was granted
    pub fn is_permission_granted(&self) -> bool {
        matches!(self.kind, InputResponseKind::PermissionGranted { .. })
    }

    /// Check if permission was denied
    pub fn is_permission_denied(&self) -> bool {
        matches!(self.kind, InputResponseKind::PermissionDenied { .. })
    }

    /// Get the text content (for simple/free text responses)
    pub fn get_text(&self) -> Option<&str> {
        match &self.kind {
            InputResponseKind::FreeText { text } => Some(text),
            InputResponseKind::Simple { content, .. } => Some(content),
            _ => None,
        }
    }

    /// Get question answers
    pub fn get_answers(&self) -> Option<&HashMap<String, String>> {
        match &self.kind {
            InputResponseKind::QuestionAnswers { answers } => Some(answers),
            _ => None,
        }
    }
}

/// Legacy response format (for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyInputResponse {
    /// ID matching the request
    pub request_id: Uuid,
    /// User's text response
    pub content: String,
    /// Selected option indices (for multi-select)
    pub selected_indices: Option<Vec<usize>>,
    /// Whether user cancelled/skipped
    pub cancelled: bool,
}

impl LegacyInputResponse {
    /// Convert to new InputResponse format
    pub fn into_response(self) -> InputResponse {
        if self.cancelled {
            InputResponse::cancelled(self.request_id)
        } else {
            InputResponse::new(
                self.request_id,
                InputResponseKind::Simple {
                    content: self.content,
                    selected_indices: self.selected_indices,
                },
            )
        }
    }
}

impl From<LegacyInputResponse> for InputResponse {
    fn from(legacy: LegacyInputResponse) -> Self {
        legacy.into_response()
    }
}
