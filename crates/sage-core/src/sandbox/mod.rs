//! Sandbox execution environment for secure tool execution
//!
//! Provides isolated execution environments with resource limits,
//! path restrictions, and command filtering.

mod config;
mod executor;
mod limits;
mod policy;

pub use config::{SandboxConfig, SandboxMode};
pub use executor::{SandboxExecutor, SandboxedExecution};
pub use limits::{ResourceLimits, ResourceUsage};
pub use policy::{CommandPolicy, NetworkPolicy, PathPolicy, SandboxPolicy};

use crate::tools::base::ToolError;
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;

/// Result type for sandbox operations
pub type SandboxResult<T> = Result<T, SandboxError>;

/// Errors that can occur during sandbox operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum SandboxError {
    /// Resource limit exceeded
    #[error("Resource limit exceeded: {resource} ({current}/{limit})")]
    ResourceLimitExceeded {
        resource: String,
        current: u64,
        limit: u64,
    },

    /// Path access denied
    #[error("Path access denied: {path}")]
    PathAccessDenied { path: String },

    /// Command not allowed
    #[error("Command not allowed: {command}")]
    CommandNotAllowed { command: String },

    /// Network access denied
    #[error("Network access denied: {host}")]
    NetworkAccessDenied { host: String },

    /// Execution timeout
    #[error("Sandbox execution timeout after {0:?}")]
    Timeout(Duration),

    /// Sandbox initialization failed
    #[error("Sandbox initialization failed: {0}")]
    InitializationFailed(String),

    /// Process spawn failed
    #[error("Failed to spawn sandboxed process: {0}")]
    SpawnFailed(String),

    /// Invalid configuration
    #[error("Invalid sandbox configuration: {0}")]
    InvalidConfig(String),

    /// Permission denied
    #[error("Sandbox permission denied: {0}")]
    PermissionDenied(String),

    /// Internal error
    #[error("Sandbox internal error: {0}")]
    Internal(String),
}

impl From<SandboxError> for ToolError {
    fn from(err: SandboxError) -> Self {
        match err {
            SandboxError::Timeout(_) => ToolError::Timeout,
            SandboxError::PathAccessDenied { path } => {
                ToolError::PermissionDenied(format!("Path access denied: {}", path))
            }
            SandboxError::CommandNotAllowed { command } => {
                ToolError::PermissionDenied(format!("Command not allowed: {}", command))
            }
            SandboxError::NetworkAccessDenied { host } => {
                ToolError::PermissionDenied(format!("Network access denied: {}", host))
            }
            SandboxError::ResourceLimitExceeded { resource, .. } => {
                ToolError::ExecutionFailed(format!("Resource limit exceeded: {}", resource))
            }
            _ => ToolError::ExecutionFailed(err.to_string()),
        }
    }
}

/// Trait for sandbox implementations
#[async_trait]
pub trait Sandbox: Send + Sync {
    /// Get the sandbox name
    fn name(&self) -> &str;

    /// Check if a path is accessible
    fn check_path(&self, path: &PathBuf, write: bool) -> SandboxResult<()>;

    /// Check if a command is allowed
    fn check_command(&self, command: &str) -> SandboxResult<()>;

    /// Check if network access is allowed
    fn check_network(&self, host: &str, port: u16) -> SandboxResult<()>;

    /// Get resource limits
    fn resource_limits(&self) -> &ResourceLimits;

    /// Execute a command in the sandbox
    async fn execute_command(
        &self,
        command: &str,
        args: &[String],
        working_dir: Option<&PathBuf>,
        env: Option<&std::collections::HashMap<String, String>>,
    ) -> SandboxResult<SandboxedExecution>;

    /// Read a file within the sandbox
    async fn read_file(&self, path: &PathBuf) -> SandboxResult<String>;

    /// Write a file within the sandbox
    async fn write_file(&self, path: &PathBuf, content: &str) -> SandboxResult<()>;

    /// Check if the sandbox is active
    fn is_active(&self) -> bool;

    /// Get current resource usage
    fn current_usage(&self) -> ResourceUsage;
}

/// Default sandbox implementation
pub struct DefaultSandbox {
    config: SandboxConfig,
    policy: SandboxPolicy,
    usage: std::sync::RwLock<ResourceUsage>,
}

impl DefaultSandbox {
    /// Create a new sandbox with the given configuration
    pub fn new(config: SandboxConfig) -> SandboxResult<Self> {
        let policy = SandboxPolicy::from_config(&config)?;
        Ok(Self {
            config,
            policy,
            usage: std::sync::RwLock::new(ResourceUsage::default()),
        })
    }

    /// Create a sandbox with default configuration
    pub fn default_sandbox() -> SandboxResult<Self> {
        Self::new(SandboxConfig::default())
    }

    /// Create a permissive sandbox (minimal restrictions)
    pub fn permissive() -> SandboxResult<Self> {
        Self::new(SandboxConfig::permissive())
    }

    /// Create a strict sandbox (maximum restrictions)
    pub fn strict(working_dir: PathBuf) -> SandboxResult<Self> {
        Self::new(SandboxConfig::strict(working_dir))
    }
}

#[async_trait]
impl Sandbox for DefaultSandbox {
    fn name(&self) -> &str {
        "default"
    }

    fn check_path(&self, path: &PathBuf, write: bool) -> SandboxResult<()> {
        self.policy.path_policy.check_path(path, write)
    }

    fn check_command(&self, command: &str) -> SandboxResult<()> {
        self.policy.command_policy.check_command(command)
    }

    fn check_network(&self, host: &str, port: u16) -> SandboxResult<()> {
        self.policy.network_policy.check_access(host, port)
    }

    fn resource_limits(&self) -> &ResourceLimits {
        &self.config.limits
    }

    async fn execute_command(
        &self,
        command: &str,
        args: &[String],
        working_dir: Option<&PathBuf>,
        env: Option<&std::collections::HashMap<String, String>>,
    ) -> SandboxResult<SandboxedExecution> {
        // Check command first
        self.check_command(command)?;

        // Check working directory if specified
        if let Some(dir) = working_dir {
            self.check_path(dir, false)?;
        }

        // Execute using the sandbox executor
        SandboxExecutor::execute(
            command,
            args,
            working_dir,
            env,
            &self.config.limits,
            self.config.timeout,
        )
        .await
    }

    async fn read_file(&self, path: &PathBuf) -> SandboxResult<String> {
        self.check_path(path, false)?;

        tokio::fs::read_to_string(path)
            .await
            .map_err(|e| SandboxError::Internal(format!("Failed to read file: {}", e)))
    }

    async fn write_file(&self, path: &PathBuf, content: &str) -> SandboxResult<()> {
        self.check_path(path, true)?;

        tokio::fs::write(path, content)
            .await
            .map_err(|e| SandboxError::Internal(format!("Failed to write file: {}", e)))
    }

    fn is_active(&self) -> bool {
        self.config.enabled
    }

    fn current_usage(&self) -> ResourceUsage {
        self.usage.read().unwrap().clone()
    }
}

/// Builder for sandbox configuration
pub struct SandboxBuilder {
    config: SandboxConfig,
}

impl SandboxBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self {
            config: SandboxConfig::default(),
        }
    }

    /// Enable or disable the sandbox
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }

    /// Set sandbox mode
    pub fn mode(mut self, mode: SandboxMode) -> Self {
        self.config.mode = mode;
        self
    }

    /// Set working directory
    pub fn working_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.working_dir = Some(path.into());
        self
    }

    /// Add allowed read path
    pub fn allow_read(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.allowed_read_paths.push(path.into());
        self
    }

    /// Add allowed write path
    pub fn allow_write(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.allowed_write_paths.push(path.into());
        self
    }

    /// Add allowed command
    pub fn allow_command(mut self, command: impl Into<String>) -> Self {
        self.config.allowed_commands.push(command.into());
        self
    }

    /// Add blocked command
    pub fn block_command(mut self, command: impl Into<String>) -> Self {
        self.config.blocked_commands.push(command.into());
        self
    }

    /// Set timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set memory limit
    pub fn memory_limit(mut self, bytes: u64) -> Self {
        self.config.limits.max_memory_bytes = Some(bytes);
        self
    }

    /// Set CPU time limit
    pub fn cpu_limit(mut self, seconds: u64) -> Self {
        self.config.limits.max_cpu_seconds = Some(seconds);
        self
    }

    /// Set output size limit
    pub fn output_limit(mut self, bytes: u64) -> Self {
        self.config.limits.max_output_bytes = Some(bytes);
        self
    }

    /// Set file size limit
    pub fn file_size_limit(mut self, bytes: u64) -> Self {
        self.config.limits.max_file_size_bytes = Some(bytes);
        self
    }

    /// Allow network access
    pub fn allow_network(mut self, allow: bool) -> Self {
        self.config.allow_network = allow;
        self
    }

    /// Build the sandbox
    pub fn build(self) -> SandboxResult<DefaultSandbox> {
        DefaultSandbox::new(self.config)
    }
}

impl Default for SandboxBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_builder() {
        let sandbox = SandboxBuilder::new()
            .enabled(true)
            .mode(SandboxMode::Restricted)
            .working_dir("/tmp/sandbox")
            .allow_read("/tmp")
            .allow_write("/tmp/sandbox")
            .allow_command("ls")
            .block_command("rm -rf")
            .timeout(Duration::from_secs(30))
            .memory_limit(100 * 1024 * 1024)
            .build()
            .unwrap();

        assert!(sandbox.is_active());
        assert_eq!(sandbox.name(), "default");
    }

    #[test]
    fn test_permissive_sandbox() {
        let sandbox = DefaultSandbox::permissive().unwrap();
        assert!(sandbox.is_active());
    }

    #[test]
    fn test_sandbox_error_display() {
        let err = SandboxError::PathAccessDenied {
            path: "/etc/passwd".into(),
        };
        assert!(err.to_string().contains("/etc/passwd"));

        let err = SandboxError::CommandNotAllowed {
            command: "rm -rf".into(),
        };
        assert!(err.to_string().contains("rm -rf"));

        let err = SandboxError::Timeout(Duration::from_secs(30));
        assert!(err.to_string().contains("30"));
    }

    #[test]
    fn test_sandbox_error_to_tool_error() {
        let err: ToolError = SandboxError::Timeout(Duration::from_secs(10)).into();
        assert!(matches!(err, ToolError::Timeout));

        let err: ToolError = SandboxError::PathAccessDenied {
            path: "/test".into(),
        }
        .into();
        assert!(matches!(err, ToolError::PermissionDenied(_)));
    }
}
