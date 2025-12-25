//! Core sandbox executor implementation

use super::limits::apply_unix_limits;
use super::types::{ExecutionResourceUsage, SandboxedExecution};
use crate::sandbox::limits::ResourceLimits;
use crate::sandbox::SandboxError;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

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
        apply_unix_limits(&mut cmd, limits);

        // Spawn process
        let mut child = cmd
            .spawn()
            .map_err(|e| SandboxError::SpawnFailed(e.to_string()))?;

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
