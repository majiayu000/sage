//! Color Definitions for Sage UI
//!
//! Based on Claude Code's color scheme with rnk Color types.

use rnk::prelude::Color;

/// Application theme colors
pub struct Colors;

impl Colors {
    // === Role Colors (Claude Code style) ===
    /// User input - Yellow
    pub const USER: Color = Color::Yellow;
    /// Assistant output - Bright White
    pub const ASSISTANT: Color = Color::BrightWhite;
    /// System messages - Gray
    pub const SYSTEM: Color = Color::Ansi256(245);

    // === Status Colors ===
    /// Success - Green
    pub const SUCCESS: Color = Color::Green;
    /// Error - Red
    pub const ERROR: Color = Color::Red;
    /// Warning - Yellow
    pub const WARNING: Color = Color::Yellow;
    /// Info - Blue
    pub const INFO: Color = Color::Blue;

    // === Specific Elements ===
    /// Tool execution - Magenta
    pub const TOOL: Color = Color::Magenta;
    /// Thinking state - Magenta
    pub const THINKING: Color = Color::Magenta;
    /// Brand color - Cyan
    pub const BRAND: Color = Color::Cyan;
    /// Git branch - Green
    pub const GIT: Color = Color::Green;

    // === Text ===
    /// Primary text - White
    pub const TEXT: Color = Color::White;
    /// Dimmed text - Gray
    pub const TEXT_DIM: Color = Color::Ansi256(245);
    /// Cursor - Bright White
    pub const CURSOR: Color = Color::BrightWhite;

    // === UI Elements ===
    /// Border - Dark Gray
    pub const BORDER: Color = Color::Ansi256(238);
    /// Input border - Gray
    pub const INPUT_BORDER: Color = Color::Ansi256(245);
}

/// Theme configuration
#[derive(Clone, Debug)]
pub struct ThemeConfig {
    pub name: String,
    pub user_color: Color,
    pub assistant_color: Color,
    pub tool_color: Color,
    pub error_color: Color,
    pub success_color: Color,
}

impl ThemeConfig {
    /// Claude Code style theme (default)
    pub fn claude_code() -> Self {
        Self {
            name: "Claude Code".to_string(),
            user_color: Color::Yellow,
            assistant_color: Color::BrightWhite,
            tool_color: Color::Magenta,
            error_color: Color::Red,
            success_color: Color::Green,
        }
    }

    /// Dark theme
    pub fn dark() -> Self {
        Self::claude_code()
    }

    /// Light theme
    pub fn light() -> Self {
        Self {
            name: "Light".to_string(),
            user_color: Color::Blue,
            assistant_color: Color::Black,
            tool_color: Color::Magenta,
            error_color: Color::Red,
            success_color: Color::Green,
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self::claude_code()
    }
}
