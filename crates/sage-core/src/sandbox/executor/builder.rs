//! Builder pattern for sandbox command execution

use super::executor::SandboxExecutor;
use super::types::SandboxedExecution;
use crate::sandbox::SandboxError;
use crate::sandbox::limits::ResourceLimits;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Builder for constructing and executing sandbox commands
pub struct ExecutionBuilder {
    command: String,
    args: Vec<String>,
    working_dir: Option<PathBuf>,
    env: HashMap<String, String>,
    limits: ResourceLimits,
    timeout: Duration,
}

impl ExecutionBuilder {
    /// Create a new execution builder for the given command
    pub fn new(command: &str) -> Self {
        Self {
            command: command.to_string(),
            args: Vec::new(),
            working_dir: None,
            env: HashMap::new(),
            limits: ResourceLimits::default(),
            timeout: Duration::from_secs(30),
        }
    }

    /// Add an argument
    pub fn arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }

    /// Set working directory
    pub fn working_dir(mut self, dir: &str) -> Self {
        self.working_dir = Some(PathBuf::from(dir));
        self
    }

    /// Set an environment variable
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.insert(key.to_string(), value.to_string());
        self
    }

    /// Set timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Execute the command
    pub async fn execute(self) -> Result<SandboxedExecution, SandboxError> {
        let env = if self.env.is_empty() {
            None
        } else {
            Some(&self.env)
        };
        SandboxExecutor::execute(
            &self.command,
            &self.args,
            self.working_dir.as_ref(),
            env,
            &self.limits,
            self.timeout,
        )
        .await
    }
}
