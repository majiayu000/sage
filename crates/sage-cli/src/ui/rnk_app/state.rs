//! UI state management for rnk app

use parking_lot::RwLock;
use sage_core::ui::bridge::state::SessionState;
use std::sync::Arc;

/// Permission mode for the UI
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionMode {
    Normal,
    Bypass,
    Plan,
}

impl PermissionMode {
    pub fn cycle(self) -> Self {
        match self {
            PermissionMode::Normal => PermissionMode::Bypass,
            PermissionMode::Bypass => PermissionMode::Plan,
            PermissionMode::Plan => PermissionMode::Normal,
        }
    }
}

/// UI state shared between render loop and background tasks
pub struct UiState {
    /// Current input text
    pub input_text: String,
    /// Permission mode
    pub permission_mode: PermissionMode,
    /// Whether agent is busy
    pub is_busy: bool,
    /// Status text
    pub status_text: String,
    /// Should quit
    pub should_quit: bool,
    /// Number of messages already printed
    pub printed_count: usize,
    /// Error already displayed (to avoid duplicate error messages)
    pub error_displayed: bool,
    /// Session info
    pub session: SessionState,
    /// Selected command suggestion index (0-based, default 0 for first match)
    pub suggestion_index: usize,
    /// Currently printed tool (to avoid duplicate tool start messages)
    pub current_tool_printed: Option<String>,
    /// Pending tool to print (waits for message to be printed first)
    pub pending_tool: Option<(String, String)>,
    /// Animation frame counter for spinner
    pub animation_frame: usize,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            input_text: String::new(),
            permission_mode: PermissionMode::Normal,
            is_busy: false,
            status_text: String::new(),
            should_quit: false,
            printed_count: 0,
            error_displayed: false,
            session: SessionState {
                session_id: None,
                model: "unknown".to_string(),
                provider: "unknown".to_string(),
                working_dir: std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                git_branch: None,
                step: 0,
                max_steps: None,
            },
            suggestion_index: 0,
            current_tool_printed: None,
            pending_tool: None,
            animation_frame: 0,
        }
    }
}

/// Shared state wrapper
pub type SharedState = Arc<RwLock<UiState>>;

/// Command from UI to executor
#[derive(Debug)]
pub enum UiCommand {
    Submit(String),
    Cancel,
    Quit,
}
