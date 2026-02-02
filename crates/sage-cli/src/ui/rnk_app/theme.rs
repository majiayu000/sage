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
#[allow(dead_code)]
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
        if let Some(bg) = cfgbg.rsplit(';').next().and_then(|s| s.parse::<u8>().ok()) {
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
    // All black text for maximum readability on light backgrounds
    Theme {
        text_primary: Color::Black,
        text_muted: Color::Black,
        text_subtle: Color::Black,

        accent_assistant: Color::Black,
        accent_user: Color::Black,
        accent_system: Color::Black,
        accent_primary: Color::Black,

        border: Color::Black,
        border_subtle: Color::Black,
        separator: Color::Black,

        surface: Color::Black,

        ok: Color::Black,
        warn: Color::Black,
        err: Color::Black,

        tool: Color::Black,
        tool_param: Color::Black,

        status_normal: Color::Black,
        status_bypass: Color::Black,
        status_plan: Color::Black,
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
