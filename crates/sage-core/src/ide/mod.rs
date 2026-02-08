//! IDE integration module
//!
//! Provides integration with various IDEs:
//! - JetBrains IDEs (IntelliJ, PyCharm, WebStorm, etc.)
//! - VS Code
//! - Other editors

mod detection;
mod jetbrains;
mod vscode;

pub use detection::{IdeDetector, DetectedIde, IdeType};
pub use jetbrains::JetBrainsIntegration;
pub use vscode::VsCodeIntegration;

use crate::error::SageResult;

/// IDE integration trait
pub trait IdeIntegration: Send + Sync {
    /// Get IDE type
    fn ide_type(&self) -> IdeType;

    /// Check if IDE is running
    fn is_running(&self) -> bool;

    /// Get current file being edited
    fn current_file(&self) -> Option<String>;

    /// Get current selection
    fn current_selection(&self) -> Option<String>;

    /// Open a file in the IDE
    fn open_file(&self, path: &str, line: Option<u32>) -> SageResult<()>;

    /// Show a notification in the IDE
    fn notify(&self, message: &str) -> SageResult<()>;
}
