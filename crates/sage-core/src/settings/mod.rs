//! Multi-level settings system
//!
//! This module provides a hierarchical settings system for Sage Agent,
//! supporting multiple configuration levels with clear precedence.
//!
//! # Settings Hierarchy
//!
//! Settings are loaded and merged in the following order (lowest to highest priority):
//!
//! 1. **Built-in defaults** - Hardcoded default values
//! 2. **User settings** - `~/.config/sage/settings.json`
//! 3. **Project settings** - `.sage/settings.json` (committed to git)
//! 4. **Local settings** - `.sage/settings.local.json` (gitignored)
//! 5. **Environment variables** - `SAGE_*` environment variables
//! 6. **CLI arguments** - Command-line flags (highest priority)
//!
//! # File Format
//!
//! Settings files use JSON format with optional comments:
//!
//! ```json,ignore
//! {
//!   // This is a comment
//!   "permissions": {
//!     "allow": [
//!       "Read(src/**)",
//!       "Write(src/**)",
//!       "Bash(cargo *)"
//!     ],
//!     "deny": [
//!       "Read(.env)",
//!       "Bash(rm -rf *)"
//!     ],
//!     "default_behavior": "ask"
//!   },
//!   "tools": {
//!     "enabled": ["bash", "read", "write", "edit"],
//!     "disabled": []
//!   }
//! }
//! ```
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use sage_core::settings::{Settings, SettingsLoader};
//!
//! // Load settings from all sources
//! let loader = SettingsLoader::new();
//! let settings = loader.load()?;
//!
//! // Check permission settings
//! if settings.permissions.default_behavior == PermissionBehavior::Allow {
//!     println!("Auto-allowing all operations");
//! }
//!
//! // Check if a tool is enabled
//! if settings.tools.is_enabled("bash") {
//!     println!("Bash tool is enabled");
//! }
//! ```
//!
//! # Permission Patterns
//!
//! Permission patterns follow the format `ToolName(pattern)`:
//!
//! - `Read(src/**)` - Allow reading files in src/ and subdirectories
//! - `Write(src/*.rs)` - Allow writing Rust files in src/
//! - `Bash(npm *)` - Allow npm commands
//! - `Bash(cargo build)` - Allow specific cargo command
//!
//! Patterns support glob syntax (`*`, `**`, `?`) for flexible matching.

pub mod loader;
pub mod locations;
pub mod types;
pub mod validation;

pub use loader::{SettingsLoadInfo, SettingsLoader, SettingsSource};
pub use locations::SettingsLocations;
pub use types::{
    HookDefinition, HookDefinitionType, HooksSettings, ModelSettings, ParsedPattern,
    PermissionBehavior, PermissionSettings, Settings, ToolSettings, UiSettings, WorkspaceSettings,
};
pub use validation::{SettingsValidationResult, SettingsValidator};
