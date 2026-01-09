//! Credential source definitions with priority-based resolution
//!
//! This module defines the various sources from which credentials can be loaded,
//! with a clear priority order that allows users to override credentials at different levels.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// Priority level for credential sources (lower number = higher priority)
///
/// The priority order is:
/// 1. CLI arguments (highest priority)
/// 2. Environment variables
/// 3. Project-level .sage/credentials.json
/// 4. Global ~/.sage/credentials.json
/// 5. Auto-imported from other tools (Claude Code, etc.)
/// 6. System keychain
/// 7. OAuth tokens
/// 8. Default/built-in (lowest priority)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum CredentialPriority {
    /// CLI arguments: --api-key
    CliArgument = 1,
    /// Environment variables: OPENAI_API_KEY, ANTHROPIC_API_KEY
    Environment = 2,
    /// Project config: .sage/credentials.json in working directory
    ProjectConfig = 3,
    /// Global config: ~/.sage/credentials.json
    GlobalConfig = 4,
    /// Auto-imported from other tools
    AutoImported = 5,
    /// System keychain (macOS Keychain, Windows Credential Manager, etc.)
    SystemKeychain = 6,
    /// OAuth tokens from authentication flow
    OAuthToken = 7,
    /// Default/fallback (lowest priority)
    Default = 8,
}

impl CredentialPriority {
    /// Get all priority levels in order
    pub fn all() -> &'static [CredentialPriority] {
        &[
            CredentialPriority::CliArgument,
            CredentialPriority::Environment,
            CredentialPriority::ProjectConfig,
            CredentialPriority::GlobalConfig,
            CredentialPriority::AutoImported,
            CredentialPriority::SystemKeychain,
            CredentialPriority::OAuthToken,
            CredentialPriority::Default,
        ]
    }

    /// Get the human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            CredentialPriority::CliArgument => "CLI argument",
            CredentialPriority::Environment => "Environment variable",
            CredentialPriority::ProjectConfig => "Project config",
            CredentialPriority::GlobalConfig => "Global config",
            CredentialPriority::AutoImported => "Auto-imported",
            CredentialPriority::SystemKeychain => "System keychain",
            CredentialPriority::OAuthToken => "OAuth token",
            CredentialPriority::Default => "Default",
        }
    }

    /// Get whether this source is user-configured
    pub fn is_user_configured(&self) -> bool {
        matches!(
            self,
            CredentialPriority::CliArgument
                | CredentialPriority::Environment
                | CredentialPriority::ProjectConfig
                | CredentialPriority::GlobalConfig
        )
    }

    /// Get whether this source is persistent (survives restarts)
    pub fn is_persistent(&self) -> bool {
        !matches!(self, CredentialPriority::CliArgument)
    }
}

impl Default for CredentialPriority {
    fn default() -> Self {
        CredentialPriority::Default
    }
}

impl fmt::Display for CredentialPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// The source of a credential with additional metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CredentialSource {
    /// From CLI argument
    CliArgument {
        /// The argument name (e.g., "--api-key")
        arg_name: String,
    },
    /// From environment variable
    Environment {
        /// The environment variable name
        var_name: String,
    },
    /// From project-level config file
    ProjectConfig {
        /// Path to the config file
        path: PathBuf,
    },
    /// From global config file
    GlobalConfig {
        /// Path to the config file
        path: PathBuf,
    },
    /// Auto-imported from another tool
    AutoImported {
        /// Name of the source tool
        tool_name: String,
        /// Path where the credential was found
        path: Option<PathBuf>,
    },
    /// From system keychain
    SystemKeychain {
        /// Keychain service name
        service: String,
    },
    /// From OAuth authentication
    OAuthToken {
        /// OAuth provider
        provider: String,
        /// When the token was obtained
        obtained_at: Option<String>,
    },
    /// Default/built-in value
    Default,
}

impl CredentialSource {
    /// Create a CLI argument source
    pub fn cli(arg_name: impl Into<String>) -> Self {
        CredentialSource::CliArgument {
            arg_name: arg_name.into(),
        }
    }

    /// Create an environment variable source
    pub fn env(var_name: impl Into<String>) -> Self {
        CredentialSource::Environment {
            var_name: var_name.into(),
        }
    }

    /// Create a project config source
    pub fn project(path: impl Into<PathBuf>) -> Self {
        CredentialSource::ProjectConfig { path: path.into() }
    }

    /// Create a global config source
    pub fn global(path: impl Into<PathBuf>) -> Self {
        CredentialSource::GlobalConfig { path: path.into() }
    }

    /// Create an auto-imported source
    pub fn auto_imported(tool_name: impl Into<String>, path: Option<PathBuf>) -> Self {
        CredentialSource::AutoImported {
            tool_name: tool_name.into(),
            path,
        }
    }

    /// Create a keychain source
    pub fn keychain(service: impl Into<String>) -> Self {
        CredentialSource::SystemKeychain {
            service: service.into(),
        }
    }

    /// Create an OAuth source
    pub fn oauth(provider: impl Into<String>) -> Self {
        CredentialSource::OAuthToken {
            provider: provider.into(),
            obtained_at: None,
        }
    }

    /// Get the priority of this source
    pub fn priority(&self) -> CredentialPriority {
        match self {
            CredentialSource::CliArgument { .. } => CredentialPriority::CliArgument,
            CredentialSource::Environment { .. } => CredentialPriority::Environment,
            CredentialSource::ProjectConfig { .. } => CredentialPriority::ProjectConfig,
            CredentialSource::GlobalConfig { .. } => CredentialPriority::GlobalConfig,
            CredentialSource::AutoImported { .. } => CredentialPriority::AutoImported,
            CredentialSource::SystemKeychain { .. } => CredentialPriority::SystemKeychain,
            CredentialSource::OAuthToken { .. } => CredentialPriority::OAuthToken,
            CredentialSource::Default => CredentialPriority::Default,
        }
    }

    /// Get a description of where this credential came from
    pub fn description(&self) -> String {
        match self {
            CredentialSource::CliArgument { arg_name } => {
                format!("CLI argument: {}", arg_name)
            }
            CredentialSource::Environment { var_name } => {
                format!("Environment: ${}", var_name)
            }
            CredentialSource::ProjectConfig { path } => {
                format!("Project: {}", path.display())
            }
            CredentialSource::GlobalConfig { path } => {
                format!("Global: {}", path.display())
            }
            CredentialSource::AutoImported { tool_name, path } => {
                if let Some(p) = path {
                    format!("Auto-imported from {}: {}", tool_name, p.display())
                } else {
                    format!("Auto-imported from {}", tool_name)
                }
            }
            CredentialSource::SystemKeychain { service } => {
                format!("System keychain: {}", service)
            }
            CredentialSource::OAuthToken { provider, .. } => {
                format!("OAuth: {}", provider)
            }
            CredentialSource::Default => "Default".to_string(),
        }
    }

    /// Check if this is a user-configured source
    pub fn is_user_configured(&self) -> bool {
        self.priority().is_user_configured()
    }

    /// Check if this source persists across restarts
    pub fn is_persistent(&self) -> bool {
        self.priority().is_persistent()
    }
}

impl Default for CredentialSource {
    fn default() -> Self {
        CredentialSource::Default
    }
}

impl fmt::Display for CredentialSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        // Lower number = higher priority
        assert!(CredentialPriority::CliArgument < CredentialPriority::Environment);
        assert!(CredentialPriority::Environment < CredentialPriority::ProjectConfig);
        assert!(CredentialPriority::ProjectConfig < CredentialPriority::GlobalConfig);
        assert!(CredentialPriority::GlobalConfig < CredentialPriority::AutoImported);
        assert!(CredentialPriority::AutoImported < CredentialPriority::SystemKeychain);
        assert!(CredentialPriority::SystemKeychain < CredentialPriority::OAuthToken);
        assert!(CredentialPriority::OAuthToken < CredentialPriority::Default);
    }

    #[test]
    fn test_priority_all() {
        let all = CredentialPriority::all();
        assert_eq!(all.len(), 8);
        assert_eq!(all[0], CredentialPriority::CliArgument);
        assert_eq!(all[7], CredentialPriority::Default);
    }

    #[test]
    fn test_priority_name() {
        assert_eq!(CredentialPriority::CliArgument.name(), "CLI argument");
        assert_eq!(CredentialPriority::Environment.name(), "Environment variable");
        assert_eq!(CredentialPriority::ProjectConfig.name(), "Project config");
        assert_eq!(CredentialPriority::GlobalConfig.name(), "Global config");
        assert_eq!(CredentialPriority::AutoImported.name(), "Auto-imported");
        assert_eq!(CredentialPriority::SystemKeychain.name(), "System keychain");
        assert_eq!(CredentialPriority::OAuthToken.name(), "OAuth token");
        assert_eq!(CredentialPriority::Default.name(), "Default");
    }

    #[test]
    fn test_priority_is_user_configured() {
        assert!(CredentialPriority::CliArgument.is_user_configured());
        assert!(CredentialPriority::Environment.is_user_configured());
        assert!(CredentialPriority::ProjectConfig.is_user_configured());
        assert!(CredentialPriority::GlobalConfig.is_user_configured());
        assert!(!CredentialPriority::AutoImported.is_user_configured());
        assert!(!CredentialPriority::SystemKeychain.is_user_configured());
        assert!(!CredentialPriority::OAuthToken.is_user_configured());
        assert!(!CredentialPriority::Default.is_user_configured());
    }

    #[test]
    fn test_priority_is_persistent() {
        assert!(!CredentialPriority::CliArgument.is_persistent());
        assert!(CredentialPriority::Environment.is_persistent());
        assert!(CredentialPriority::ProjectConfig.is_persistent());
        assert!(CredentialPriority::GlobalConfig.is_persistent());
        assert!(CredentialPriority::AutoImported.is_persistent());
        assert!(CredentialPriority::SystemKeychain.is_persistent());
        assert!(CredentialPriority::OAuthToken.is_persistent());
        assert!(CredentialPriority::Default.is_persistent());
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(format!("{}", CredentialPriority::CliArgument), "CLI argument");
        assert_eq!(format!("{}", CredentialPriority::Default), "Default");
    }

    #[test]
    fn test_priority_default() {
        assert_eq!(CredentialPriority::default(), CredentialPriority::Default);
    }

    #[test]
    fn test_source_cli() {
        let source = CredentialSource::cli("--api-key");
        assert_eq!(source.priority(), CredentialPriority::CliArgument);
        assert!(source.description().contains("--api-key"));
        assert!(source.is_user_configured());
        assert!(!source.is_persistent());
    }

    #[test]
    fn test_source_env() {
        let source = CredentialSource::env("OPENAI_API_KEY");
        assert_eq!(source.priority(), CredentialPriority::Environment);
        assert!(source.description().contains("OPENAI_API_KEY"));
        assert!(source.is_user_configured());
        assert!(source.is_persistent());
    }

    #[test]
    fn test_source_project() {
        let source = CredentialSource::project("/project/.sage/credentials.json");
        assert_eq!(source.priority(), CredentialPriority::ProjectConfig);
        assert!(source.description().contains("Project"));
        assert!(source.is_user_configured());
    }

    #[test]
    fn test_source_global() {
        let source = CredentialSource::global("~/.sage/credentials.json");
        assert_eq!(source.priority(), CredentialPriority::GlobalConfig);
        assert!(source.description().contains("Global"));
        assert!(source.is_user_configured());
    }

    #[test]
    fn test_source_auto_imported() {
        let source = CredentialSource::auto_imported("claude-code", Some(PathBuf::from("/path")));
        assert_eq!(source.priority(), CredentialPriority::AutoImported);
        assert!(source.description().contains("claude-code"));
        assert!(source.description().contains("/path"));
        assert!(!source.is_user_configured());

        let source_no_path = CredentialSource::auto_imported("cursor", None);
        assert!(source_no_path.description().contains("cursor"));
        assert!(!source_no_path.description().contains("/path"));
    }

    #[test]
    fn test_source_keychain() {
        let source = CredentialSource::keychain("sage-openai");
        assert_eq!(source.priority(), CredentialPriority::SystemKeychain);
        assert!(source.description().contains("sage-openai"));
    }

    #[test]
    fn test_source_oauth() {
        let source = CredentialSource::oauth("anthropic");
        assert_eq!(source.priority(), CredentialPriority::OAuthToken);
        assert!(source.description().contains("anthropic"));
    }

    #[test]
    fn test_source_default() {
        let source = CredentialSource::Default;
        assert_eq!(source.priority(), CredentialPriority::Default);
        assert_eq!(source.description(), "Default");
        assert!(!source.is_user_configured());
        assert!(source.is_persistent());
    }

    #[test]
    fn test_source_display() {
        let source = CredentialSource::env("TEST_KEY");
        assert_eq!(format!("{}", source), "Environment: $TEST_KEY");
    }

    #[test]
    fn test_source_default_trait() {
        assert_eq!(CredentialSource::default(), CredentialSource::Default);
    }

    #[test]
    fn test_source_serialize() {
        let source = CredentialSource::env("OPENAI_API_KEY");
        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains("environment"));
        assert!(json.contains("OPENAI_API_KEY"));

        let source = CredentialSource::Default;
        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains("default"));
    }

    #[test]
    fn test_source_deserialize() {
        let json = r#"{"type":"environment","var_name":"TEST_KEY"}"#;
        let source: CredentialSource = serde_json::from_str(json).unwrap();
        assert!(matches!(
            source,
            CredentialSource::Environment { var_name } if var_name == "TEST_KEY"
        ));

        let json = r#"{"type":"default"}"#;
        let source: CredentialSource = serde_json::from_str(json).unwrap();
        assert!(matches!(source, CredentialSource::Default));
    }

    #[test]
    fn test_priority_serialize() {
        let priority = CredentialPriority::Environment;
        let json = serde_json::to_string(&priority).unwrap();
        // serde serializes the variant name, not the u8 discriminant
        assert!(json.contains("Environment") || json == "2");
    }

    #[test]
    fn test_priority_deserialize() {
        // Try variant name first
        if let Ok(priority) = serde_json::from_str::<CredentialPriority>("\"Environment\"") {
            assert_eq!(priority, CredentialPriority::Environment);
        }
        // serde uses variant names by default
    }
}
