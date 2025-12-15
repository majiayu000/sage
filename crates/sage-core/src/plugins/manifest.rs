//! Plugin manifest definitions

use serde::{Deserialize, Serialize};
use super::PluginCapability;

/// Plugin manifest describing metadata and requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin name (unique identifier)
    pub name: String,

    /// Plugin version (semver)
    pub version: String,

    /// Human-readable description
    pub description: Option<String>,

    /// Plugin author
    pub author: Option<String>,

    /// Homepage URL
    pub homepage: Option<String>,

    /// Repository URL
    pub repository: Option<String>,

    /// License identifier
    pub license: Option<String>,

    /// Plugin capabilities
    pub capabilities: Vec<PluginCapability>,

    /// Required permissions
    pub permissions: Vec<PluginPermission>,

    /// Plugin dependencies
    pub dependencies: Vec<PluginDependency>,

    /// Minimum sage version required
    pub min_sage_version: Option<String>,

    /// Maximum sage version supported
    pub max_sage_version: Option<String>,

    /// Entry point (for external plugins)
    pub entry_point: Option<String>,

    /// Additional metadata
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl PluginManifest {
    /// Create a new manifest
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: None,
            author: None,
            homepage: None,
            repository: None,
            license: None,
            capabilities: Vec::new(),
            permissions: Vec::new(),
            dependencies: Vec::new(),
            min_sage_version: None,
            max_sage_version: None,
            entry_point: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set author
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Add capability
    pub fn capability(mut self, cap: PluginCapability) -> Self {
        self.capabilities.push(cap);
        self
    }

    /// Add permission
    pub fn permission(mut self, perm: PluginPermission) -> Self {
        self.permissions.push(perm);
        self
    }

    /// Add dependency
    pub fn dependency(mut self, dep: PluginDependency) -> Self {
        self.dependencies.push(dep);
        self
    }

    /// Set minimum sage version
    pub fn min_sage_version(mut self, version: impl Into<String>) -> Self {
        self.min_sage_version = Some(version.into());
        self
    }

    /// Validate the manifest
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Name validation
        if self.name.is_empty() {
            errors.push("Plugin name cannot be empty".to_string());
        }
        if !self.name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            errors.push("Plugin name can only contain alphanumeric characters, hyphens, and underscores".to_string());
        }

        // Version validation (basic semver check)
        if self.version.is_empty() {
            errors.push("Plugin version cannot be empty".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Plugin permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginPermission {
    /// Read files
    ReadFiles,

    /// Write files
    WriteFiles,

    /// Execute commands
    ExecuteCommands,

    /// Network access
    NetworkAccess,

    /// Environment variables
    EnvironmentAccess,

    /// System information
    SystemInfo,

    /// Access to other plugins
    PluginInterop,

    /// Configuration access
    ConfigAccess,

    /// Event subscription
    EventSubscription,

    /// Unsafe operations (requires explicit approval)
    Unsafe,

    /// Custom permission
    Custom(String),
}

impl PluginPermission {
    /// Check if this is a dangerous permission
    pub fn is_dangerous(&self) -> bool {
        matches!(
            self,
            PluginPermission::WriteFiles
                | PluginPermission::ExecuteCommands
                | PluginPermission::Unsafe
        )
    }

    /// Get permission description
    pub fn description(&self) -> &str {
        match self {
            PluginPermission::ReadFiles => "Read files from the filesystem",
            PluginPermission::WriteFiles => "Write files to the filesystem",
            PluginPermission::ExecuteCommands => "Execute system commands",
            PluginPermission::NetworkAccess => "Access network resources",
            PluginPermission::EnvironmentAccess => "Access environment variables",
            PluginPermission::SystemInfo => "Access system information",
            PluginPermission::PluginInterop => "Interact with other plugins",
            PluginPermission::ConfigAccess => "Access configuration",
            PluginPermission::EventSubscription => "Subscribe to system events",
            PluginPermission::Unsafe => "Perform unsafe operations",
            PluginPermission::Custom(_) => "Custom permission",
        }
    }
}

/// Plugin dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Dependency name
    pub name: String,

    /// Required version (semver constraint)
    pub version: String,

    /// Whether the dependency is optional
    pub optional: bool,
}

impl PluginDependency {
    /// Create new required dependency
    pub fn required(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            optional: false,
        }
    }

    /// Create new optional dependency
    pub fn optional(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            optional: true,
        }
    }

    /// Check if version satisfies constraint (simplified)
    pub fn satisfies(&self, version: &str) -> bool {
        // Simple version check - could be enhanced with proper semver
        if self.version.starts_with('^') {
            // ^1.0.0 means >=1.0.0 <2.0.0
            let constraint = self.version.trim_start_matches('^');
            if let Some(major) = constraint.split('.').next() {
                if let Some(version_major) = version.split('.').next() {
                    return major == version_major;
                }
            }
        } else if self.version.starts_with('~') {
            // ~1.0.0 means >=1.0.0 <1.1.0
            let constraint = self.version.trim_start_matches('~');
            let parts: Vec<&str> = constraint.split('.').collect();
            let version_parts: Vec<&str> = version.split('.').collect();
            if parts.len() >= 2 && version_parts.len() >= 2 {
                return parts[0] == version_parts[0] && parts[1] == version_parts[1];
            }
        } else if self.version == "*" {
            return true;
        }

        // Exact match
        self.version == version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_builder() {
        let manifest = PluginManifest::new("test-plugin", "1.0.0")
            .description("A test plugin")
            .author("Test Author")
            .capability(PluginCapability::Tools)
            .permission(PluginPermission::ReadFiles)
            .dependency(PluginDependency::required("other-plugin", "^1.0.0"));

        assert_eq!(manifest.name, "test-plugin");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.capabilities.len(), 1);
        assert_eq!(manifest.permissions.len(), 1);
        assert_eq!(manifest.dependencies.len(), 1);
    }

    #[test]
    fn test_manifest_validation() {
        let valid = PluginManifest::new("valid-plugin", "1.0.0");
        assert!(valid.validate().is_ok());

        let invalid = PluginManifest::new("", "1.0.0");
        assert!(invalid.validate().is_err());

        let invalid = PluginManifest::new("invalid name!", "1.0.0");
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_permission_dangerous() {
        assert!(!PluginPermission::ReadFiles.is_dangerous());
        assert!(PluginPermission::WriteFiles.is_dangerous());
        assert!(PluginPermission::ExecuteCommands.is_dangerous());
        assert!(PluginPermission::Unsafe.is_dangerous());
    }

    #[test]
    fn test_dependency_satisfies() {
        let dep = PluginDependency::required("test", "1.0.0");
        assert!(dep.satisfies("1.0.0"));
        assert!(!dep.satisfies("2.0.0"));

        let dep = PluginDependency::required("test", "^1.0.0");
        assert!(dep.satisfies("1.0.0"));
        assert!(dep.satisfies("1.5.0"));
        assert!(!dep.satisfies("2.0.0"));

        let dep = PluginDependency::required("test", "~1.2.0");
        assert!(dep.satisfies("1.2.0"));
        assert!(dep.satisfies("1.2.5"));
        assert!(!dep.satisfies("1.3.0"));

        let dep = PluginDependency::required("test", "*");
        assert!(dep.satisfies("1.0.0"));
        assert!(dep.satisfies("999.0.0"));
    }
}
