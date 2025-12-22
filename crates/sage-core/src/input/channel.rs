//! Input channel for async communication between execution loop and UI

use std::time::Duration;
use tokio::sync::mpsc;

use crate::error::{SageError, SageResult};

use super::auto_response::{AutoResponse, AutoResponder};
use super::request::InputRequest;
use super::response::InputResponse;

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
                use super::request::InputRequestKind;
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
