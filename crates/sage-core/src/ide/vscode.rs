//! VS Code integration

use super::{IdeIntegration, IdeType};
use crate::error::{SageError, SageResult};
use std::process::Command;

/// VS Code integration
pub struct VsCodeIntegration {
    ide_type: IdeType,
    cli_path: Option<String>,
}

impl VsCodeIntegration {
    /// Create new VS Code integration
    pub fn new(ide_type: IdeType) -> Self {
        let cli_path = Self::find_cli_path(ide_type);
        Self { ide_type, cli_path }
    }

    /// Create for standard VS Code
    pub fn vscode() -> Self {
        Self::new(IdeType::VsCode)
    }

    /// Create for VS Code Insiders
    pub fn vscode_insiders() -> Self {
        Self::new(IdeType::VsCodeInsiders)
    }

    /// Create for Cursor
    pub fn cursor() -> Self {
        Self::new(IdeType::Cursor)
    }

    /// Find CLI path for the IDE
    fn find_cli_path(ide_type: IdeType) -> Option<String> {
        let cli_name = match ide_type {
            IdeType::VsCode => "code",
            IdeType::VsCodeInsiders => "code-insiders",
            IdeType::Cursor => "cursor",
            _ => return None,
        };

        #[cfg(target_os = "macos")]
        {
            let app_name = match ide_type {
                IdeType::VsCode => "Visual Studio Code",
                IdeType::VsCodeInsiders => "Visual Studio Code - Insiders",
                IdeType::Cursor => "Cursor",
                _ => return None,
            };

            let paths = [
                format!("/usr/local/bin/{}", cli_name),
                format!(
                    "/Applications/{}.app/Contents/Resources/app/bin/{}",
                    app_name, cli_name
                ),
            ];

            for path in paths {
                if std::path::Path::new(&path).exists() {
                    return Some(path);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = Command::new("which").arg(cli_name).output() {
                if output.status.success() {
                    return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            let paths = [
                format!(r"C:\Program Files\Microsoft VS Code\bin\{}.cmd", cli_name),
                format!(
                    r"C:\Users\{}\AppData\Local\Programs\Microsoft VS Code\bin\{}.cmd",
                    std::env::var("USERNAME").unwrap_or_default(),
                    cli_name
                ),
            ];

            for path in paths {
                if std::path::Path::new(&path).exists() {
                    return Some(path);
                }
            }
        }

        None
    }
}

impl IdeIntegration for VsCodeIntegration {
    fn ide_type(&self) -> IdeType {
        self.ide_type
    }

    fn is_running(&self) -> bool {
        let process_name = match self.ide_type {
            IdeType::VsCode => "Code",
            IdeType::VsCodeInsiders => "Code - Insiders",
            IdeType::Cursor => "Cursor",
            _ => return false,
        };

        #[cfg(target_os = "macos")]
        {
            Command::new("pgrep")
                .args(["-x", process_name])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }

        #[cfg(target_os = "linux")]
        {
            let process_name = match self.ide_type {
                IdeType::VsCode => "code",
                IdeType::VsCodeInsiders => "code-insiders",
                IdeType::Cursor => "cursor",
                _ => return false,
            };

            Command::new("pgrep")
                .args(["-x", process_name])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("tasklist")
                .args(["/FI", &format!("IMAGENAME eq {}.exe", process_name)])
                .output()
                .map(|o| {
                    let output = String::from_utf8_lossy(&o.stdout);
                    output.contains(process_name)
                })
                .unwrap_or(false)
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            false
        }
    }

    fn current_file(&self) -> Option<String> {
        // VS Code doesn't expose current file easily
        // Would need extension support
        None
    }

    fn current_selection(&self) -> Option<String> {
        // Would need extension support
        None
    }

    fn open_file(&self, path: &str, line: Option<u32>) -> SageResult<()> {
        let cli_path = self.cli_path.as_ref().ok_or_else(|| {
            SageError::invalid_input(format!(
                "{} CLI not found. Please install the 'code' command in PATH.",
                self.ide_type.display_name()
            ))
        })?;

        let mut cmd = Command::new(cli_path);

        if let Some(line_num) = line {
            cmd.arg("--goto").arg(format!("{}:{}", path, line_num));
        } else {
            cmd.arg(path);
        }

        cmd.spawn().map_err(|e| {
            SageError::invalid_input(format!("Failed to open file in VS Code: {}", e))
        })?;

        Ok(())
    }

    fn notify(&self, message: &str) -> SageResult<()> {
        // VS Code doesn't have a simple notification API from CLI
        tracing::info!("VS Code notification: {}", message);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vscode_integration_creation() {
        let vscode = VsCodeIntegration::vscode();
        assert_eq!(vscode.ide_type(), IdeType::VsCode);

        let insiders = VsCodeIntegration::vscode_insiders();
        assert_eq!(insiders.ide_type(), IdeType::VsCodeInsiders);

        let cursor = VsCodeIntegration::cursor();
        assert_eq!(cursor.ide_type(), IdeType::Cursor);
    }
}
