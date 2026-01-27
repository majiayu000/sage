//! Terminal-safe theming (light/dark + no-color)

use rnk::prelude::Color;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeKind {
    Dark,
    Light,
    NoColor,
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub kind: ThemeKind,

    pub text_primary: Color,
    pub text_muted: Color,
    pub text_dim: Color,

    pub accent_assistant: Color,
    pub accent_user: Color,
    pub accent_system: Color,

    pub border: Color,
    pub separator: Color,
    pub separator_active: Color,

    pub ok: Color,
    pub warn: Color,
    pub err: Color,

    pub tool: Color,
    pub tool_param: Color,
    pub tool_result: Color,

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

    ThemeKind::Dark
}

fn dark_theme() -> Theme {
    Theme {
        kind: ThemeKind::Dark,
        text_primary: Color::White,
        text_muted: Color::BrightBlack,
        text_dim: Color::BrightBlack,

        accent_assistant: Color::Cyan,
        accent_user: Color::Green,
        accent_system: Color::Magenta,

        border: Color::BrightBlack,
        separator: Color::BrightBlack,
        separator_active: Color::Cyan,

        ok: Color::Green,
        warn: Color::Yellow,
        err: Color::Red,

        tool: Color::Magenta,
        tool_param: Color::BrightBlack,
        tool_result: Color::White,

        status_normal: Color::Yellow,
        status_bypass: Color::Red,
        status_plan: Color::Cyan,
    }
}

fn light_theme() -> Theme {
    Theme {
        kind: ThemeKind::Light,
        text_primary: Color::Black,
        text_muted: Color::BrightBlack,
        text_dim: Color::BrightBlack,

        accent_assistant: Color::Blue,
        accent_user: Color::Green,
        accent_system: Color::Magenta,

        border: Color::BrightBlack,
        separator: Color::BrightBlack,
        separator_active: Color::Blue,

        ok: Color::Green,
        warn: Color::Yellow,
        err: Color::Red,

        tool: Color::Magenta,
        tool_param: Color::BrightBlack,
        tool_result: Color::Black,

        status_normal: Color::Yellow,
        status_bypass: Color::Red,
        status_plan: Color::Blue,
    }
}

fn no_color_theme() -> Theme {
    Theme {
        kind: ThemeKind::NoColor,
        text_primary: Color::White,
        text_muted: Color::White,
        text_dim: Color::White,

        accent_assistant: Color::White,
        accent_user: Color::White,
        accent_system: Color::White,

        border: Color::White,
        separator: Color::White,
        separator_active: Color::White,

        ok: Color::White,
        warn: Color::White,
        err: Color::White,

        tool: Color::White,
        tool_param: Color::White,
        tool_result: Color::White,

        status_normal: Color::White,
        status_bypass: Color::White,
        status_plan: Color::White,
    }
}
