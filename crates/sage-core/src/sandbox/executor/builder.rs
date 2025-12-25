//! Builder for sandboxed executions

use super::executor::SandboxExecutor;
use super::types::SandboxedExecution;
use crate::sandbox::limits::ResourceLimits;
use crate::sandbox::SandboxError;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Builder for sandboxed executions
#[allow(dead_code)]
pub struct ExecutionBuilder {
    command: String,
    args: Vec<String>,
    working_dir: Option<PathBuf>,
    env: HashMap<String, String>,
    limits: ResourceLimits,
    timeout: Duration,
}

#[allow(dead_code)]
impl ExecutionBuilder {
    /// Create a new execution builder
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            working_dir: None,
            env: HashMap::new(),
            limits: ResourceLimits::default(),
            timeout: Duration::from_secs(120),
        }
    }

    /// Add argument
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Set working directory
    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Set environment variable
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set multiple environment variables
    pub fn envs(
        mut self,
        vars: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        for (key, value) in vars {
            self.env.insert(key.into(), value.into());
        }
        self
    }

    /// Set resource limits
    pub fn limits(mut self, limits: ResourceLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Set timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Execute the command
    pub async fn execute(self) -> Result<SandboxedExecution, SandboxError> {
        SandboxExecutor::execute(
            &self.command,
            &self.args,
            self.working_dir.as_ref(),
            Some(&self.env),
            &self.limits,
            self.timeout,
        )
        .await
    }
}
