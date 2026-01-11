//! Sage CLI Main Application (rnk-based)
//!
//! Complete declarative UI with keyboard input and Agent integration.

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
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
use std::time::Duration;
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
pub fn sage_app(state: Arc<RwLock<AppState>>) -> Element {
    let current_state = state.read().unwrap().clone();

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

/// Keyboard event handler - runs in a separate thread
fn spawn_keyboard_handler(
    state: Arc<RwLock<AppState>>,
    action_tx: mpsc::UnboundedSender<UserAction>,
) {
    std::thread::spawn(move || {
        loop {
            if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                if let Ok(Event::Key(key_event)) = event::read() {
                    if let Some(action) = handle_key_event(key_event, &state) {
                        if action_tx.send(action).is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });
}

/// Handle a single key event
fn handle_key_event(key: KeyEvent, state: &Arc<RwLock<AppState>>) -> Option<UserAction> {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(UserAction::Exit)
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(UserAction::Exit)
        }
        KeyCode::Esc => Some(UserAction::Cancel),
        KeyCode::Enter => {
            let text = state.read().unwrap().input.text.clone();
            if !text.is_empty() {
                {
                    let mut s = state.write().unwrap();
                    s.input.text.clear();
                    s.input.cursor_pos = 0;
                }
                Some(UserAction::Submit(text))
            } else {
                None
            }
        }
        KeyCode::Backspace => {
            let mut s = state.write().unwrap();
            if s.input.cursor_pos > 0 {
                let pos = s.input.cursor_pos - 1;
                s.input.text.remove(pos);
                s.input.cursor_pos = pos;
            }
            None
        }
        KeyCode::Delete => {
            let mut s = state.write().unwrap();
            let pos = s.input.cursor_pos;
            if pos < s.input.text.len() {
                s.input.text.remove(pos);
            }
            None
        }
        KeyCode::Left => {
            let mut s = state.write().unwrap();
            if s.input.cursor_pos > 0 {
                s.input.cursor_pos -= 1;
            }
            None
        }
        KeyCode::Right => {
            let mut s = state.write().unwrap();
            if s.input.cursor_pos < s.input.text.len() {
                s.input.cursor_pos += 1;
            }
            None
        }
        KeyCode::Home => {
            let mut s = state.write().unwrap();
            s.input.cursor_pos = 0;
            None
        }
        KeyCode::End => {
            let mut s = state.write().unwrap();
            s.input.cursor_pos = s.input.text.len();
            None
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let mut s = state.write().unwrap();
            s.input.text.clear();
            s.input.cursor_pos = 0;
            None
        }
        KeyCode::Char(c) => {
            let mut s = state.write().unwrap();
            let pos = s.input.cursor_pos;
            s.input.text.insert(pos, c);
            s.input.cursor_pos += 1;
            None
        }
        _ => None,
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
    Icons::init_from_env();

    // Create app state
    let app_state = SageAppState::new().with_session("claude-sonnet-4-20250514", "anthropic");

    // Set global adapter for Agent events
    set_global_adapter(app_state.adapter.clone());

    let state = app_state.state();

    // Create action channel
    let (action_tx, mut action_rx) = mpsc::unbounded_channel::<UserAction>();

    // Spawn keyboard handler
    spawn_keyboard_handler(Arc::clone(&state), action_tx);

    // Enable raw mode
    enable_raw_mode()?;

    // Spawn Agent executor thread
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

    // Run the rnk app
    let result = render(move || sage_app(Arc::clone(&state)));

    // Cleanup
    disable_raw_mode()?;

    result
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
    render(move || sage_app(Arc::clone(&state)))
}
