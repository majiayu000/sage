//! External UI runtime adapter.
//!
//! Provides a minimal command API for GUI/service integrations:
//! - start task
//! - cancel task
//! - respond to input requests
//! - switch model
//!
//! This adapter keeps execution logic in `UnifiedExecutor` while exposing
//! framework-agnostic state and input channels.

use crate::agent::{ExecutionOutcome, UnifiedExecutor};
use crate::error::{SageError, SageResult};
use crate::input::{InputChannel, InputRequestDto, InputResponse, InputResponseDto};
use crate::interrupt::{InterruptReason, interrupt_current_task, reset_global_interrupt_manager};
use crate::output::UiEventOutput;
use crate::types::TaskMetadata;
use crate::ui::bridge::{AgentEvent, AppState, EventAdapter, StateReceiver};
use crate::ui::traits::{EventSink, UiContext};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinHandle;

pub struct ExternalUiRuntime {
    executor: Arc<Mutex<UnifiedExecutor>>,
    event_adapter: Arc<EventAdapter>,
    input_request_rx: Arc<Mutex<mpsc::UnboundedReceiver<InputRequestDto>>>,
    input_response_tx: mpsc::Sender<InputResponse>,
    running_task: Arc<Mutex<Option<JoinHandle<SageResult<ExecutionOutcome>>>>>,
    input_relay_handle: JoinHandle<()>,
}

impl ExternalUiRuntime {
    /// Create a new runtime adapter around a pre-configured executor.
    ///
    /// Notes:
    /// - This method injects `UiContext` + `UiEventOutput` for event-driven rendering.
    /// - It also configures an `InputChannel` and surfaces requests as DTOs.
    pub fn new(mut executor: UnifiedExecutor) -> Self {
        let event_adapter = Arc::new(EventAdapter::with_default_state());
        let sink = Arc::new(AdapterEventSink::new(Arc::clone(&event_adapter)));
        let ui_context = UiContext::new(sink);

        executor.set_ui_context(ui_context.clone());
        executor.set_output_strategy(Arc::new(UiEventOutput::new(ui_context)));

        let (input_channel, mut input_handle) = InputChannel::new(32);
        executor.set_input_channel(input_channel);

        let input_response_tx = input_handle.response_tx.clone();
        let (input_request_tx, input_request_rx) = mpsc::unbounded_channel();
        let input_relay_handle = tokio::spawn(async move {
            while let Some(request) = input_handle.request_rx.recv().await {
                if input_request_tx
                    .send(InputRequestDto::from(request))
                    .is_err()
                {
                    break;
                }
            }
        });

        Self {
            executor: Arc::new(Mutex::new(executor)),
            event_adapter,
            input_request_rx: Arc::new(Mutex::new(input_request_rx)),
            input_response_tx,
            running_task: Arc::new(Mutex::new(None)),
            input_relay_handle,
        }
    }

    /// Subscribe to AppState changes for real-time UI rendering.
    pub fn subscribe_state(&self) -> StateReceiver<AppState> {
        self.event_adapter.subscribe()
    }

    /// Get an immediate state snapshot.
    pub fn state_snapshot(&self) -> AppState {
        self.event_adapter.get_state()
    }

    /// Start executing a new task.
    ///
    /// Returns an error if another task is still running.
    pub async fn start_task(&self, task_description: impl Into<String>) -> SageResult<()> {
        let task_description = task_description.into();

        {
            let mut running = self.running_task.lock().await;
            if running.as_ref().is_some_and(|handle| !handle.is_finished()) {
                return Err(SageError::invalid_input(
                    "A task is already running; cancel or wait for completion.",
                ));
            }
            if running.as_ref().is_some_and(|handle| handle.is_finished()) {
                *running = None;
            }
        }

        let task = {
            let executor = self.executor.lock().await;
            let working_dir = executor
                .options()
                .working_directory
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
            TaskMetadata::new(&task_description, &working_dir.display().to_string())
        };

        reset_global_interrupt_manager();

        let executor = Arc::clone(&self.executor);
        let handle = tokio::spawn(async move {
            let mut locked = executor.lock().await;
            locked.execute(task).await
        });

        let mut running = self.running_task.lock().await;
        *running = Some(handle);
        Ok(())
    }

    /// Try to collect a finished execution outcome.
    ///
    /// Returns:
    /// - `Some(result)` if the task has completed
    /// - `None` if no task is running or it's still in progress
    pub async fn take_finished_outcome(&self) -> Option<SageResult<ExecutionOutcome>> {
        let handle = {
            let mut running = self.running_task.lock().await;
            if running.as_ref().is_some_and(|h| h.is_finished()) {
                running.take()
            } else {
                None
            }
        }?;

        match handle.await {
            Ok(result) => Some(result),
            Err(err) => Some(Err(SageError::agent(format!(
                "Execution task join error: {}",
                err
            )))),
        }
    }

    /// Cancel the currently running task.
    pub fn cancel_task(&self) {
        interrupt_current_task(InterruptReason::UserInterrupt);
    }

    /// Respond to a pending input request from the UI.
    pub async fn respond_input(&self, response: InputResponseDto) -> SageResult<()> {
        let response = InputResponse::try_from(response)
            .map_err(|e| SageError::invalid_input(format!("Invalid input response DTO: {}", e)))?;

        self.input_response_tx
            .send(response)
            .await
            .map_err(|_| SageError::agent("Input response channel is closed"))
    }

    /// Receive the next pending input request (if any).
    pub async fn recv_input_request(&self) -> Option<InputRequestDto> {
        let mut rx = self.input_request_rx.lock().await;
        rx.recv().await
    }

    /// Non-blocking input request poll.
    pub async fn try_recv_input_request(&self) -> Option<InputRequestDto> {
        let mut rx = self.input_request_rx.lock().await;
        rx.try_recv().ok()
    }

    /// Switch model when no task is currently running.
    pub async fn switch_model(&self, model: &str) -> SageResult<String> {
        {
            let running = self.running_task.lock().await;
            if running.as_ref().is_some_and(|handle| !handle.is_finished()) {
                return Err(SageError::invalid_input(
                    "Cannot switch model while task is running",
                ));
            }
        }

        let mut executor = self.executor.lock().await;
        executor.switch_model(model)
    }
}

impl Drop for ExternalUiRuntime {
    fn drop(&mut self) {
        self.input_relay_handle.abort();
    }
}

struct AdapterEventSink {
    adapter: Arc<EventAdapter>,
}

impl AdapterEventSink {
    fn new(adapter: Arc<EventAdapter>) -> Self {
        Self { adapter }
    }
}

impl EventSink for AdapterEventSink {
    fn handle_event(&self, event: AgentEvent) {
        self.adapter.handle_event(event);
    }

    fn request_refresh(&self) {
        // External frontends should use state subscriptions; no-op here.
    }
}
