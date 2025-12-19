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
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::error::{SageError, SageResult};

/// Input request sent from execution loop to user interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputRequest {
    /// Unique ID for this request
    pub id: Uuid,
    /// The question to display to the user (markdown formatted)
    pub question: String,
    /// Optional structured options for selection
    pub options: Option<Vec<InputOption>>,
    /// Whether multiple selections are allowed
    pub multi_select: bool,
    /// Optional timeout for auto-response (None = wait indefinitely)
    pub timeout: Option<Duration>,
    /// Context about why input is needed
    pub context: InputContext,
}

impl InputRequest {
    /// Create a new input request
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

/// User's response to an input request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputResponse {
    /// ID matching the request
    pub request_id: Uuid,
    /// User's text response
    pub content: String,
    /// Selected option indices (for multi-select)
    pub selected_indices: Option<Vec<usize>>,
    /// Whether user cancelled/skipped
    pub cancelled: bool,
}

impl InputResponse {
    /// Create a successful response with text content
    pub fn text(request_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            request_id,
            content: content.into(),
            selected_indices: None,
            cancelled: false,
        }
    }

    /// Create a response with selected option index
    pub fn selected(request_id: Uuid, index: usize, content: impl Into<String>) -> Self {
        Self {
            request_id,
            content: content.into(),
            selected_indices: Some(vec![index]),
            cancelled: false,
        }
    }

    /// Create a response with multiple selected indices
    pub fn multi_selected(
        request_id: Uuid,
        indices: Vec<usize>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            request_id,
            content: content.into(),
            selected_indices: Some(indices),
            cancelled: false,
        }
    }

    /// Create a cancelled response
    pub fn cancelled(request_id: Uuid) -> Self {
        Self {
            request_id,
            content: String::new(),
            selected_indices: None,
            cancelled: true,
        }
    }
}

/// Auto-responder function type
pub type AutoResponder = Box<dyn Fn(&InputRequest) -> InputResponse + Send + Sync>;

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
    /// The `default_response` is returned for all input requests.
    pub fn non_interactive(default_response: impl Into<String>) -> Self {
        let response = default_response.into();

        // Create dummy channels (won't be used)
        let (request_tx, _) = mpsc::channel(1);
        let (_, response_rx) = mpsc::channel(1);

        Self {
            request_tx,
            response_rx,
            default_timeout: Some(Duration::from_millis(10)),
            auto_responder: Some(Box::new(move |req| {
                // If there are options, select the first one
                if req.options.is_some() {
                    InputResponse::selected(req.id, 0, response.clone())
                } else {
                    InputResponse::text(req.id, response.clone())
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

        let request = InputRequest::new("What is your choice?");
        let response = channel.request_input(request).await.unwrap();

        assert_eq!(response.content, "user response");
        assert!(!response.cancelled);

        responder.abort();
    }

    #[tokio::test]
    async fn test_non_interactive_channel() {
        let mut channel = InputChannel::non_interactive("auto response");

        let request = InputRequest::new("Question?");
        let response = channel.request_input(request).await.unwrap();

        assert_eq!(response.content, "auto response");
        assert!(!response.cancelled);
    }

    #[tokio::test]
    async fn test_non_interactive_with_options() {
        let mut channel = InputChannel::non_interactive("selected");

        let request = InputRequest::new("Choose one:")
            .with_options(vec![
                InputOption::new("Option A", "First option"),
                InputOption::new("Option B", "Second option"),
            ]);

        let response = channel.request_input(request).await.unwrap();

        assert_eq!(response.selected_indices, Some(vec![0]));
    }

    #[tokio::test]
    async fn test_fail_on_input_channel() {
        let mut channel = InputChannel::fail_on_input();

        let request = InputRequest::new("Question?");
        let response = channel.request_input(request).await.unwrap();

        assert!(response.cancelled);
    }

    #[test]
    fn test_input_option() {
        let opt = InputOption::new("Label", "Description");
        assert_eq!(opt.get_value(), "Label");

        let opt_with_value = InputOption::new("Label", "Desc").with_value("custom_value");
        assert_eq!(opt_with_value.get_value(), "custom_value");
    }

    #[test]
    fn test_input_request_builder() {
        let request = InputRequest::new("Question?")
            .with_options(vec![InputOption::new("A", "Option A")])
            .with_multi_select(true)
            .with_timeout(Duration::from_secs(30))
            .with_context(InputContext::Decision);

        assert!(request.options.is_some());
        assert!(request.multi_select);
        assert_eq!(request.timeout, Some(Duration::from_secs(30)));
        assert_eq!(request.context, InputContext::Decision);
    }
}
