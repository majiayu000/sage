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
#[path = "source_tests.rs"]
mod source_tests;
