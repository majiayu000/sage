//! Icon system with Nerd Font and ASCII fallbacks
//!
//! Icons from Nerd Fonts: https://www.nerdfonts.com/cheat-sheet
//!
//! Control icon mode via:
//! - Environment variable: `SAGE_NERD_FONTS=false` to disable Nerd Fonts
//! - Or programmatically: `Icons::set_nerd_fonts(false)`

use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag for Nerd Font mode (default: true)
static USE_NERD_FONTS: AtomicBool = AtomicBool::new(true);

/// Initialize icon mode from environment variable
pub fn init_from_env() {
    if let Ok(val) = std::env::var("SAGE_NERD_FONTS") {
        let enabled = !matches!(val.to_lowercase().as_str(), "false" | "0" | "no" | "off");
        USE_NERD_FONTS.store(enabled, Ordering::SeqCst);
    }
}

/// Check if Nerd Fonts are enabled
pub fn is_nerd_fonts_enabled() -> bool {
    USE_NERD_FONTS.load(Ordering::SeqCst)
}

/// Set Nerd Font mode programmatically
pub fn set_nerd_fonts(enabled: bool) {
    USE_NERD_FONTS.store(enabled, Ordering::SeqCst);
}

/// Icon constants with Nerd Font and ASCII variants
pub struct Icons;

impl Icons {
    // === Claude Code Style Message Indicators ===
    /// Filled circle for messages (Claude Code style)
    pub const MESSAGE: &'static str = "⏺";
    /// Result/output indicator (corner bracket)
    pub const RESULT: &'static str = "⎿";
    /// Thinking/cogitating indicator
    pub const COGITATE: &'static str = "✻";
    /// User prompt indicator
    pub const PROMPT: &'static str = "❯";

    // === Application ===
    pub const SAGE: &'static str = "󰚩"; // nf-md-robot
    pub const SAGE_ASCII: &'static str = "[S]";

    // === Status ===
    pub const SUCCESS: &'static str = ""; // nf-fa-check
    pub const SUCCESS_ASCII: &'static str = "✓";

    pub const ERROR: &'static str = ""; // nf-fa-times
    pub const ERROR_ASCII: &'static str = "✗";

    pub const WARNING: &'static str = ""; // nf-fa-warning
    pub const WARNING_ASCII: &'static str = "⚠";

    pub const INFO: &'static str = ""; // nf-fa-info_circle
    pub const INFO_ASCII: &'static str = "ℹ";

    pub const THINKING: &'static str = "󰔟"; // nf-md-timer_sand
    pub const THINKING_ASCII: &'static str = "⏳";

    pub const RUNNING: &'static str = ""; // nf-fa-spinner
    pub const RUNNING_ASCII: &'static str = "▶";

    // === Tools ===
    pub const TOOL: &'static str = ""; // nf-fa-wrench
    pub const TOOL_ASCII: &'static str = "🔧";

    pub const TERMINAL: &'static str = ""; // nf-oct-terminal
    pub const TERMINAL_ASCII: &'static str = "$";

    pub const FILE: &'static str = ""; // nf-oct-file
    pub const FILE_ASCII: &'static str = "📄";

    pub const EDIT: &'static str = ""; // nf-fa-pencil
    pub const EDIT_ASCII: &'static str = "✎";

    pub const SEARCH: &'static str = ""; // nf-fa-search
    pub const SEARCH_ASCII: &'static str = "🔍";

    pub const CODE: &'static str = ""; // nf-fa-code
    pub const CODE_ASCII: &'static str = "<>";

    pub const WEB: &'static str = "󰖟"; // nf-md-web
    pub const WEB_ASCII: &'static str = "🌐";

    pub const TASK: &'static str = ""; // nf-fa-tasks
    pub const TASK_ASCII: &'static str = "☐";

    pub const FOLDER: &'static str = ""; // nf-oct-file_directory
    pub const FOLDER_ASCII: &'static str = "📁";

    // === Dynamic getters ===

    /// Get message indicator (filled circle)
    pub fn message() -> &'static str {
        Self::MESSAGE
    }

    /// Get result indicator (corner bracket)
    pub fn result() -> &'static str {
        Self::RESULT
    }

    /// Get cogitate/thinking indicator
    pub fn cogitate() -> &'static str {
        Self::COGITATE
    }

    /// Get user prompt indicator
    pub fn prompt() -> &'static str {
        Self::PROMPT
    }

    /// Get sage icon based on current mode
    pub fn sage() -> &'static str {
        if is_nerd_fonts_enabled() {
            Self::SAGE
        } else {
            Self::SAGE_ASCII
        }
    }

    /// Get success icon based on current mode
    pub fn success() -> &'static str {
        if is_nerd_fonts_enabled() {
            Self::SUCCESS
        } else {
            Self::SUCCESS_ASCII
        }
    }

    /// Get error icon based on current mode
    pub fn error() -> &'static str {
        if is_nerd_fonts_enabled() {
            Self::ERROR
        } else {
            Self::ERROR_ASCII
        }
    }

    /// Get warning icon based on current mode
    pub fn warning() -> &'static str {
        if is_nerd_fonts_enabled() {
            Self::WARNING
        } else {
            Self::WARNING_ASCII
        }
    }

    /// Get info icon based on current mode
    pub fn info() -> &'static str {
        if is_nerd_fonts_enabled() {
            Self::INFO
        } else {
            Self::INFO_ASCII
        }
    }

    /// Get thinking icon based on current mode
    pub fn thinking() -> &'static str {
        if is_nerd_fonts_enabled() {
            Self::THINKING
        } else {
            Self::THINKING_ASCII
        }
    }

    /// Get running icon based on current mode
    pub fn running() -> &'static str {
        if is_nerd_fonts_enabled() {
            Self::RUNNING
        } else {
            Self::RUNNING_ASCII
        }
    }

    /// Get tool icon based on current mode
    pub fn tool() -> &'static str {
        if is_nerd_fonts_enabled() {
            Self::TOOL
        } else {
            Self::TOOL_ASCII
        }
    }

    /// Get terminal icon based on current mode
    pub fn terminal() -> &'static str {
        if is_nerd_fonts_enabled() {
            Self::TERMINAL
        } else {
            Self::TERMINAL_ASCII
        }
    }

    /// Get file icon based on current mode
    pub fn file() -> &'static str {
        if is_nerd_fonts_enabled() {
            Self::FILE
        } else {
            Self::FILE_ASCII
        }
    }

    /// Get edit icon based on current mode
    pub fn edit() -> &'static str {
        if is_nerd_fonts_enabled() {
            Self::EDIT
        } else {
            Self::EDIT_ASCII
        }
    }

    /// Get search icon based on current mode
    pub fn search() -> &'static str {
        if is_nerd_fonts_enabled() {
            Self::SEARCH
        } else {
            Self::SEARCH_ASCII
        }
    }

    /// Get code icon based on current mode
    pub fn code() -> &'static str {
        if is_nerd_fonts_enabled() {
            Self::CODE
        } else {
            Self::CODE_ASCII
        }
    }

    /// Get web icon based on current mode
    pub fn web() -> &'static str {
        if is_nerd_fonts_enabled() {
            Self::WEB
        } else {
            Self::WEB_ASCII
        }
    }

    /// Get task icon based on current mode
    pub fn task() -> &'static str {
        if is_nerd_fonts_enabled() {
            Self::TASK
        } else {
            Self::TASK_ASCII
        }
    }

    /// Get folder icon based on current mode
    pub fn folder() -> &'static str {
        if is_nerd_fonts_enabled() {
            Self::FOLDER
        } else {
            Self::FOLDER_ASCII
        }
    }

    /// Get icon for a specific tool by name
    pub fn for_tool(tool_name: &str) -> &'static str {
        match tool_name.to_lowercase().as_str() {
            "bash" | "shell" | "execute" => Self::terminal(),
            "read" | "cat" => Self::file(),
            "write" | "edit" => Self::edit(),
            "grep" | "search" => Self::search(),
            "glob" | "find" => Self::folder(),
            "lsp" | "code" => Self::code(),
            "web_fetch" | "web_search" => Self::web(),
            "task" | "todo_write" => Self::task(),
            _ => Self::tool(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_nerd_fonts_enabled() {
        // Reset to default state
        set_nerd_fonts(true);
        assert!(is_nerd_fonts_enabled());
        assert_eq!(Icons::sage(), Icons::SAGE);
    }

    #[test]
    fn test_ascii_mode() {
        set_nerd_fonts(false);
        assert!(!is_nerd_fonts_enabled());
        assert_eq!(Icons::sage(), Icons::SAGE_ASCII);
        // Reset
        set_nerd_fonts(true);
    }

    #[test]
    fn test_for_tool() {
        set_nerd_fonts(true);
        assert_eq!(Icons::for_tool("bash"), Icons::TERMINAL);
        assert_eq!(Icons::for_tool("read"), Icons::FILE);
        assert_eq!(Icons::for_tool("unknown"), Icons::TOOL);
        set_nerd_fonts(false);
        assert_eq!(Icons::for_tool("bash"), Icons::TERMINAL_ASCII);
        // Reset
        set_nerd_fonts(true);
    }
}
