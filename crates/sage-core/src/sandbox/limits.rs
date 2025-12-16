//! Resource limits for sandbox execution

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Resource limits for sandboxed execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory usage in bytes
    pub max_memory_bytes: Option<u64>,

    /// Maximum CPU time in seconds
    pub max_cpu_seconds: Option<u64>,

    /// Maximum output size in bytes
    pub max_output_bytes: Option<u64>,

    /// Maximum file size that can be created/modified in bytes
    pub max_file_size_bytes: Option<u64>,

    /// Maximum number of processes
    pub max_processes: Option<u32>,

    /// Maximum number of open files
    pub max_open_files: Option<u32>,

    /// Maximum stack size in bytes
    pub max_stack_bytes: Option<u64>,

    /// Maximum number of file writes per execution
    pub max_file_writes: Option<u32>,

    /// Maximum total bytes written to files
    pub max_total_write_bytes: Option<u64>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: Some(512 * 1024 * 1024),   // 512 MB
            max_cpu_seconds: Some(60),                   // 60 seconds
            max_output_bytes: Some(10 * 1024 * 1024),    // 10 MB
            max_file_size_bytes: Some(50 * 1024 * 1024), // 50 MB
            max_processes: Some(10),
            max_open_files: Some(100),
            max_stack_bytes: Some(8 * 1024 * 1024), // 8 MB
            max_file_writes: Some(100),
            max_total_write_bytes: Some(100 * 1024 * 1024), // 100 MB
        }
    }
}

impl ResourceLimits {
    /// Create permissive resource limits
    pub fn permissive() -> Self {
        Self {
            max_memory_bytes: Some(4 * 1024 * 1024 * 1024), // 4 GB
            max_cpu_seconds: Some(300),                     // 5 minutes
            max_output_bytes: Some(100 * 1024 * 1024),      // 100 MB
            max_file_size_bytes: Some(1024 * 1024 * 1024),  // 1 GB
            max_processes: Some(100),
            max_open_files: Some(1000),
            max_stack_bytes: Some(64 * 1024 * 1024), // 64 MB
            max_file_writes: Some(1000),
            max_total_write_bytes: Some(1024 * 1024 * 1024), // 1 GB
        }
    }

    /// Create strict resource limits
    pub fn strict() -> Self {
        Self {
            max_memory_bytes: Some(128 * 1024 * 1024),   // 128 MB
            max_cpu_seconds: Some(10),                   // 10 seconds
            max_output_bytes: Some(1024 * 1024),         // 1 MB
            max_file_size_bytes: Some(10 * 1024 * 1024), // 10 MB
            max_processes: Some(1),
            max_open_files: Some(20),
            max_stack_bytes: Some(2 * 1024 * 1024), // 2 MB
            max_file_writes: Some(10),
            max_total_write_bytes: Some(10 * 1024 * 1024), // 10 MB
        }
    }

    /// Create unlimited resource limits (no restrictions)
    pub fn unlimited() -> Self {
        Self {
            max_memory_bytes: None,
            max_cpu_seconds: None,
            max_output_bytes: None,
            max_file_size_bytes: None,
            max_processes: None,
            max_open_files: None,
            max_stack_bytes: None,
            max_file_writes: None,
            max_total_write_bytes: None,
        }
    }

    /// Check if memory usage is within limits
    pub fn check_memory(&self, bytes: u64) -> bool {
        self.max_memory_bytes.map_or(true, |limit| bytes <= limit)
    }

    /// Check if CPU time is within limits
    pub fn check_cpu_time(&self, seconds: u64) -> bool {
        self.max_cpu_seconds.map_or(true, |limit| seconds <= limit)
    }

    /// Check if output size is within limits
    pub fn check_output(&self, bytes: u64) -> bool {
        self.max_output_bytes.map_or(true, |limit| bytes <= limit)
    }

    /// Check if file size is within limits
    pub fn check_file_size(&self, bytes: u64) -> bool {
        self.max_file_size_bytes
            .map_or(true, |limit| bytes <= limit)
    }

    /// Check if process count is within limits
    pub fn check_processes(&self, count: u32) -> bool {
        self.max_processes.map_or(true, |limit| count <= limit)
    }

    /// Check if open files count is within limits
    pub fn check_open_files(&self, count: u32) -> bool {
        self.max_open_files.map_or(true, |limit| count <= limit)
    }
}

/// Current resource usage tracking
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// Current memory usage in bytes
    pub memory_bytes: u64,

    /// CPU time used in milliseconds
    pub cpu_time_ms: u64,

    /// Output bytes generated
    pub output_bytes: u64,

    /// Number of files written
    pub files_written: u32,

    /// Total bytes written to files
    pub total_write_bytes: u64,

    /// Number of processes spawned
    pub processes_spawned: u32,

    /// Number of network connections made
    pub network_connections: u32,

    /// Timestamp when tracking started
    pub started_at: Option<Instant>,

    /// Total execution time in milliseconds
    pub execution_time_ms: u64,
}

impl ResourceUsage {
    /// Create new resource usage tracker
    pub fn new() -> Self {
        Self {
            started_at: Some(Instant::now()),
            ..Default::default()
        }
    }

    /// Record memory usage
    pub fn record_memory(&mut self, bytes: u64) {
        self.memory_bytes = self.memory_bytes.max(bytes);
    }

    /// Record output bytes
    pub fn record_output(&mut self, bytes: u64) {
        self.output_bytes += bytes;
    }

    /// Record file write
    pub fn record_file_write(&mut self, bytes: u64) {
        self.files_written += 1;
        self.total_write_bytes += bytes;
    }

    /// Record process spawn
    pub fn record_process_spawn(&mut self) {
        self.processes_spawned += 1;
    }

    /// Record network connection
    pub fn record_network_connection(&mut self) {
        self.network_connections += 1;
    }

    /// Update CPU time
    pub fn update_cpu_time(&mut self, ms: u64) {
        self.cpu_time_ms = ms;
    }

    /// Finalize usage tracking
    pub fn finalize(&mut self) {
        if let Some(started) = self.started_at {
            self.execution_time_ms = started.elapsed().as_millis() as u64;
        }
    }

    /// Check usage against limits
    pub fn check_against_limits(&self, limits: &ResourceLimits) -> Vec<LimitViolation> {
        let mut violations = Vec::new();

        if let Some(limit) = limits.max_memory_bytes {
            if self.memory_bytes > limit {
                violations.push(LimitViolation {
                    resource: "memory".to_string(),
                    limit,
                    current: self.memory_bytes,
                });
            }
        }

        if let Some(limit) = limits.max_cpu_seconds {
            let cpu_seconds = self.cpu_time_ms / 1000;
            if cpu_seconds > limit {
                violations.push(LimitViolation {
                    resource: "cpu_time".to_string(),
                    limit,
                    current: cpu_seconds,
                });
            }
        }

        if let Some(limit) = limits.max_output_bytes {
            if self.output_bytes > limit {
                violations.push(LimitViolation {
                    resource: "output".to_string(),
                    limit,
                    current: self.output_bytes,
                });
            }
        }

        if let Some(limit) = limits.max_file_writes {
            if self.files_written > limit {
                violations.push(LimitViolation {
                    resource: "file_writes".to_string(),
                    limit: limit as u64,
                    current: self.files_written as u64,
                });
            }
        }

        if let Some(limit) = limits.max_total_write_bytes {
            if self.total_write_bytes > limit {
                violations.push(LimitViolation {
                    resource: "total_write_bytes".to_string(),
                    limit,
                    current: self.total_write_bytes,
                });
            }
        }

        violations
    }
}

/// Represents a resource limit violation
#[derive(Debug, Clone)]
pub struct LimitViolation {
    /// Name of the resource
    pub resource: String,
    /// The limit that was exceeded
    pub limit: u64,
    /// The current usage
    pub current: u64,
}

impl std::fmt::Display for LimitViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Resource '{}' exceeded limit: {} > {}",
            self.resource, self.current, self.limit
        )
    }
}

/// Resource usage summary for reporting
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageSummary {
    pub memory_mb: f64,
    pub cpu_seconds: f64,
    pub output_kb: f64,
    pub files_written: u32,
    pub total_write_mb: f64,
    pub execution_seconds: f64,
}

impl From<&ResourceUsage> for ResourceUsageSummary {
    fn from(usage: &ResourceUsage) -> Self {
        Self {
            memory_mb: usage.memory_bytes as f64 / (1024.0 * 1024.0),
            cpu_seconds: usage.cpu_time_ms as f64 / 1000.0,
            output_kb: usage.output_bytes as f64 / 1024.0,
            files_written: usage.files_written,
            total_write_mb: usage.total_write_bytes as f64 / (1024.0 * 1024.0),
            execution_seconds: usage.execution_time_ms as f64 / 1000.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits() {
        let limits = ResourceLimits::default();
        assert!(limits.max_memory_bytes.is_some());
        assert!(limits.max_cpu_seconds.is_some());
    }

    #[test]
    fn test_check_limits() {
        let limits = ResourceLimits::default();

        // Within limits
        assert!(limits.check_memory(100 * 1024 * 1024)); // 100 MB
        assert!(limits.check_cpu_time(30));
        assert!(limits.check_output(1024 * 1024)); // 1 MB

        // Exceeding limits
        assert!(!limits.check_memory(1024 * 1024 * 1024)); // 1 GB
        assert!(!limits.check_cpu_time(120));
    }

    #[test]
    fn test_unlimited() {
        let limits = ResourceLimits::unlimited();

        // Everything passes with unlimited
        assert!(limits.check_memory(u64::MAX));
        assert!(limits.check_cpu_time(u64::MAX));
        assert!(limits.check_output(u64::MAX));
    }

    #[test]
    fn test_resource_usage_tracking() {
        let mut usage = ResourceUsage::new();

        usage.record_memory(100 * 1024 * 1024);
        usage.record_output(1024);
        usage.record_file_write(2048);
        usage.record_process_spawn();

        assert_eq!(usage.memory_bytes, 100 * 1024 * 1024);
        assert_eq!(usage.output_bytes, 1024);
        assert_eq!(usage.files_written, 1);
        assert_eq!(usage.total_write_bytes, 2048);
        assert_eq!(usage.processes_spawned, 1);
    }

    #[test]
    fn test_limit_violations() {
        let limits = ResourceLimits {
            max_memory_bytes: Some(100),
            max_output_bytes: Some(50),
            ..Default::default()
        };

        let mut usage = ResourceUsage::new();
        usage.memory_bytes = 200;
        usage.output_bytes = 100;

        let violations = usage.check_against_limits(&limits);
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn test_usage_summary() {
        let mut usage = ResourceUsage::new();
        usage.memory_bytes = 100 * 1024 * 1024; // 100 MB
        usage.cpu_time_ms = 5000; // 5 seconds
        usage.output_bytes = 10 * 1024; // 10 KB

        let summary = ResourceUsageSummary::from(&usage);
        assert!((summary.memory_mb - 100.0).abs() < 0.01);
        assert!((summary.cpu_seconds - 5.0).abs() < 0.01);
        assert!((summary.output_kb - 10.0).abs() < 0.01);
    }
}
