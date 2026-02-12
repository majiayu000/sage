//! Settings type definitions
//!
//! This module defines the settings structure for Sage Agent,
//! supporting multi-level configuration (user, project, local).

mod base;
mod config;
mod hooks;
mod permissions;
mod tools;

pub use base::Settings;
pub use config::{ModelSettings, UiSettings, WorkspaceSettings};
pub use hooks::HooksSettings;
pub use permissions::PermissionSettings;

#[cfg(test)]
pub use permissions::SettingsPermissionBehavior;
pub use tools::ToolSettings;
