//! Sandbox command executor

use super::limits::ResourceLimits;
use super::SandboxError;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

/// Result of a sandboxed execution
#[derive(Debug, Clone)]
pub struct SandboxedExecution {
    /// Exit code (None if process was killed)
    pub exit_code: Option<i32>,

    /// Standard output
    pub stdout: String,

    /// Standard error
    pub stderr: String,

    /// Execution duration
    pub duration: Duration,

    /// Whether execution was killed due to timeout
    pub timed_out: bool,

    /// Whether execution was killed due to resource limits
    pub resource_limited: bool,

    /// Resource usage during execution
    pub resource_usage: ExecutionResourceUsage,
}

impl SandboxedExecution {
    /// Check if execution was successful
    pub fn success(&self) -> bool {
        self.exit_code == Some(0) && !self.timed_out && !self.resource_limited
    }

    /// Get combined output (stdout + stderr)
    pub fn combined_output(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }
}

/// Resource usage during execution
#[derive(Debug, Clone, Default)]
pub struct ExecutionResourceUsage {
    /// Peak memory usage (estimated)
    pub peak_memory_bytes: u64,

    /// Total output bytes
    pub output_bytes: u64,

    /// CPU time (user + system) in milliseconds
    pub cpu_time_ms: u64,
}

/// Sandbox executor for running commands
pub struct SandboxExecutor;

impl SandboxExecutor {
    /// Execute a command in the sandbox
    pub async fn execute(
        command: &str,
        args: &[String],
        working_dir: Option<&PathBuf>,
        env: Option<&HashMap<String, String>>,
        limits: &ResourceLimits,
        timeout_duration: Duration,
    ) -> Result<SandboxedExecution, SandboxError> {
        let start = Instant::now();

        // Build command
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        // Set working directory
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        // Set environment variables
        if let Some(env_vars) = env {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }

        // Apply resource limits (platform-specific)
        #[cfg(unix)]
        Self::apply_unix_limits(&mut cmd, limits);

        // Spawn process
        let mut child = cmd.spawn().map_err(|e| SandboxError::SpawnFailed(e.to_string()))?;

        // Capture output with limits
        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        let max_output = limits.max_output_bytes.unwrap_or(u64::MAX);

        // Read stdout with limit
        let stdout_task = tokio::spawn(async move {
            if let Some(stdout) = stdout_handle {
                Self::read_output_limited(stdout, max_output).await
            } else {
                String::new()
            }
        });

        // Read stderr with limit
        let stderr_task = tokio::spawn(async move {
            if let Some(stderr) = stderr_handle {
                Self::read_output_limited(stderr, max_output).await
            } else {
                String::new()
            }
        });

        // Wait for completion with timeout
        let result = timeout(timeout_duration, child.wait()).await;

        let (exit_code, timed_out) = match result {
            Ok(Ok(status)) => (status.code(), false),
            Ok(Err(e)) => {
                return Err(SandboxError::Internal(format!(
                    "Process wait failed: {}",
                    e
                )));
            }
            Err(_) => {
                // Timeout - kill the process
                let _ = child.kill().await;
                (None, true)
            }
        };

        // Collect output
        let stdout = stdout_task.await.unwrap_or_default();
        let stderr = stderr_task.await.unwrap_or_default();

        let duration = start.elapsed();
        let output_bytes = (stdout.len() + stderr.len()) as u64;

        // Check output limit
        let resource_limited = limits
            .max_output_bytes
            .map_or(false, |limit| output_bytes > limit);

        Ok(SandboxedExecution {
            exit_code,
            stdout,
            stderr,
            duration,
            timed_out,
            resource_limited,
            resource_usage: ExecutionResourceUsage {
                peak_memory_bytes: 0, // Would need platform-specific tracking
                output_bytes,
                cpu_time_ms: duration.as_millis() as u64, // Approximation
            },
        })
    }

    /// Read output with size limit
    async fn read_output_limited<R: tokio::io::AsyncRead + Unpin>(
        reader: R,
        max_bytes: u64,
    ) -> String {
        let mut reader = BufReader::new(reader);
        let mut output = String::new();
        let mut total_bytes: u64 = 0;

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(n) => {
                    total_bytes += n as u64;
                    if total_bytes > max_bytes {
                        output.push_str("\n... (output truncated due to size limit)");
                        break;
                    }
                    output.push_str(&line);
                }
                Err(_) => break,
            }
        }

        output
    }

    /// Apply Unix-specific resource limits
    #[cfg(unix)]
    #[allow(unused_imports)]
    fn apply_unix_limits(cmd: &mut Command, limits: &ResourceLimits) {
        use std::os::unix::process::CommandExt;

        // Clone limits for the closure
        let max_memory = limits.max_memory_bytes;
        let max_cpu = limits.max_cpu_seconds;
        let max_files = limits.max_open_files;
        let max_stack = limits.max_stack_bytes;

        unsafe {
            cmd.pre_exec(move || {
                // Set memory limit (RLIMIT_AS)
                if let Some(mem) = max_memory {
                    let limit = libc::rlimit {
                        rlim_cur: mem,
                        rlim_max: mem,
                    };
                    libc::setrlimit(libc::RLIMIT_AS, &limit);
                }

                // Set CPU time limit (RLIMIT_CPU)
                if let Some(cpu) = max_cpu {
                    let limit = libc::rlimit {
                        rlim_cur: cpu,
                        rlim_max: cpu,
                    };
                    libc::setrlimit(libc::RLIMIT_CPU, &limit);
                }

                // Set open files limit (RLIMIT_NOFILE)
                if let Some(files) = max_files {
                    let limit = libc::rlimit {
                        rlim_cur: files as u64,
                        rlim_max: files as u64,
                    };
                    libc::setrlimit(libc::RLIMIT_NOFILE, &limit);
                }

                // Set stack size limit (RLIMIT_STACK)
                if let Some(stack) = max_stack {
                    let limit = libc::rlimit {
                        rlim_cur: stack,
                        rlim_max: stack,
                    };
                    libc::setrlimit(libc::RLIMIT_STACK, &limit);
                }

                Ok(())
            });
        }
    }

    /// Execute a shell command (through /bin/sh)
    pub async fn execute_shell(
        shell_command: &str,
        working_dir: Option<&PathBuf>,
        env: Option<&HashMap<String, String>>,
        limits: &ResourceLimits,
        timeout_duration: Duration,
    ) -> Result<SandboxedExecution, SandboxError> {
        Self::execute(
            "sh",
            &["-c".to_string(), shell_command.to_string()],
            working_dir,
            env,
            limits,
            timeout_duration,
        )
        .await
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_execution() {
        let result = SandboxExecutor::execute(
            "echo",
            &["hello".to_string()],
            None,
            None,
            &ResourceLimits::default(),
            Duration::from_secs(10),
        )
        .await
        .unwrap();

        assert!(result.success());
        assert!(result.stdout.contains("hello"));
        assert!(!result.timed_out);
    }

    #[tokio::test]
    async fn test_exit_code() {
        let result = SandboxExecutor::execute(
            "false",
            &[],
            None,
            None,
            &ResourceLimits::default(),
            Duration::from_secs(10),
        )
        .await
        .unwrap();

        assert!(!result.success());
        assert_eq!(result.exit_code, Some(1));
    }

    #[tokio::test]
    async fn test_timeout() {
        let result = SandboxExecutor::execute(
            "sleep",
            &["10".to_string()],
            None,
            None,
            &ResourceLimits::default(),
            Duration::from_millis(100),
        )
        .await
        .unwrap();

        assert!(result.timed_out);
        assert!(!result.success());
    }

    #[tokio::test]
    async fn test_shell_execution() {
        let result = SandboxExecutor::execute_shell(
            "echo $((1 + 1))",
            None,
            None,
            &ResourceLimits::default(),
            Duration::from_secs(10),
        )
        .await
        .unwrap();

        assert!(result.success());
        assert!(result.stdout.contains("2"));
    }

    #[tokio::test]
    async fn test_execution_builder() {
        let result = ExecutionBuilder::new("echo")
            .arg("hello")
            .arg("world")
            .timeout(Duration::from_secs(10))
            .execute()
            .await
            .unwrap();

        assert!(result.success());
        assert!(result.stdout.contains("hello world"));
    }

    #[tokio::test]
    async fn test_working_directory() {
        let result = ExecutionBuilder::new("pwd")
            .working_dir("/tmp")
            .timeout(Duration::from_secs(10))
            .execute()
            .await
            .unwrap();

        assert!(result.success());
        // On macOS /tmp is symlinked to /private/tmp
        assert!(result.stdout.contains("tmp"));
    }

    #[tokio::test]
    async fn test_environment_variables() {
        let result = ExecutionBuilder::new("sh")
            .arg("-c")
            .arg("echo $TEST_VAR")
            .env("TEST_VAR", "test_value")
            .timeout(Duration::from_secs(10))
            .execute()
            .await
            .unwrap();

        assert!(result.success());
        assert!(result.stdout.contains("test_value"));
    }

    #[tokio::test]
    async fn test_stderr_capture() {
        let result = ExecutionBuilder::new("sh")
            .arg("-c")
            .arg("echo error >&2")
            .timeout(Duration::from_secs(10))
            .execute()
            .await
            .unwrap();

        assert!(result.success());
        assert!(result.stderr.contains("error"));
    }

    #[tokio::test]
    async fn test_combined_output() {
        let result = ExecutionBuilder::new("sh")
            .arg("-c")
            .arg("echo stdout; echo stderr >&2")
            .timeout(Duration::from_secs(10))
            .execute()
            .await
            .unwrap();

        let combined = result.combined_output();
        assert!(combined.contains("stdout"));
        assert!(combined.contains("stderr"));
    }
}
