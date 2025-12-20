//! Input channel system for unified execution loop
//!
//! This module provides an async communication channel between the execution loop
//! and the user interface, enabling blocking user input within the agent loop.
//!
//! # Design
//!
//! Following Claude Code's architecture, the execution loop can await on user input
//! instead of exiting and resuming. This is achieved through a bidirectional channel:
//!
//! - Execution loop sends `InputRequest` when user input is needed
//! - UI layer receives the request, displays it, and sends back `InputResponse`
//! - Execution loop receives response and continues (no exit/resume cycle)
//!
//! # Request Types
//!
//! - `InputRequest::Question` - Structured questions (from AskUserQuestion tool)
//! - `InputRequest::Permission` - Tool permission requests
//! - `InputRequest::FreeText` - Free text input (when model needs user response)
//!
//! # Example
//!
//! ```ignore
//! // Create channel pair
//! let (channel, handle) = InputChannel::new(16);
//!
//! // Spawn UI handler
//! tokio::spawn(async move {
//!     while let Some(request) = handle.request_rx.recv().await {
//!         // Display question to user, get input
//!         let response = get_user_input(&request);
//!         handle.response_tx.send(response).await.ok();
//!     }
//! });
//!
//! // In execution loop
//! let response = channel.request_input(request).await?;
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::error::{SageError, SageResult};

// ============================================================================
// Question Types (for AskUserQuestion tool)
// ============================================================================

/// A structured question with options (Claude Code style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    /// The question text to ask
    pub question: String,
    /// Short label/header (max 12 chars) like "Auth method", "Library"
    pub header: String,
    /// List of options to choose from (2-4 options)
    pub options: Vec<QuestionOption>,
    /// Whether multiple options can be selected
    #[serde(default)]
    pub multi_select: bool,
}

impl Question {
    /// Create a new question
    pub fn new(
        question: impl Into<String>,
        header: impl Into<String>,
        options: Vec<QuestionOption>,
    ) -> Self {
        Self {
            question: question.into(),
            header: header.into(),
            options,
            multi_select: false,
        }
    }

    /// Enable multi-select
    pub fn with_multi_select(mut self) -> Self {
        self.multi_select = true;
        self
    }

    /// Validate the question structure
    pub fn validate(&self) -> Result<(), String> {
        if self.header.len() > 12 {
            return Err(format!(
                "Header '{}' exceeds 12 characters (length: {})",
                self.header,
                self.header.len()
            ));
        }
        if self.question.trim().is_empty() {
            return Err("Question text cannot be empty".to_string());
        }
        if self.options.len() < 2 {
            return Err("Question must have at least 2 options".to_string());
        }
        if self.options.len() > 4 {
            return Err("Question cannot have more than 4 options".to_string());
        }
        for (i, opt) in self.options.iter().enumerate() {
            if opt.label.trim().is_empty() {
                return Err(format!("Option {} has empty label", i + 1));
            }
        }
        Ok(())
    }
}

/// Option for a structured question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    /// Display label (1-5 words)
    pub label: String,
    /// Explanation of what this option means
    pub description: String,
}

impl QuestionOption {
    /// Create a new option
    pub fn new(label: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: description.into(),
        }
    }
}

// ============================================================================
// Permission Types
// ============================================================================

/// Permission behavior types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionBehavior {
    /// Automatically allow
    Allow,
    /// Automatically deny
    Deny,
    /// Ask the user
    Ask,
    /// Pass through without checking
    Passthrough,
}

/// Permission check result
#[derive(Debug, Clone)]
pub enum PermissionResult {
    /// Tool execution allowed
    Allow,
    /// Tool execution denied
    Deny { message: String },
    /// Need to ask user
    Ask {
        message: String,
        suggestions: Vec<PermissionSuggestion>,
    },
}

impl PermissionResult {
    /// Check if permission is granted
    pub fn is_allowed(&self) -> bool {
        matches!(self, PermissionResult::Allow)
    }

    /// Check if permission is denied
    pub fn is_denied(&self) -> bool {
        matches!(self, PermissionResult::Deny { .. })
    }

    /// Check if user input is needed
    pub fn needs_user_input(&self) -> bool {
        matches!(self, PermissionResult::Ask { .. })
    }
}

/// Permission suggestion for the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSuggestion {
    /// Type of suggestion
    pub suggestion_type: SuggestionType,
    /// Tool name this applies to
    pub tool_name: String,
    /// Rule pattern/content
    pub rule_content: String,
    /// Behavior to apply
    pub behavior: PermissionBehavior,
    /// Where to save this rule
    pub destination: RuleDestination,
}

/// Types of permission suggestions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestionType {
    AddRule,
    RemoveRule,
    ModifyRule,
}

/// Where to save permission rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleDestination {
    /// Only for this session
    Session,
    /// Local project settings
    LocalSettings,
    /// User-level settings
    UserSettings,
    /// Project-specific settings
    ProjectSettings,
}

// ============================================================================
// Input Request Types
// ============================================================================

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

/// A single option for selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputOption {
    /// Display label
    pub label: String,
    /// Description of what this option means
    pub description: String,
    /// Value to return if selected (defaults to label if not set)
    pub value: Option<String>,
}

impl InputOption {
    /// Create a new option
    pub fn new(label: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: description.into(),
            value: None,
        }
    }

    /// Set a custom value
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Get the value (returns label if no custom value set)
    pub fn get_value(&self) -> &str {
        self.value.as_deref().unwrap_or(&self.label)
    }
}

/// Context about why input is needed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputContext {
    /// Agent needs clarification on task
    Clarification,
    /// Agent needs user decision
    Decision,
    /// Agent needs confirmation to proceed
    Confirmation,
    /// Agent wants to provide information and get feedback
    Feedback,
    /// Agent is asking about preferences
    Preference,
}

// ============================================================================
// Input Response Types
// ============================================================================

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
            InputResponseKind::FreeText {
                text: text.into(),
            },
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

// ============================================================================
// Auto Response
// ============================================================================

/// Auto-responder function type
pub type AutoResponder = Box<dyn Fn(&InputRequest) -> InputResponse + Send + Sync>;

/// Auto-response strategies for non-interactive mode
#[derive(Clone)]
pub enum AutoResponse {
    /// Use default responses (empty answers, deny permissions)
    Default,
    /// Always allow permissions, use first option for questions
    AlwaysAllow,
    /// Always deny/cancel
    AlwaysDeny,
    /// Custom responder function
    Custom(Arc<dyn Fn(&InputRequest) -> InputResponse + Send + Sync>),
}

impl std::fmt::Debug for AutoResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AutoResponse::Default => write!(f, "AutoResponse::Default"),
            AutoResponse::AlwaysAllow => write!(f, "AutoResponse::AlwaysAllow"),
            AutoResponse::AlwaysDeny => write!(f, "AutoResponse::AlwaysDeny"),
            AutoResponse::Custom(_) => write!(f, "AutoResponse::Custom(...)"),
        }
    }
}

impl AutoResponse {
    /// Convert to a responder function
    pub fn into_responder(self) -> AutoResponder {
        match self {
            AutoResponse::Default => Box::new(|req: &InputRequest| match &req.kind {
                InputRequestKind::Questions { .. } => {
                    InputResponse::question_answers(req.id, HashMap::new())
                }
                InputRequestKind::Permission { .. } => {
                    InputResponse::permission_denied(req.id, Some("Non-interactive mode".to_string()))
                }
                InputRequestKind::FreeText { .. } => InputResponse::free_text(req.id, ""),
                InputRequestKind::Simple { options, .. } => {
                    if options.is_some() {
                        InputResponse::selected(req.id, 0, "")
                    } else {
                        InputResponse::text(req.id, "")
                    }
                }
            }),
            AutoResponse::AlwaysAllow => Box::new(|req: &InputRequest| match &req.kind {
                InputRequestKind::Questions { questions } => {
                    // Select first option for each question
                    let answers: HashMap<String, String> = questions
                        .iter()
                        .map(|q| {
                            let answer = q.options.first().map(|o| o.label.clone()).unwrap_or_default();
                            (q.question.clone(), answer)
                        })
                        .collect();
                    InputResponse::question_answers(req.id, answers)
                }
                InputRequestKind::Permission { input, .. } => {
                    InputResponse::permission_granted_with_input(req.id, input.clone())
                }
                InputRequestKind::FreeText { .. } => InputResponse::free_text(req.id, "continue"),
                InputRequestKind::Simple { options, .. } => {
                    if options.is_some() {
                        InputResponse::selected(req.id, 0, "auto-selected")
                    } else {
                        InputResponse::text(req.id, "continue")
                    }
                }
            }),
            AutoResponse::AlwaysDeny => {
                Box::new(|req: &InputRequest| InputResponse::cancelled(req.id))
            }
            AutoResponse::Custom(f) => Box::new(move |req: &InputRequest| f(req)),
        }
    }
}

// ============================================================================
// Input Channel
// ============================================================================

/// Input channel for communication between execution loop and UI
///
/// This is the execution loop's side of the channel. It sends requests
/// and receives responses.
pub struct InputChannel {
    /// Sender for input requests (loop -> UI)
    request_tx: mpsc::Sender<InputRequest>,
    /// Receiver for user responses (UI -> loop)
    response_rx: mpsc::Receiver<InputResponse>,
    /// Default timeout for responses
    default_timeout: Option<Duration>,
    /// Auto-responder for non-interactive mode
    auto_responder: Option<AutoResponder>,
}

/// Handle for the UI side to receive requests and send responses
pub struct InputChannelHandle {
    /// Receiver for input requests (loop -> UI)
    pub request_rx: mpsc::Receiver<InputRequest>,
    /// Sender for user responses (UI -> loop)
    pub response_tx: mpsc::Sender<InputResponse>,
}

impl InputChannel {
    /// Create a new input channel pair
    ///
    /// Returns the channel (for execution loop) and handle (for UI).
    /// `buffer_size` controls how many pending requests can be queued.
    pub fn new(buffer_size: usize) -> (Self, InputChannelHandle) {
        let (request_tx, request_rx) = mpsc::channel(buffer_size);
        let (response_tx, response_rx) = mpsc::channel(buffer_size);

        let channel = Self {
            request_tx,
            response_rx,
            default_timeout: None,
            auto_responder: None,
        };

        let handle = InputChannelHandle {
            request_rx,
            response_tx,
        };

        (channel, handle)
    }

    /// Create a non-interactive channel that auto-responds
    ///
    /// This is used for batch/CI mode where no user input is available.
    pub fn non_interactive(auto_response: AutoResponse) -> Self {
        // Create dummy channels (won't be used)
        let (request_tx, _) = mpsc::channel(1);
        let (_, response_rx) = mpsc::channel(1);

        Self {
            request_tx,
            response_rx,
            default_timeout: Some(Duration::from_millis(10)),
            auto_responder: Some(auto_response.into_responder()),
        }
    }

    /// Create a non-interactive channel with a simple text response (legacy)
    pub fn non_interactive_text(default_response: impl Into<String>) -> Self {
        let response = default_response.into();

        // Create dummy channels (won't be used)
        let (request_tx, _) = mpsc::channel(1);
        let (_, response_rx) = mpsc::channel(1);

        Self {
            request_tx,
            response_rx,
            default_timeout: Some(Duration::from_millis(10)),
            auto_responder: Some(Box::new(move |req: &InputRequest| {
                match &req.kind {
                    InputRequestKind::Simple { options, .. } if options.is_some() => {
                        InputResponse::selected(req.id, 0, response.clone())
                    }
                    _ => InputResponse::text(req.id, response.clone()),
                }
            })),
        }
    }

    /// Create a channel that fails on any input request
    ///
    /// This is used for strict batch mode where user prompts should fail.
    pub fn fail_on_input() -> Self {
        let (request_tx, _) = mpsc::channel(1);
        let (_, response_rx) = mpsc::channel(1);

        Self {
            request_tx,
            response_rx,
            default_timeout: Some(Duration::from_millis(10)),
            auto_responder: Some(Box::new(|req| InputResponse::cancelled(req.id))),
        }
    }

    /// Set default timeout for responses
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = Some(timeout);
        self
    }

    /// Set auto-responder function
    pub fn with_auto_responder<F>(mut self, responder: F) -> Self
    where
        F: Fn(&InputRequest) -> InputResponse + Send + Sync + 'static,
    {
        self.auto_responder = Some(Box::new(responder));
        self
    }

    /// Check if this channel uses auto-response (non-interactive)
    pub fn is_non_interactive(&self) -> bool {
        self.auto_responder.is_some()
    }

    /// Request input from user, blocking until response received
    ///
    /// This is the key method that enables blocking user input in the execution loop.
    /// It will:
    /// 1. If auto-responder is set, return immediately with auto-response
    /// 2. Otherwise, send request to UI and wait for response
    /// 3. Respect timeout if set (request timeout > channel default > wait forever)
    pub async fn request_input(&mut self, request: InputRequest) -> SageResult<InputResponse> {
        // If we have an auto-responder (non-interactive mode), use it immediately
        if let Some(ref responder) = self.auto_responder {
            return Ok(responder(&request));
        }

        // Send request to UI
        self.request_tx
            .send(request.clone())
            .await
            .map_err(|_| SageError::agent("Input channel closed - UI handler not running"))?;

        // Determine timeout: request-specific > channel default > no timeout
        let timeout = request.timeout.or(self.default_timeout);

        // Wait for response
        if let Some(timeout_duration) = timeout {
            match tokio::time::timeout(timeout_duration, self.response_rx.recv()).await {
                Ok(Some(response)) => {
                    // Verify response matches request
                    if response.request_id != request.id {
                        return Err(SageError::agent("Response ID mismatch"));
                    }
                    Ok(response)
                }
                Ok(None) => Err(SageError::agent("Input channel closed")),
                Err(_) => Err(SageError::Timeout {
                    seconds: timeout_duration.as_secs(),
                }),
            }
        } else {
            // Block indefinitely until response
            match self.response_rx.recv().await {
                Some(response) => {
                    if response.request_id != request.id {
                        return Err(SageError::agent("Response ID mismatch"));
                    }
                    Ok(response)
                }
                None => Err(SageError::agent("Input channel closed")),
            }
        }
    }

    /// Try to receive a response without blocking
    ///
    /// Returns None if no response is available yet.
    pub fn try_recv(&mut self) -> Option<InputResponse> {
        self.response_rx.try_recv().ok()
    }
}

impl InputChannelHandle {
    /// Send a response back to the execution loop
    pub async fn respond(&self, response: InputResponse) -> SageResult<()> {
        self.response_tx
            .send(response)
            .await
            .map_err(|_| SageError::agent("Input channel closed - execution loop not running"))
    }

    /// Try to receive a request without blocking
    pub fn try_recv_request(&mut self) -> Option<InputRequest> {
        self.request_rx.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_input_channel_basic() {
        let (mut channel, mut handle) = InputChannel::new(16);

        // Spawn responder
        let responder = tokio::spawn(async move {
            if let Some(request) = handle.request_rx.recv().await {
                let response = InputResponse::text(request.id, "user response");
                handle.response_tx.send(response).await.ok();
            }
        });

        let request = InputRequest::simple("What is your choice?");
        let response = channel.request_input(request).await.unwrap();

        assert_eq!(response.get_text(), Some("user response"));
        assert!(!response.is_cancelled());

        responder.abort();
    }

    #[tokio::test]
    async fn test_non_interactive_channel() {
        let mut channel = InputChannel::non_interactive(AutoResponse::AlwaysAllow);

        let request = InputRequest::permission("bash", "Run command", serde_json::json!({}));
        let response = channel.request_input(request).await.unwrap();

        assert!(response.is_permission_granted());
    }

    #[tokio::test]
    async fn test_non_interactive_deny() {
        let mut channel = InputChannel::non_interactive(AutoResponse::AlwaysDeny);

        let request = InputRequest::simple("Question?");
        let response = channel.request_input(request).await.unwrap();

        assert!(response.is_cancelled());
    }

    #[tokio::test]
    async fn test_fail_on_input_channel() {
        let mut channel = InputChannel::fail_on_input();

        let request = InputRequest::simple("Question?");
        let response = channel.request_input(request).await.unwrap();

        assert!(response.is_cancelled());
    }

    #[tokio::test]
    async fn test_questions_request() {
        let mut channel = InputChannel::non_interactive(AutoResponse::AlwaysAllow);

        let questions = vec![Question::new(
            "Which framework?",
            "Framework",
            vec![
                QuestionOption::new("React", "Popular UI library"),
                QuestionOption::new("Vue", "Progressive framework"),
            ],
        )];

        let request = InputRequest::questions(questions);
        let response = channel.request_input(request).await.unwrap();

        let answers = response.get_answers().unwrap();
        assert!(answers.contains_key("Which framework?"));
    }

    #[test]
    fn test_input_option() {
        let opt = InputOption::new("Label", "Description");
        assert_eq!(opt.get_value(), "Label");

        let opt_with_value = InputOption::new("Label", "Desc").with_value("custom_value");
        assert_eq!(opt_with_value.get_value(), "custom_value");
    }

    #[test]
    fn test_question_validation() {
        let valid_question = Question::new(
            "Which one?",
            "Choice",
            vec![
                QuestionOption::new("A", "Option A"),
                QuestionOption::new("B", "Option B"),
            ],
        );
        assert!(valid_question.validate().is_ok());

        let invalid_header = Question::new(
            "Which one?",
            "This header is way too long",
            vec![
                QuestionOption::new("A", "Option A"),
                QuestionOption::new("B", "Option B"),
            ],
        );
        assert!(invalid_header.validate().is_err());

        let too_few_options = Question::new(
            "Which one?",
            "Choice",
            vec![QuestionOption::new("A", "Option A")],
        );
        assert!(too_few_options.validate().is_err());
    }

    #[test]
    fn test_permission_result() {
        let allow = PermissionResult::Allow;
        assert!(allow.is_allowed());
        assert!(!allow.is_denied());
        assert!(!allow.needs_user_input());

        let deny = PermissionResult::Deny {
            message: "Not allowed".to_string(),
        };
        assert!(!deny.is_allowed());
        assert!(deny.is_denied());

        let ask = PermissionResult::Ask {
            message: "Confirm?".to_string(),
            suggestions: vec![],
        };
        assert!(ask.needs_user_input());
    }
}
