//! Transport-safe DTOs for external UI/service integration.
//!
//! These DTOs provide serializable shapes for bridge events and state snapshots.

use super::{
    AgentEvent, AppState, ExecutionPhase, InputState, Message, Role, ToolExecution, ToolStatus,
    UiMessageContent, UiToolResult,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEventDto {
    SessionStarted {
        session_id: String,
        model: String,
        provider: String,
    },
    SessionEnded {
        session_id: String,
    },
    ModelSwitched {
        old_model: String,
        new_model: String,
    },
    StepStarted {
        step_number: u32,
    },
    ThinkingStarted,
    ThinkingStopped,
    ContentStreamStarted,
    ContentChunk {
        chunk: String,
    },
    ContentStreamEnded,
    ToolExecutionStarted {
        tool_name: String,
        tool_id: String,
        description: String,
    },
    ToolExecutionCompleted {
        tool_name: String,
        tool_id: String,
        success: bool,
        duration_ms: u64,
        result_preview: Option<String>,
    },
    ErrorOccurred {
        error_type: String,
        message: String,
    },
    UserInputRequested {
        prompt: String,
    },
    UserInputReceived {
        input: String,
    },
    GitBranchChanged {
        branch: String,
    },
    WorkingDirectoryChanged {
        path: String,
    },
}

impl From<&AgentEvent> for AgentEventDto {
    fn from(event: &AgentEvent) -> Self {
        match event {
            AgentEvent::SessionStarted {
                session_id,
                model,
                provider,
            } => Self::SessionStarted {
                session_id: session_id.clone(),
                model: model.clone(),
                provider: provider.clone(),
            },
            AgentEvent::SessionEnded { session_id } => Self::SessionEnded {
                session_id: session_id.clone(),
            },
            AgentEvent::ModelSwitched {
                old_model,
                new_model,
            } => Self::ModelSwitched {
                old_model: old_model.clone(),
                new_model: new_model.clone(),
            },
            AgentEvent::StepStarted { step_number } => Self::StepStarted {
                step_number: *step_number,
            },
            AgentEvent::ThinkingStarted => Self::ThinkingStarted,
            AgentEvent::ThinkingStopped => Self::ThinkingStopped,
            AgentEvent::ContentStreamStarted => Self::ContentStreamStarted,
            AgentEvent::ContentChunk { chunk } => Self::ContentChunk {
                chunk: chunk.clone(),
            },
            AgentEvent::ContentStreamEnded => Self::ContentStreamEnded,
            AgentEvent::ToolExecutionStarted {
                tool_name,
                tool_id,
                description,
            } => Self::ToolExecutionStarted {
                tool_name: tool_name.clone(),
                tool_id: tool_id.clone(),
                description: description.clone(),
            },
            AgentEvent::ToolExecutionCompleted {
                tool_name,
                tool_id,
                success,
                duration_ms,
                result_preview,
            } => Self::ToolExecutionCompleted {
                tool_name: tool_name.clone(),
                tool_id: tool_id.clone(),
                success: *success,
                duration_ms: *duration_ms,
                result_preview: result_preview.clone(),
            },
            AgentEvent::ErrorOccurred {
                error_type,
                message,
            } => Self::ErrorOccurred {
                error_type: error_type.clone(),
                message: message.clone(),
            },
            AgentEvent::UserInputRequested { prompt } => Self::UserInputRequested {
                prompt: prompt.clone(),
            },
            AgentEvent::UserInputReceived { input } => Self::UserInputReceived {
                input: input.clone(),
            },
            AgentEvent::GitBranchChanged { branch } => Self::GitBranchChanged {
                branch: branch.clone(),
            },
            AgentEvent::WorkingDirectoryChanged { path } => {
                Self::WorkingDirectoryChanged { path: path.clone() }
            }
        }
    }
}

impl From<AgentEvent> for AgentEventDto {
    fn from(event: AgentEvent) -> Self {
        Self::from(&event)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppStateDto {
    pub phase: ExecutionPhaseDto,
    pub session: UiSessionInfoDto,
    pub messages: Vec<MessageDto>,
    pub input: InputStateDto,
    pub tool_execution: Option<ToolExecutionDto>,
    pub streaming_content: Option<StreamingContentDto>,
    pub status_text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecutionPhaseDto {
    Idle,
    Thinking { elapsed_ms: Option<u64> },
    Streaming { elapsed_ms: u64 },
    ExecutingTool { tool_name: String, elapsed_ms: u64 },
    WaitingConfirmation { prompt: String },
    Error { message: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UiSessionInfoDto {
    pub session_id: Option<String>,
    pub model: String,
    pub provider: String,
    pub working_dir: String,
    pub git_branch: Option<String>,
    pub step: u32,
    pub max_steps: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputStateDto {
    pub text: String,
    pub cursor_pos: usize,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageDto {
    pub role: RoleDto,
    pub content: UiMessageContentDto,
    pub timestamp: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleDto {
    User,
    Assistant,
    System,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiMessageContentDto {
    Text {
        text: String,
    },
    ToolCall {
        tool_name: String,
        params: String,
        result: Option<UiToolResultDto>,
    },
    Thinking {
        text: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolExecutionDto {
    pub tool_name: String,
    pub description: String,
    pub status: ToolStatusDto,
    pub elapsed_ms: u64,
    pub output_preview: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolStatusDto {
    Running,
    Completed { duration_ms: u64 },
    Failed { error: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UiToolResultDto {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamingContentDto {
    pub buffer: String,
    pub elapsed_ms: u64,
}

impl From<&AppState> for AppStateDto {
    fn from(state: &AppState) -> Self {
        Self {
            phase: ExecutionPhaseDto::from(&state.phase),
            session: UiSessionInfoDto {
                session_id: state.session.session_id.clone(),
                model: state.session.model.clone(),
                provider: state.session.provider.clone(),
                working_dir: state.session.working_dir.clone(),
                git_branch: state.session.git_branch.clone(),
                step: state.session.step,
                max_steps: state.session.max_steps,
            },
            messages: state.messages.iter().map(MessageDto::from).collect(),
            input: InputStateDto::from(&state.input),
            tool_execution: state.tool_execution.as_ref().map(ToolExecutionDto::from),
            streaming_content: state.streaming_content.as_ref().map(|streaming| {
                StreamingContentDto {
                    buffer: streaming.buffer.clone(),
                    elapsed_ms: duration_ms(streaming.started_at.elapsed()),
                }
            }),
            status_text: state.status_text(),
        }
    }
}

impl From<AppState> for AppStateDto {
    fn from(state: AppState) -> Self {
        Self::from(&state)
    }
}

impl From<&ExecutionPhase> for ExecutionPhaseDto {
    fn from(phase: &ExecutionPhase) -> Self {
        match phase {
            ExecutionPhase::Idle => Self::Idle,
            ExecutionPhase::Thinking => Self::Thinking { elapsed_ms: None },
            ExecutionPhase::Streaming { started_at } => Self::Streaming {
                elapsed_ms: duration_ms(started_at.elapsed()),
            },
            ExecutionPhase::ExecutingTool {
                tool_name,
                started_at,
            } => Self::ExecutingTool {
                tool_name: tool_name.clone(),
                elapsed_ms: duration_ms(started_at.elapsed()),
            },
            ExecutionPhase::WaitingConfirmation { prompt } => Self::WaitingConfirmation {
                prompt: prompt.clone(),
            },
            ExecutionPhase::Error { message } => Self::Error {
                message: message.clone(),
            },
        }
    }
}

impl From<&InputState> for InputStateDto {
    fn from(input: &InputState) -> Self {
        Self {
            text: input.text.clone(),
            cursor_pos: input.cursor_pos,
            enabled: input.enabled,
        }
    }
}

impl From<&Message> for MessageDto {
    fn from(message: &Message) -> Self {
        Self {
            role: RoleDto::from(&message.role),
            content: UiMessageContentDto::from(&message.content),
            timestamp: message.timestamp.to_rfc3339(),
        }
    }
}

impl From<&Role> for RoleDto {
    fn from(role: &Role) -> Self {
        match role {
            Role::User => Self::User,
            Role::Assistant => Self::Assistant,
            Role::System => Self::System,
        }
    }
}

impl From<&UiMessageContent> for UiMessageContentDto {
    fn from(content: &UiMessageContent) -> Self {
        match content {
            UiMessageContent::Text(text) => Self::Text { text: text.clone() },
            UiMessageContent::ToolCall {
                tool_name,
                params,
                result,
            } => Self::ToolCall {
                tool_name: tool_name.clone(),
                params: params.clone(),
                result: result.as_ref().map(UiToolResultDto::from),
            },
            UiMessageContent::Thinking(text) => Self::Thinking { text: text.clone() },
        }
    }
}

impl From<&ToolExecution> for ToolExecutionDto {
    fn from(tool_execution: &ToolExecution) -> Self {
        Self {
            tool_name: tool_execution.tool_name.clone(),
            description: tool_execution.description.clone(),
            status: ToolStatusDto::from(&tool_execution.status),
            elapsed_ms: duration_ms(tool_execution.started_at.elapsed()),
            output_preview: tool_execution.output_preview.clone(),
        }
    }
}

impl From<&ToolStatus> for ToolStatusDto {
    fn from(status: &ToolStatus) -> Self {
        match status {
            ToolStatus::Running => Self::Running,
            ToolStatus::Completed { duration } => Self::Completed {
                duration_ms: duration_ms(*duration),
            },
            ToolStatus::Failed { error } => Self::Failed {
                error: error.clone(),
            },
        }
    }
}

impl From<&UiToolResult> for UiToolResultDto {
    fn from(result: &UiToolResult) -> Self {
        Self {
            success: result.success,
            output: result.output.clone(),
            error: result.error.clone(),
            duration_ms: duration_ms(result.duration),
        }
    }
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_dto_is_serializable() {
        let event = AgentEvent::chunk("hello");
        let dto = AgentEventDto::from(event);
        let json = serde_json::to_string(&dto).expect("serialize AgentEventDto");
        assert!(json.contains("content_chunk"));
    }

    #[test]
    fn app_state_dto_is_serializable() {
        let mut state = AppState::default();
        state.start_streaming();
        state.append_streaming_chunk("hi");

        let dto = AppStateDto::from(&state);
        let json = serde_json::to_string(&dto).expect("serialize AppStateDto");
        assert!(json.contains("status_text"));
    }
}
