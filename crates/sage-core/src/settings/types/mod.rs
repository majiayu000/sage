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
pub use hooks::{HookDefinition, HookDefinitionType, HooksSettings};
pub use permissions::{ParsedPattern, SettingsPermissionBehavior, PermissionSettings};
pub use tools::ToolSettings;
