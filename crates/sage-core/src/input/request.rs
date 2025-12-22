//! Input request types for user interaction

use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use super::permission::PermissionSuggestion;
use super::types::{InputContext, InputOption, Question};

/// Input request types (unified enum for all request kinds)
#[derive(Debug, Clone)]
pub enum InputRequestKind {
    /// Structured questions (from AskUserQuestion tool)
    Questions {
        /// List of questions to ask
        questions: Vec<Question>,
    },
    /// Permission request for a tool
    Permission {
        /// Tool name
        tool_name: String,
        /// Description of what the tool wants to do
        description: String,
        /// Tool input parameters
        input: serde_json::Value,
        /// Suggested permission rules
        suggestions: Vec<PermissionSuggestion>,
    },
    /// Free text input (when model outputs text without tools)
    FreeText {
        /// Prompt to show
        prompt: String,
        /// Last response from the model
        last_response: String,
    },
    /// Legacy: Simple question with options
    Simple {
        /// Question text
        question: String,
        /// Options (optional)
        options: Option<Vec<InputOption>>,
        /// Multi-select
        multi_select: bool,
        /// Context
        context: InputContext,
    },
}

/// Input request sent from execution loop to user interface
#[derive(Debug, Clone)]
pub struct InputRequest {
    /// Unique ID for this request
    pub id: Uuid,
    /// The kind of input request
    pub kind: InputRequestKind,
    /// Optional timeout for auto-response (None = wait indefinitely)
    pub timeout: Option<Duration>,
}

impl InputRequest {
    /// Create a new input request with a kind
    pub fn new(kind: InputRequestKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            timeout: None,
        }
    }

    /// Create a questions request (for AskUserQuestion tool)
    pub fn questions(questions: Vec<Question>) -> Self {
        Self::new(InputRequestKind::Questions { questions })
    }

    /// Create a permission request
    pub fn permission(
        tool_name: impl Into<String>,
        description: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        Self::new(InputRequestKind::Permission {
            tool_name: tool_name.into(),
            description: description.into(),
            input,
            suggestions: vec![],
        })
    }

    /// Create a free text request (when model needs user input)
    pub fn free_text(prompt: impl Into<String>, last_response: impl Into<String>) -> Self {
        Self::new(InputRequestKind::FreeText {
            prompt: prompt.into(),
            last_response: last_response.into(),
        })
    }

    /// Create a simple question request (legacy compatibility)
    pub fn simple(question: impl Into<String>) -> Self {
        Self::new(InputRequestKind::Simple {
            question: question.into(),
            options: None,
            multi_select: false,
            context: InputContext::Clarification,
        })
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Add permission suggestions
    pub fn with_suggestions(mut self, suggestions: Vec<PermissionSuggestion>) -> Self {
        if let InputRequestKind::Permission {
            suggestions: ref mut s,
            ..
        } = self.kind
        {
            *s = suggestions;
        }
        self
    }

    /// Check if this is a questions request
    pub fn is_questions(&self) -> bool {
        matches!(self.kind, InputRequestKind::Questions { .. })
    }

    /// Check if this is a permission request
    pub fn is_permission(&self) -> bool {
        matches!(self.kind, InputRequestKind::Permission { .. })
    }

    /// Check if this is a free text request
    pub fn is_free_text(&self) -> bool {
        matches!(self.kind, InputRequestKind::FreeText { .. })
    }
}

/// Legacy input request (for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyInputRequest {
    /// Unique ID for this request
    pub id: Uuid,
    /// The question to display to the user (markdown formatted)
    pub question: String,
    /// Optional structured options for selection
    pub options: Option<Vec<InputOption>>,
    /// Whether multiple selections are allowed
    pub multi_select: bool,
    /// Optional timeout for auto-response (None = wait indefinitely)
    #[serde(skip)]
    pub timeout: Option<Duration>,
    /// Context about why input is needed
    pub context: InputContext,
}

impl LegacyInputRequest {
    /// Create a new legacy input request
    pub fn new(question: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            question: question.into(),
            options: None,
            multi_select: false,
            timeout: None,
            context: InputContext::Clarification,
        }
    }

    /// Add options for selection
    pub fn with_options(mut self, options: Vec<InputOption>) -> Self {
        self.options = Some(options);
        self
    }

    /// Enable multi-select
    pub fn with_multi_select(mut self, multi_select: bool) -> Self {
        self.multi_select = multi_select;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set context
    pub fn with_context(mut self, context: InputContext) -> Self {
        self.context = context;
        self
    }

    /// Convert to new InputRequest format
    pub fn into_request(self) -> InputRequest {
        InputRequest {
            id: self.id,
            kind: InputRequestKind::Simple {
                question: self.question,
                options: self.options,
                multi_select: self.multi_select,
                context: self.context,
            },
            timeout: self.timeout,
        }
    }
}
