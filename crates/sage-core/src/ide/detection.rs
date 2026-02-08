//! IDE detection utilities

use std::path::Path;
use std::process::Command;

/// Detected IDE information
#[derive(Debug, Clone)]
pub struct DetectedIde {
    /// IDE type
    pub ide_type: IdeType,
    /// IDE name
    pub name: String,
    /// IDE version (if available)
    pub version: Option<String>,
    /// Path to IDE executable
    pub path: Option<String>,
    /// Whether the IDE is currently running
    pub running: bool,
}

/// IDE types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdeType {
    /// VS Code
    VsCode,
    /// VS Code Insiders
    VsCodeInsiders,
    /// Cursor (VS Code fork)
    Cursor,
    /// IntelliJ IDEA
    IntelliJ,
    /// PyCharm
    PyCharm,
    /// WebStorm
    WebStorm,
    /// PhpStorm
    PhpStorm,
    /// RubyMine
    RubyMine,
    /// CLion
    CLion,
    /// GoLand
    GoLand,
    /// Rider
    Rider,
    /// DataGrip
    DataGrip,
    /// Android Studio
    AndroidStudio,
    /// Fleet
    Fleet,
    /// Vim/Neovim
    Vim,
    /// Emacs
    Emacs,
    /// Sublime Text
    SublimeText,
    /// Atom
    Atom,
    /// Unknown
    Unknown,
}

impl IdeType {
    /// Get display name
    pub fn display_name(&self) -> &str {
        match self {
            IdeType::VsCode => "Visual Studio Code",
            IdeType::VsCodeInsiders => "VS Code Insiders",
            IdeType::Cursor => "Cursor",
            IdeType::IntelliJ => "IntelliJ IDEA",
            IdeType::PyCharm => "PyCharm",
            IdeType::WebStorm => "WebStorm",
            IdeType::PhpStorm => "PhpStorm",
            IdeType::RubyMine => "RubyMine",
            IdeType::CLion => "CLion",
            IdeType::GoLand => "GoLand",
            IdeType::Rider => "Rider",
            IdeType::DataGrip => "DataGrip",
            IdeType::AndroidStudio => "Android Studio",
            IdeType::Fleet => "Fleet",
            IdeType::Vim => "Vim/Neovim",
            IdeType::Emacs => "Emacs",
            IdeType::SublimeText => "Sublime Text",
            IdeType::Atom => "Atom",
            IdeType::Unknown => "Unknown",
        }
    }

    /// Check if this is a JetBrains IDE
    pub fn is_jetbrains(&self) -> bool {
        matches!(
            self,
            IdeType::IntelliJ
                | IdeType::PyCharm
                | IdeType::WebStorm
                | IdeType::PhpStorm
                | IdeType::RubyMine
                | IdeType::CLion
                | IdeType::GoLand
                | IdeType::Rider
                | IdeType::DataGrip
                | IdeType::AndroidStudio
                | IdeType::Fleet
        )
    }

    /// Check if this is a VS Code variant
    pub fn is_vscode(&self) -> bool {
        matches!(
            self,
            IdeType::VsCode | IdeType::VsCodeInsiders | IdeType::Cursor
        )
    }
}

/// IDE detector
pub struct IdeDetector;

impl IdeDetector {
    /// Detect all running IDEs
    pub fn detect_running() -> Vec<DetectedIde> {
        let mut ides = Vec::new();

        // Detect VS Code variants
        if let Some(ide) = Self::detect_vscode() {
            ides.push(ide);
        }

        // Detect JetBrains IDEs
        ides.extend(Self::detect_jetbrains());

        // Detect other editors
        if let Some(ide) = Self::detect_vim() {
            ides.push(ide);
        }

        ides
    }

    /// Detect VS Code
    fn detect_vscode() -> Option<DetectedIde> {
        // Check for VS Code process
        #[cfg(target_os = "macos")]
        let processes = ["Code", "Code - Insiders", "Cursor"];

        #[cfg(target_os = "linux")]
        let processes = ["code", "code-insiders", "cursor"];

        #[cfg(target_os = "windows")]
        let processes = ["Code.exe", "Code - Insiders.exe", "Cursor.exe"];

        for (i, process) in processes.iter().enumerate() {
            if Self::is_process_running(process) {
                let ide_type = match i {
                    0 => IdeType::VsCode,
                    1 => IdeType::VsCodeInsiders,
                    2 => IdeType::Cursor,
                    _ => IdeType::VsCode,
                };

                return Some(DetectedIde {
                    ide_type,
                    name: ide_type.display_name().to_string(),
                    version: None,
                    path: None,
                    running: true,
                });
            }
        }

        None
    }

    /// Detect JetBrains IDEs
    fn detect_jetbrains() -> Vec<DetectedIde> {
        let mut ides = Vec::new();

        let jetbrains_ides = [
            ("idea", IdeType::IntelliJ),
            ("pycharm", IdeType::PyCharm),
            ("webstorm", IdeType::WebStorm),
            ("phpstorm", IdeType::PhpStorm),
            ("rubymine", IdeType::RubyMine),
            ("clion", IdeType::CLion),
            ("goland", IdeType::GoLand),
            ("rider", IdeType::Rider),
            ("datagrip", IdeType::DataGrip),
            ("studio", IdeType::AndroidStudio),
            ("fleet", IdeType::Fleet),
        ];

        for (process_name, ide_type) in jetbrains_ides {
            if Self::is_process_running(process_name) {
                ides.push(DetectedIde {
                    ide_type,
                    name: ide_type.display_name().to_string(),
                    version: None,
                    path: None,
                    running: true,
                });
            }
        }

        ides
    }

    /// Detect Vim/Neovim
    fn detect_vim() -> Option<DetectedIde> {
        if Self::is_process_running("nvim") || Self::is_process_running("vim") {
            return Some(DetectedIde {
                ide_type: IdeType::Vim,
                name: "Vim/Neovim".to_string(),
                version: None,
                path: None,
                running: true,
            });
        }
        None
    }

    /// Check if a process is running
    fn is_process_running(name: &str) -> bool {
        #[cfg(target_os = "macos")]
        {
            Command::new("pgrep")
                .args(["-x", name])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }

        #[cfg(target_os = "linux")]
        {
            Command::new("pgrep")
                .args(["-x", name])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("tasklist")
                .args(["/FI", &format!("IMAGENAME eq {}", name)])
                .output()
                .map(|o| {
                    let output = String::from_utf8_lossy(&o.stdout);
                    output.contains(name)
                })
                .unwrap_or(false)
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            false
        }
    }

    /// Get installed IDEs (not necessarily running)
    pub fn detect_installed() -> Vec<DetectedIde> {
        let mut ides = Vec::new();

        // Check for VS Code
        if Self::is_vscode_installed() {
            ides.push(DetectedIde {
                ide_type: IdeType::VsCode,
                name: "Visual Studio Code".to_string(),
                version: None,
                path: Self::get_vscode_path(),
                running: false,
            });
        }

        // Check for JetBrains Toolbox
        if let Some(toolbox_ides) = Self::detect_jetbrains_toolbox() {
            ides.extend(toolbox_ides);
        }

        ides
    }

    /// Check if VS Code is installed
    fn is_vscode_installed() -> bool {
        #[cfg(target_os = "macos")]
        {
            Path::new("/Applications/Visual Studio Code.app").exists()
        }

        #[cfg(target_os = "linux")]
        {
            Command::new("which")
                .arg("code")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }

        #[cfg(target_os = "windows")]
        {
            // Check common installation paths
            let paths = [
                r"C:\Program Files\Microsoft VS Code\Code.exe",
                r"C:\Users\*\AppData\Local\Programs\Microsoft VS Code\Code.exe",
            ];
            paths.iter().any(|p| Path::new(p).exists())
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            false
        }
    }

    /// Get VS Code executable path
    fn get_vscode_path() -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            Some("/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code".to_string())
        }

        #[cfg(target_os = "linux")]
        {
            Command::new("which")
                .arg("code")
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                    } else {
                        None
                    }
                })
        }

        #[cfg(target_os = "windows")]
        {
            Some(r"C:\Program Files\Microsoft VS Code\Code.exe".to_string())
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            None
        }
    }

    /// Detect JetBrains Toolbox installed IDEs
    fn detect_jetbrains_toolbox() -> Option<Vec<DetectedIde>> {
        let toolbox_path = Self::get_jetbrains_toolbox_path()?;

        if !toolbox_path.exists() {
            return None;
        }

        // TODO: Parse JetBrains Toolbox configuration to find installed IDEs
        // For now, return empty
        Some(Vec::new())
    }

    /// Get JetBrains Toolbox path
    fn get_jetbrains_toolbox_path() -> Option<std::path::PathBuf> {
        let home = dirs::home_dir()?;

        #[cfg(target_os = "macos")]
        {
            Some(home.join("Library/Application Support/JetBrains/Toolbox"))
        }

        #[cfg(target_os = "linux")]
        {
            Some(home.join(".local/share/JetBrains/Toolbox"))
        }

        #[cfg(target_os = "windows")]
        {
            Some(home.join("AppData/Local/JetBrains/Toolbox"))
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ide_type_display_name() {
        assert_eq!(IdeType::VsCode.display_name(), "Visual Studio Code");
        assert_eq!(IdeType::IntelliJ.display_name(), "IntelliJ IDEA");
    }

    #[test]
    fn test_ide_type_is_jetbrains() {
        assert!(IdeType::IntelliJ.is_jetbrains());
        assert!(IdeType::PyCharm.is_jetbrains());
        assert!(!IdeType::VsCode.is_jetbrains());
    }

    #[test]
    fn test_ide_type_is_vscode() {
        assert!(IdeType::VsCode.is_vscode());
        assert!(IdeType::Cursor.is_vscode());
        assert!(!IdeType::IntelliJ.is_vscode());
    }
}
