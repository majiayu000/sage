//! LSP client and server configuration

use std::collections::HashMap;

/// LSP client for a specific language
pub struct LspClient {
    /// Language ID (e.g., "rust", "typescript")
    pub language_id: String,
    /// Server command
    pub command: String,
    /// Server arguments
    pub args: Vec<String>,
    /// Whether the server is running
    pub running: bool,
}

impl LspClient {
    pub fn new(language_id: &str, command: &str, args: Vec<String>) -> Self {
        Self {
            language_id: language_id.to_string(),
            command: command.to_string(),
            args,
            running: false,
        }
    }
}

/// LSP configuration
#[derive(Debug, Clone, Default)]
pub struct LspConfig {
    /// Registered language servers
    pub servers: HashMap<String, LspServerConfig>,
}

impl LspConfig {
    /// Create config with common language servers pre-registered
    pub fn with_defaults() -> Self {
        let mut config = Self::default();

        config.servers.insert(
            "rust".to_string(),
            LspServerConfig {
                language_id: "rust".to_string(),
                command: "rust-analyzer".to_string(),
                args: vec![],
                file_extensions: vec!["rs".to_string()],
            },
        );

        config.servers.insert(
            "typescript".to_string(),
            LspServerConfig {
                language_id: "typescript".to_string(),
                command: "typescript-language-server".to_string(),
                args: vec!["--stdio".to_string()],
                file_extensions: vec![
                    "ts".to_string(),
                    "tsx".to_string(),
                    "js".to_string(),
                    "jsx".to_string(),
                ],
            },
        );

        config.servers.insert(
            "python".to_string(),
            LspServerConfig {
                language_id: "python".to_string(),
                command: "pylsp".to_string(),
                args: vec![],
                file_extensions: vec!["py".to_string()],
            },
        );

        config.servers.insert(
            "go".to_string(),
            LspServerConfig {
                language_id: "go".to_string(),
                command: "gopls".to_string(),
                args: vec![],
                file_extensions: vec!["go".to_string()],
            },
        );

        config
    }
}

/// Configuration for a single LSP server
#[derive(Debug, Clone)]
pub struct LspServerConfig {
    pub language_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub file_extensions: Vec<String>,
}
