//! JetBrains IDE integration

use super::{IdeIntegration, IdeType};
use crate::error::{SageError, SageResult};
use std::process::Command;

/// JetBrains IDE integration
pub struct JetBrainsIntegration {
    ide_type: IdeType,
    cli_path: Option<String>,
}

impl JetBrainsIntegration {
    /// Create new JetBrains integration
    pub fn new(ide_type: IdeType) -> Self {
        let cli_path = Self::find_cli_path(ide_type);
        Self { ide_type, cli_path }
    }

    /// Find CLI launcher path for the IDE
    fn find_cli_path(ide_type: IdeType) -> Option<String> {
        let cli_name = match ide_type {
            IdeType::IntelliJ => "idea",
            IdeType::PyCharm => "pycharm",
            IdeType::WebStorm => "webstorm",
            IdeType::PhpStorm => "phpstorm",
            IdeType::RubyMine => "rubymine",
            IdeType::CLion => "clion",
            IdeType::GoLand => "goland",
            IdeType::Rider => "rider",
            IdeType::DataGrip => "datagrip",
            IdeType::AndroidStudio => "studio",
            IdeType::Fleet => "fleet",
            _ => return None,
        };

        // Check common locations
        #[cfg(target_os = "macos")]
        {
            let paths = [
                format!("/usr/local/bin/{}", cli_name),
                format!(
                    "/Applications/{}.app/Contents/MacOS/{}",
                    ide_type.display_name(),
                    cli_name
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
            // Check if in PATH
            if let Ok(output) = Command::new("which").arg(cli_name).output() {
                if output.status.success() {
                    return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
                }
            }
        }

        None
    }
}

impl IdeIntegration for JetBrainsIntegration {
    fn ide_type(&self) -> IdeType {
        self.ide_type
    }

    fn is_running(&self) -> bool {
        // Check if IDE process is running
        let process_name = match self.ide_type {
            IdeType::IntelliJ => "idea",
            IdeType::PyCharm => "pycharm",
            IdeType::WebStorm => "webstorm",
            IdeType::PhpStorm => "phpstorm",
            IdeType::RubyMine => "rubymine",
            IdeType::CLion => "clion",
            IdeType::GoLand => "goland",
            IdeType::Rider => "rider",
            IdeType::DataGrip => "datagrip",
            IdeType::AndroidStudio => "studio",
            IdeType::Fleet => "fleet",
            _ => return false,
        };

        #[cfg(unix)]
        {
            Command::new("pgrep")
                .args(["-x", process_name])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }

        #[cfg(windows)]
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

        #[cfg(not(any(unix, windows)))]
        {
            false
        }
    }

    fn current_file(&self) -> Option<String> {
        // JetBrains IDEs don't have a simple way to get current file
        // Would need to use IDE's REST API or plugin
        None
    }

    fn current_selection(&self) -> Option<String> {
        // Would need IDE plugin support
        None
    }

    fn open_file(&self, path: &str, line: Option<u32>) -> SageResult<()> {
        let cli_path = self.cli_path.as_ref().ok_or_else(|| {
            SageError::invalid_input(format!(
                "{} CLI not found. Please install the command-line launcher.",
                self.ide_type.display_name()
            ))
        })?;

        let mut cmd = Command::new(cli_path);

        if let Some(line_num) = line {
            cmd.arg("--line").arg(line_num.to_string());
        }

        cmd.arg(path);

        cmd.spawn().map_err(|e| {
            SageError::invalid_input(format!("Failed to open file in IDE: {}", e))
        })?;

        Ok(())
    }

    fn notify(&self, message: &str) -> SageResult<()> {
        // JetBrains IDEs don't have a simple notification API
        // Would need to use IDE's REST API or plugin
        tracing::info!("IDE notification: {}", message);
        Ok(())
    }
}
