//! Terminal-safe theming (light/dark + no-color)
//!
//! Color palettes based on Catppuccin (https://catppuccin.com/)
//! - Dark theme: Catppuccin Mocha
//! - Light theme: Catppuccin Latte

use rnk::prelude::Color;
use std::sync::OnceLock;

enum ThemeKind {
    Dark,
    Light,
    NoColor,
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub text_primary: Color,
    pub text_muted: Color,
    pub text_subtle: Color,

    pub accent_assistant: Color,
    pub accent_user: Color,
    pub accent_system: Color,
    pub accent_primary: Color,

    pub border: Color,
    pub border_subtle: Color,
    pub separator: Color,

    pub surface: Color,

    pub ok: Color,
    pub warn: Color,
    pub err: Color,

    pub tool: Color,
    pub tool_param: Color,

    pub status_normal: Color,
    pub status_bypass: Color,
    pub status_plan: Color,
}

static THEME: OnceLock<Theme> = OnceLock::new();

pub fn current_theme() -> &'static Theme {
    THEME.get_or_init(|| match detect_theme_kind() {
        ThemeKind::Light => light_theme(),
        ThemeKind::NoColor => no_color_theme(),
        ThemeKind::Dark => dark_theme(),
    })
}

fn detect_theme_kind() -> ThemeKind {
    if std::env::var_os("NO_COLOR").is_some() {
        return ThemeKind::NoColor;
    }

    if let Ok(v) = std::env::var("SAGE_THEME") {
        match v.to_ascii_lowercase().as_str() {
            "light" => return ThemeKind::Light,
            "dark" => return ThemeKind::Dark,
            "none" | "no" | "off" => return ThemeKind::NoColor,
            _ => {}
        }
    }

    // Heuristic: COLORFGBG=fg;bg where bg>=7 often means light bg
    if let Ok(cfgbg) = std::env::var("COLORFGBG") {
        if let Some(bg) = cfgbg.split(';').last().and_then(|s| s.parse::<u8>().ok()) {
            if bg >= 7 {
                return ThemeKind::Light;
            }
        }
    }

    ThemeKind::Light
}

fn dark_theme() -> Theme {
    // Catppuccin Mocha palette
    Theme {
        text_primary: Color::Rgb(205, 214, 244),  // #cdd6f4
        text_muted: Color::Rgb(108, 112, 134),    // #6c7086
        text_subtle: Color::Rgb(88, 91, 112),     // #585b70

        accent_assistant: Color::Rgb(137, 180, 250), // #89b4fa (blue)
        accent_user: Color::Rgb(166, 227, 161),      // #a6e3a1 (green)
        accent_system: Color::Rgb(203, 166, 247),    // #cba6f7 (mauve)
        accent_primary: Color::Rgb(137, 180, 250),   // #89b4fa (blue)

        border: Color::Rgb(88, 91, 112),          // #585b70
        border_subtle: Color::Rgb(69, 71, 90),    // #45475a
        separator: Color::Rgb(69, 71, 90),        // #45475a

        surface: Color::Rgb(49, 50, 68),          // #313244

        ok: Color::Rgb(166, 227, 161),            // #a6e3a1 (green)
        warn: Color::Rgb(249, 226, 175),          // #f9e2af (yellow)
        err: Color::Rgb(243, 139, 168),           // #f38ba8 (red)

        tool: Color::Rgb(203, 166, 247),          // #cba6f7 (mauve)
        tool_param: Color::Rgb(166, 173, 200),    // #a6adc8 (subtext0)

        status_normal: Color::Rgb(249, 226, 175), // #f9e2af (yellow)
        status_bypass: Color::Rgb(243, 139, 168), // #f38ba8 (red)
        status_plan: Color::Rgb(137, 180, 250),   // #89b4fa (blue)
    }
}

fn light_theme() -> Theme {
    // Catppuccin Latte palette
    Theme {
        text_primary: Color::Rgb(76, 79, 105),    // #4c4f69
        text_muted: Color::Rgb(108, 111, 133),    // #6c6f85
        text_subtle: Color::Rgb(140, 143, 161),   // #8c8fa1

        accent_assistant: Color::Rgb(30, 102, 245),  // #1e66f5 (blue)
        accent_user: Color::Rgb(64, 160, 43),        // #40a02b (green)
        accent_system: Color::Rgb(136, 57, 239),     // #8839ef (mauve)
        accent_primary: Color::Rgb(30, 102, 245),    // #1e66f5 (blue)

        border: Color::Rgb(140, 143, 161),        // #8c8fa1
        border_subtle: Color::Rgb(172, 176, 190), // #acb0be
        separator: Color::Rgb(188, 192, 204),     // #bcc0cc

        surface: Color::Rgb(230, 233, 239),       // #e6e9ef

        ok: Color::Rgb(64, 160, 43),              // #40a02b (green)
        warn: Color::Rgb(223, 142, 29),           // #df8e1d (yellow)
        err: Color::Rgb(210, 15, 57),             // #d20f39 (red)

        tool: Color::Rgb(136, 57, 239),           // #8839ef (mauve)
        tool_param: Color::Rgb(92, 95, 119),      // #5c5f77 (subtext1)

        status_normal: Color::Rgb(223, 142, 29),  // #df8e1d (yellow)
        status_bypass: Color::Rgb(210, 15, 57),   // #d20f39 (red)
        status_plan: Color::Rgb(30, 102, 245),    // #1e66f5 (blue)
    }
}

fn no_color_theme() -> Theme {
    Theme {
        text_primary: Color::White,
        text_muted: Color::White,
        text_subtle: Color::White,

        accent_assistant: Color::White,
        accent_user: Color::White,
        accent_system: Color::White,
        accent_primary: Color::White,

        border: Color::White,
        border_subtle: Color::White,
        separator: Color::White,

        surface: Color::White,

        ok: Color::White,
        warn: Color::White,
        err: Color::White,

        tool: Color::White,
        tool_param: Color::White,

        status_normal: Color::White,
        status_bypass: Color::White,
        status_plan: Color::White,
    }
}
