//! BashTool type definitions and construction

use sage_core::tools::base::CommandTool;
use std::path::PathBuf;

/// Tool for executing bash commands
pub struct BashTool {
    pub(crate) working_directory: PathBuf,
    pub(crate) allowed_commands: Vec<String>,
}

impl BashTool {
    /// Create a new bash tool
    pub fn new() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            allowed_commands: Vec::new(), // Empty means all commands allowed
        }
    }

    /// Create a bash tool with specific working directory
    pub fn with_working_directory(working_dir: impl Into<PathBuf>) -> Self {
        Self {
            working_directory: working_dir.into(),
            allowed_commands: Vec::new(),
        }
    }

    /// Create a bash tool with allowed commands
    pub fn with_allowed_commands(mut self, commands: impl Into<Vec<String>>) -> Self {
        self.allowed_commands = commands.into();
        self
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandTool for BashTool {
    fn allowed_commands(&self) -> Vec<&str> {
        self.allowed_commands.iter().map(|s| s.as_str()).collect()
    }

    fn command_working_directory(&self) -> &std::path::Path {
        &self.working_directory
    }

    fn command_environment(&self) -> std::collections::HashMap<String, String> {
        let mut env = std::collections::HashMap::new();

        // Add some safe environment variables
        if let Ok(path) = std::env::var("PATH") {
            env.insert("PATH".to_string(), path);
        }

        if let Ok(home) = std::env::var("HOME") {
            env.insert("HOME".to_string(), home);
        }

        if let Ok(user) = std::env::var("USER") {
            env.insert("USER".to_string(), user);
        }

        env
    }
}
