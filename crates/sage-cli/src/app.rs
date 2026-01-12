//! Sage CLI Main Application (rnk-based)
//!
//! Uses rnk's internal event loop with use_input hook for keyboard handling.

use crossterm::tty::IsTty;
use rnk::core::Dimension;
use rnk::prelude::*;
use sage_core::agent::{ExecutionMode, ExecutionOptions, ExecutionOutcome, UnifiedExecutor};
use sage_core::config::load_config;
use sage_core::error::SageResult;
use sage_core::input::InputChannel;
use sage_core::output::OutputMode;
use sage_core::types::TaskMetadata;
use sage_core::ui::{
    bridge::{emit_event, set_global_adapter, AgentEvent, AppState, EventAdapter, ExecutionPhase},
    components::{InputBox, MessageList, StatusBar, ThinkingIndicator, ToolExecutionView},
    theme::Icons,
};
use sage_tools::get_default_tools;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

/// User action from keyboard input
#[derive(Debug, Clone)]
pub enum UserAction {
    /// Submit the current input
    Submit(String),
    /// Exit the application
    Exit,
    /// Cancel current operation
    Cancel,
}

/// Sage CLI main application component
pub fn sage_app(state: Arc<RwLock<AppState>>, action_tx: mpsc::UnboundedSender<UserAction>) -> Element {
    let current_state = state.read().unwrap().clone();
    let state_for_input = Arc::clone(&state);
    let app = use_app();

    // Register keyboard input handler using rnk's use_input hook
    use_input(move |input, key| {
        // Ctrl+C or Ctrl+D to exit
        if key.ctrl && (input == "c" || input == "d") {
            let _ = action_tx.send(UserAction::Exit);
            app.exit();
            return;
        }

        // Escape to cancel
        if key.escape {
            let _ = action_tx.send(UserAction::Cancel);
            return;
        }

        // Enter to submit
        if key.return_key {
            let text = state_for_input.read().unwrap().input.text.clone();
            if !text.is_empty() {
                {
                    let mut s = state_for_input.write().unwrap();
                    s.input.text.clear();
                    s.input.cursor_pos = 0;
                }
                let _ = action_tx.send(UserAction::Submit(text));
            }
            return;
        }

        // Backspace - remove character before cursor
        if key.backspace {
            let mut s = state_for_input.write().unwrap();
            if s.input.cursor_pos > 0 {
                let new_pos = s.input.cursor_pos - 1;
                // Convert character position to byte position for String::remove
                if let Some((byte_pos, _)) = s.input.text.char_indices().nth(new_pos) {
                    s.input.text.remove(byte_pos);
                }
                s.input.cursor_pos = new_pos;
            }
            return;
        }

        // Delete - remove character at cursor
        if key.delete {
            let mut s = state_for_input.write().unwrap();
            let char_pos = s.input.cursor_pos;
            // Convert character position to byte position
            if let Some((byte_pos, _)) = s.input.text.char_indices().nth(char_pos) {
                s.input.text.remove(byte_pos);
            }
            return;
        }

        // Left arrow
        if key.left_arrow {
            let mut s = state_for_input.write().unwrap();
            if s.input.cursor_pos > 0 {
                s.input.cursor_pos -= 1;
            }
            return;
        }

        // Right arrow
        if key.right_arrow {
            let mut s = state_for_input.write().unwrap();
            let char_count = s.input.text.chars().count();
            if s.input.cursor_pos < char_count {
                s.input.cursor_pos += 1;
            }
            return;
        }

        // Home
        if key.home {
            let mut s = state_for_input.write().unwrap();
            s.input.cursor_pos = 0;
            return;
        }

        // End
        if key.end {
            let mut s = state_for_input.write().unwrap();
            s.input.cursor_pos = s.input.text.chars().count();
            return;
        }

        // Ctrl+U to clear line
        if key.ctrl && input == "u" {
            let mut s = state_for_input.write().unwrap();
            s.input.text.clear();
            s.input.cursor_pos = 0;
            return;
        }

        // Regular character input (not control characters)
        // Filter out control characters and empty input
        if !input.is_empty() && !key.ctrl && !key.alt {
            // Skip non-printable characters
            let printable: String = input.chars().filter(|c| !c.is_control()).collect();
            if printable.is_empty() {
                return;
            }

            let mut s = state_for_input.write().unwrap();

            for c in printable.chars() {
                let char_pos = s.input.cursor_pos;
                // Convert character position to byte position for String::insert
                let byte_pos = s.input.text
                    .char_indices()
                    .nth(char_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(s.input.text.len());

                s.input.text.insert(byte_pos, c);
                s.input.cursor_pos += 1;
            }
        }
    });

    Box::new()
        .flex_direction(FlexDirection::Column)
        .height(Dimension::Percent(100.0))
        // Status bar at top
        .child(
            StatusBar::new(current_state.session.clone(), current_state.phase.clone()).render(),
        )
        // Message area (scrollable)
        .child(
            Box::new()
                .flex_direction(FlexDirection::Column)
                .flex_grow(1.0)
                .overflow_y(Overflow::Scroll)
                // Message list
                .child(MessageList(current_state.display_messages()))
                // Thinking indicator
                .child(if let Some(thinking) = current_state.thinking.clone() {
                    ThinkingIndicator::new(thinking).render()
                } else {
                    Box::new().into_element()
                })
                // Tool execution
                .child(if let Some(tool) = current_state.tool_execution.clone() {
                    ToolExecutionView::new(tool).render()
                } else {
                    Box::new().into_element()
                })
                .into_element(),
        )
        // Input box at bottom
        .child(InputBox::new(current_state.input.clone()).render())
        .into_element()
}

/// Application state holder
pub struct SageAppState {
    pub adapter: EventAdapter,
    pub state: Arc<RwLock<AppState>>,
}

impl SageAppState {
    pub fn new() -> Self {
        let adapter = EventAdapter::with_default_state();
        let state = adapter.state_handle();
        Self { adapter, state }
    }

    pub fn with_session(self, model: impl Into<String>, provider: impl Into<String>) -> Self {
        self.adapter.handle_event(AgentEvent::session_started(
            uuid::Uuid::new_v4().to_string(),
            model,
            provider,
        ));
        self
    }

    #[allow(dead_code)]
    pub fn event_adapter(&self) -> EventAdapter {
        self.adapter.clone()
    }

    pub fn state(&self) -> Arc<RwLock<AppState>> {
        Arc::clone(&self.state)
    }
}

impl Default for SageAppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Create and configure UnifiedExecutor
async fn create_executor() -> SageResult<UnifiedExecutor> {
    let config = load_config()?;

    let working_dir = std::env::current_dir().unwrap_or_default();

    let mode = ExecutionMode::interactive();
    let options = ExecutionOptions::default()
        .with_mode(mode)
        .with_working_directory(&working_dir);

    let mut executor = UnifiedExecutor::with_options(config, options)?;

    // Use Rnk output mode
    executor.set_output_mode(OutputMode::Rnk);

    // Register default tools
    executor.register_tools(get_default_tools());

    // Initialize sub-agent support
    let _ = executor.init_subagent_support();

    Ok(executor)
}

/// Run the Sage CLI application with rnk and full Agent integration
pub fn run_app() -> std::io::Result<()> {
    // Check if we have a TTY - rnk requires an interactive terminal
    if !std::io::stdin().is_tty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Sage requires an interactive terminal. For non-interactive use, try: sage -p \"your task\""
        ));
    }

    Icons::init_from_env();

    // Create app state
    let app_state = SageAppState::new().with_session("claude-sonnet-4-20250514", "anthropic");

    // Set global adapter for Agent events
    set_global_adapter(app_state.adapter.clone());

    let state = app_state.state();

    // Create action channel
    let (action_tx, mut action_rx) = mpsc::unbounded_channel::<UserAction>();

    // Spawn Agent executor in background thread with tokio runtime
    let state_clone = Arc::clone(&state);
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Create executor
            let executor_result = create_executor().await;
            let mut executor = match executor_result {
                Ok(e) => e,
                Err(e) => {
                    emit_event(AgentEvent::ErrorOccurred {
                        error_type: "InitError".to_string(),
                        message: e.to_string(),
                    });
                    return;
                }
            };

            // Set up input channel
            let (input_channel, _input_handle) = InputChannel::new(16);
            executor.set_input_channel(input_channel);

            // Process user actions
            while let Some(action) = action_rx.recv().await {
                match action {
                    UserAction::Submit(text) => {
                        // Handle exit command
                        if text == "/exit" || text == "/quit" {
                            std::process::exit(0);
                        }

                        // Add user message to UI
                        emit_event(AgentEvent::UserInputReceived {
                            input: text.clone(),
                        });

                        // Execute task
                        emit_event(AgentEvent::ThinkingStarted);

                        let working_dir = std::env::current_dir()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let task = TaskMetadata::new(&text, &working_dir);
                        match executor.execute(task).await {
                            Ok(outcome) => {
                                emit_event(AgentEvent::ThinkingStopped);

                                // Extract final response based on outcome type
                                let response = match outcome {
                                    ExecutionOutcome::Success(exec) => exec.final_result,
                                    ExecutionOutcome::NeedsUserInput { last_response, .. } => {
                                        Some(last_response)
                                    }
                                    ExecutionOutcome::Failed { error, .. } => {
                                        Some(format!("Error: {}", error.message))
                                    }
                                    ExecutionOutcome::MaxStepsReached { .. } => {
                                        Some("Max steps reached".to_string())
                                    }
                                    ExecutionOutcome::Interrupted { .. } => {
                                        Some("Interrupted".to_string())
                                    }
                                    ExecutionOutcome::UserCancelled { .. } => {
                                        Some("Cancelled".to_string())
                                    }
                                };

                                if let Some(response_text) = response {
                                    emit_event(AgentEvent::UserInputReceived {
                                        input: response_text,
                                    });
                                }
                            }
                            Err(e) => {
                                emit_event(AgentEvent::ThinkingStopped);
                                emit_event(AgentEvent::ErrorOccurred {
                                    error_type: "ExecutionError".to_string(),
                                    message: e.to_string(),
                                });
                            }
                        }

                        // Reset to idle
                        {
                            let mut s = state_clone.write().unwrap();
                            s.phase = ExecutionPhase::Idle;
                        }
                    }
                    UserAction::Exit => {
                        std::process::exit(0);
                    }
                    UserAction::Cancel => {
                        emit_event(AgentEvent::ThinkingStopped);
                        let mut s = state_clone.write().unwrap();
                        s.phase = ExecutionPhase::Idle;
                    }
                }
            }
        });
    });

    // Clone for render closure
    let action_tx_clone = action_tx.clone();

    // Run the rnk app - rnk handles raw mode internally
    render(move || sage_app(Arc::clone(&state), action_tx_clone.clone()))
}

/// Demo mode for testing UI
pub fn run_demo() -> std::io::Result<()> {
    use sage_core::ui::bridge::{Message, MessageContent, Role, SessionState};

    Icons::init_from_env();

    let app_state = SageAppState::new();

    {
        let mut state = app_state.state.write().unwrap();
        state.session = SessionState {
            session_id: Some("demo-session".to_string()),
            model: "claude-sonnet-4-20250514".to_string(),
            provider: "anthropic".to_string(),
            working_dir: std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            git_branch: Some("main".to_string()),
            step: 1,
            max_steps: Some(10),
        };

        state.messages.push(Message {
            role: Role::User,
            content: MessageContent::Text("Help me refactor the UI code".to_string()),
            timestamp: chrono::Utc::now(),
            metadata: Default::default(),
        });

        state.messages.push(Message {
            role: Role::Assistant,
            content: MessageContent::Text(
                "I'll help you refactor the UI code. Let me analyze the structure first."
                    .to_string(),
            ),
            timestamp: chrono::Utc::now(),
            metadata: Default::default(),
        });

        state.phase = ExecutionPhase::Idle;
    }

    let state = app_state.state();
    let (action_tx, _) = mpsc::unbounded_channel::<UserAction>();
    render(move || sage_app(Arc::clone(&state), action_tx.clone()))
}
