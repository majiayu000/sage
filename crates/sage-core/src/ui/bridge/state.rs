//! Application State - Core state model for UI rendering
//!
//! This module defines the complete application state that drives
//! the declarative UI. All UI components read from this state.

use chrono::{DateTime, Utc};
use std::time::{Duration, Instant};

/// Application global state (immutable, updated via Signal)
#[derive(Clone, Debug)]
pub struct AppState {
    /// Current execution phase
    pub phase: ExecutionPhase,

    /// Session information
    pub session: SessionState,

    /// Message history
    pub messages: Vec<Message>,

    /// Current input state
    pub input: InputState,

    /// Tool execution state
    pub tool_execution: Option<ToolExecution>,

    /// Thinking state
    pub thinking: Option<ThinkingState>,

    /// Streaming content buffer
    pub streaming_content: Option<StreamingContent>,

    /// UI configuration
    pub ui_config: UiConfig,
}

/// Execution phase
#[derive(Clone, Debug, PartialEq)]
pub enum ExecutionPhase {
    /// Idle, waiting for user input
    Idle,

    /// Thinking
    Thinking,

    /// Receiving streaming response
    Streaming { started_at: Instant },

    /// Executing tool
    ExecutingTool {
        tool_name: String,
        started_at: Instant,
    },

    /// Waiting for user confirmation
    WaitingConfirmation { prompt: String },

    /// Error state
    Error { message: String },
}

/// Session state
#[derive(Clone, Debug)]
pub struct SessionState {
    pub session_id: Option<String>,
    pub model: String,
    pub provider: String,
    pub working_dir: String,
    pub git_branch: Option<String>,
    pub step: u32,
    pub max_steps: Option<u32>,
}

/// Message
#[derive(Clone, Debug)]
pub struct Message {
    pub role: Role,
    pub content: MessageContent,
    pub timestamp: DateTime<Utc>,
    pub metadata: MessageMetadata,
}

/// Message role
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
}

/// Message content
#[derive(Clone, Debug)]
pub enum MessageContent {
    /// Plain text
    Text(String),

    /// Tool call
    ToolCall {
        tool_name: String,
        params: String,
        result: Option<ToolResult>,
    },

    /// Thinking process
    Thinking(String),
}

/// Message metadata
#[derive(Clone, Debug, Default)]
pub struct MessageMetadata {
    pub truncated: bool,
    pub formatted: bool,
    pub cost: Option<f64>,
}

/// Tool execution state
#[derive(Clone, Debug)]
pub struct ToolExecution {
    pub tool_name: String,
    pub description: String,
    pub status: ToolStatus,
    pub started_at: Instant,
    pub output_preview: Option<String>,
}

/// Tool status
#[derive(Clone, Debug, PartialEq)]
pub enum ToolStatus {
    Running,
    Completed { duration: Duration },
    Failed { error: String },
}

/// Tool result
#[derive(Clone, Debug)]
pub struct ToolResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration: Duration,
}

/// Thinking state
#[derive(Clone, Debug)]
pub struct ThinkingState {
    pub started_at: Instant,
    pub completed: bool,
    pub duration: Option<Duration>,
    pub preview: Option<String>,
}

/// Streaming content
#[derive(Clone, Debug)]
pub struct StreamingContent {
    pub buffer: String,
    pub started_at: Instant,
    pub last_update: Instant,
}

/// Input state
#[derive(Clone, Debug, Default)]
pub struct InputState {
    pub text: String,
    pub cursor_pos: usize,
    pub enabled: bool,
}

/// UI configuration
#[derive(Clone, Debug)]
pub struct UiConfig {
    pub use_nerd_fonts: bool,
    pub show_timestamps: bool,
    pub show_cost: bool,
    pub markdown_enabled: bool,
    pub theme: Theme,
}

/// Theme
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
    Auto,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            phase: ExecutionPhase::Idle,
            session: SessionState {
                session_id: None,
                model: "unknown".to_string(),
                provider: "unknown".to_string(),
                working_dir: ".".to_string(),
                git_branch: None,
                step: 0,
                max_steps: None,
            },
            messages: Vec::new(),
            input: InputState {
                text: String::new(),
                cursor_pos: 0,
                enabled: true,
            },
            tool_execution: None,
            thinking: None,
            streaming_content: None,
            ui_config: UiConfig {
                use_nerd_fonts: true,
                show_timestamps: false,
                show_cost: true,
                markdown_enabled: true,
                theme: Theme::Dark,
            },
        }
    }
}

impl AppState {
    /// Get current display messages including streaming content
    pub fn display_messages(&self) -> Vec<Message> {
        let mut messages = self.messages.clone();

        // If streaming, add temporary message
        if let Some(streaming) = &self.streaming_content {
            messages.push(Message {
                role: Role::Assistant,
                content: MessageContent::Text(streaming.buffer.clone()),
                timestamp: Utc::now(),
                metadata: MessageMetadata::default(),
            });
        }

        messages
    }

    /// Get status bar text
    pub fn status_text(&self) -> String {
        match &self.phase {
            ExecutionPhase::Idle => "Ready".to_string(),
            ExecutionPhase::Thinking => {
                if let Some(thinking) = &self.thinking {
                    let elapsed = thinking.started_at.elapsed().as_secs_f32();
                    format!("Thinking ({:.1}s)", elapsed)
                } else {
                    "Thinking...".to_string()
                }
            }
            ExecutionPhase::Streaming { started_at } => {
                let elapsed = started_at.elapsed().as_secs_f32();
                format!("Streaming ({:.1}s)", elapsed)
            }
            ExecutionPhase::ExecutingTool {
                tool_name,
                started_at,
            } => {
                let elapsed = started_at.elapsed().as_secs_f32();
                format!("Running {} ({:.1}s)", tool_name, elapsed)
            }
            ExecutionPhase::WaitingConfirmation { .. } => "Waiting for confirmation".to_string(),
            ExecutionPhase::Error { .. } => "Error".to_string(),
        }
    }

    /// Start thinking
    pub fn start_thinking(&mut self) {
        self.phase = ExecutionPhase::Thinking;
        self.thinking = Some(ThinkingState {
            started_at: Instant::now(),
            completed: false,
            duration: None,
            preview: None,
        });
    }

    /// Stop thinking
    pub fn stop_thinking(&mut self) {
        if let Some(thinking) = &mut self.thinking {
            thinking.completed = true;
            thinking.duration = Some(thinking.started_at.elapsed());
        }
        // Don't overwrite Error state
        if !matches!(self.phase, ExecutionPhase::Error { .. }) {
            self.phase = ExecutionPhase::Idle;
        }
    }

    /// Start streaming
    pub fn start_streaming(&mut self) {
        let now = Instant::now();
        self.phase = ExecutionPhase::Streaming { started_at: now };
        self.streaming_content = Some(StreamingContent {
            buffer: String::new(),
            started_at: now,
            last_update: now,
        });
    }

    /// Append streaming chunk
    pub fn append_streaming_chunk(&mut self, chunk: &str) {
        if let Some(content) = &mut self.streaming_content {
            content.buffer.push_str(chunk);
            content.last_update = Instant::now();
        }
    }

    /// Finish streaming
    pub fn finish_streaming(&mut self) {
        if let Some(content) = self.streaming_content.take() {
            self.messages.push(Message {
                role: Role::Assistant,
                content: MessageContent::Text(content.buffer),
                timestamp: Utc::now(),
                metadata: MessageMetadata::default(),
            });
        }
        self.phase = ExecutionPhase::Idle;
    }

    /// Start tool execution
    pub fn start_tool(&mut self, tool_name: String, description: String) {
        let now = Instant::now();
        self.phase = ExecutionPhase::ExecutingTool {
            tool_name: tool_name.clone(),
            started_at: now,
        };
        self.tool_execution = Some(ToolExecution {
            tool_name,
            description,
            status: ToolStatus::Running,
            started_at: now,
            output_preview: None,
        });
    }

    /// Finish tool execution
    pub fn finish_tool(&mut self, success: bool, output: Option<String>, error: Option<String>) {
        if let Some(execution) = &mut self.tool_execution {
            let duration = execution.started_at.elapsed();
            execution.status = if success {
                ToolStatus::Completed { duration }
            } else {
                ToolStatus::Failed {
                    error: error.clone().unwrap_or_else(|| "Unknown error".to_string()),
                }
            };

            // Add tool call message
            self.messages.push(Message {
                role: Role::Assistant,
                content: MessageContent::ToolCall {
                    tool_name: execution.tool_name.clone(),
                    params: execution.description.clone(),
                    result: Some(ToolResult {
                        success,
                        output,
                        error,
                        duration,
                    }),
                },
                timestamp: Utc::now(),
                metadata: MessageMetadata::default(),
            });
        }
        self.tool_execution = None;
        self.phase = ExecutionPhase::Idle;
    }

    /// Add user message
    pub fn add_user_message(&mut self, content: String) {
        self.messages.push(Message {
            role: Role::User,
            content: MessageContent::Text(content),
            timestamp: Utc::now(),
            metadata: MessageMetadata::default(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_thinking_cycle() {
        let mut state = AppState::default();

        state.start_thinking();
        assert!(matches!(state.phase, ExecutionPhase::Thinking));
        assert!(state.thinking.is_some());

        state.stop_thinking();
        assert!(matches!(state.phase, ExecutionPhase::Idle));
        assert!(state.thinking.as_ref().unwrap().completed);
    }

    #[test]
    fn test_streaming_content() {
        let mut state = AppState::default();

        state.start_streaming();
        state.append_streaming_chunk("Hello ");
        state.append_streaming_chunk("World");

        assert_eq!(
            state.streaming_content.as_ref().unwrap().buffer,
            "Hello World"
        );

        state.finish_streaming();

        if let MessageContent::Text(text) = &state.messages.last().unwrap().content {
            assert_eq!(text, "Hello World");
        } else {
            panic!("Expected Text content");
        }
    }

    #[test]
    fn test_tool_execution() {
        let mut state = AppState::default();

        state.start_tool("bash".to_string(), "ls -la".to_string());
        assert!(matches!(
            state.phase,
            ExecutionPhase::ExecutingTool { .. }
        ));

        state.finish_tool(true, Some("file1\nfile2".to_string()), None);
        assert!(matches!(state.phase, ExecutionPhase::Idle));
        assert!(state.tool_execution.is_none());
    }
}
