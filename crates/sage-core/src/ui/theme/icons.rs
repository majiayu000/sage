//! Icon System for Sage UI
//!
//! Provides Unicode and Nerd Font icons with ASCII fallback.
//! Migrated from the original icons.rs with rnk integration.

use std::sync::atomic::{AtomicBool, Ordering};

static USE_NERD_FONTS: AtomicBool = AtomicBool::new(true);

/// Application icons
pub struct Icons;

impl Icons {
    // === Message Indicators (Claude Code style) ===
    /// Message indicator - filled circle
    pub fn message() -> &'static str {
        "●"
    }

    /// Result indicator - corner bracket
    pub fn result() -> &'static str {
        "⎿"
    }

    /// Cogitate/thinking indicator
    pub fn cogitate() -> &'static str {
        "✻"
    }

    /// User prompt indicator
    pub fn prompt() -> &'static str {
        "❯"
    }

    // === Status Icons ===
    /// Success checkmark
    pub fn success() -> &'static str {
        if USE_NERD_FONTS.load(Ordering::SeqCst) {
            "\u{f00c}" // nf-fa-check
        } else {
            "✓"
        }
    }

    /// Error cross
    pub fn error() -> &'static str {
        if USE_NERD_FONTS.load(Ordering::SeqCst) {
            "\u{f00d}" // nf-fa-times
        } else {
            "✗"
        }
    }

    /// Warning triangle
    pub fn warning() -> &'static str {
        if USE_NERD_FONTS.load(Ordering::SeqCst) {
            "\u{f071}" // nf-fa-warning
        } else {
            "⚠"
        }
    }

    /// Info circle
    pub fn info() -> &'static str {
        if USE_NERD_FONTS.load(Ordering::SeqCst) {
            "\u{f05a}" // nf-fa-info_circle
        } else {
            "ℹ"
        }
    }

    /// Running/spinner
    pub fn running() -> &'static str {
        if USE_NERD_FONTS.load(Ordering::SeqCst) {
            "\u{f110}" // nf-fa-spinner
        } else {
            "▶"
        }
    }

    // === Application Icons ===
    /// Sage brand icon
    pub fn sage() -> &'static str {
        if USE_NERD_FONTS.load(Ordering::SeqCst) {
            "\u{f06a9}" // nf-md-robot (󰚩)
        } else {
            "[S]"
        }
    }

    /// Git branch
    pub fn git_branch() -> &'static str {
        if USE_NERD_FONTS.load(Ordering::SeqCst) {
            "\u{e725}" // nf-oct-git_branch
        } else {
            "⎇"
        }
    }

    /// Folder
    pub fn folder() -> &'static str {
        if USE_NERD_FONTS.load(Ordering::SeqCst) {
            "\u{f07b}" // nf-fa-folder
        } else {
            "📁"
        }
    }

    /// File
    pub fn file() -> &'static str {
        if USE_NERD_FONTS.load(Ordering::SeqCst) {
            "\u{f15b}" // nf-fa-file
        } else {
            "📄"
        }
    }

    /// Terminal/bash
    pub fn terminal() -> &'static str {
        if USE_NERD_FONTS.load(Ordering::SeqCst) {
            "\u{f120}" // nf-fa-terminal
        } else {
            ">"
        }
    }

    /// Code/edit
    pub fn code() -> &'static str {
        if USE_NERD_FONTS.load(Ordering::SeqCst) {
            "\u{f121}" // nf-fa-code
        } else {
            "<>"
        }
    }

    /// Search/grep
    pub fn search() -> &'static str {
        if USE_NERD_FONTS.load(Ordering::SeqCst) {
            "\u{f002}" // nf-fa-search
        } else {
            "🔍"
        }
    }

    // === Tool Icons ===
    /// Get icon for a specific tool
    pub fn for_tool(tool_name: &str) -> &'static str {
        match tool_name.to_lowercase().as_str() {
            "bash" => Self::terminal(),
            "edit" | "write" => Self::code(),
            "read" => Self::file(),
            "glob" | "grep" => Self::search(),
            "task" => Self::running(),
            _ => Self::message(),
        }
    }

    // === Spinner Frames ===
    /// Get spinner frames for animation
    pub fn spinner_frames() -> &'static [&'static str] {
        // Use Unicode braille patterns for spinner (works everywhere)
        &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
    }

    // === Control Functions ===
    /// Initialize from environment variable
    pub fn init_from_env() {
        if let Ok(val) = std::env::var("SAGE_NERD_FONTS") {
            let enabled = !matches!(val.to_lowercase().as_str(), "false" | "0" | "no" | "off");
            USE_NERD_FONTS.store(enabled, Ordering::SeqCst);
        }
    }

    /// Set Nerd Fonts enabled/disabled
    pub fn set_nerd_fonts(enabled: bool) {
        USE_NERD_FONTS.store(enabled, Ordering::SeqCst);
    }

    /// Check if Nerd Fonts are enabled
    pub fn is_nerd_fonts_enabled() -> bool {
        USE_NERD_FONTS.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icons() {
        // Basic icons should always work
        assert!(!Icons::message().is_empty());
        assert!(!Icons::success().is_empty());
        assert!(!Icons::error().is_empty());
    }

    #[test]
    fn test_nerd_fonts_toggle() {
        let original = Icons::is_nerd_fonts_enabled();

        Icons::set_nerd_fonts(false);
        assert!(!Icons::is_nerd_fonts_enabled());
        assert_eq!(Icons::success(), "✓");

        Icons::set_nerd_fonts(true);
        assert!(Icons::is_nerd_fonts_enabled());
        assert_eq!(Icons::success(), "\u{f00c}");

        Icons::set_nerd_fonts(original);
    }

    #[test]
    fn test_tool_icons() {
        assert_eq!(Icons::for_tool("bash"), Icons::terminal());
        assert_eq!(Icons::for_tool("BASH"), Icons::terminal());
        assert_eq!(Icons::for_tool("edit"), Icons::code());
    }
}
