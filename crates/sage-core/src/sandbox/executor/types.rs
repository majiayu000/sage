//! Types for sandboxed execution

use std::time::Duration;

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
