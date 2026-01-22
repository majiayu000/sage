//! UI state management for rnk app

use parking_lot::RwLock;
use rnk::prelude::Color;
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

    pub fn display_text(self) -> &'static str {
        match self {
            PermissionMode::Normal => "permissions required",
            PermissionMode::Bypass => "bypass permissions",
            PermissionMode::Plan => "plan mode",
        }
    }

    pub fn color(self) -> Color {
        match self {
            PermissionMode::Normal => Color::Yellow,
            PermissionMode::Bypass => Color::Red,
            PermissionMode::Plan => Color::Cyan,
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
    /// Header already printed
    pub header_printed: bool,
    /// Error already displayed (to avoid duplicate error messages)
    pub error_displayed: bool,
    /// Session info
    pub session: SessionState,
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
            header_printed: false,
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
