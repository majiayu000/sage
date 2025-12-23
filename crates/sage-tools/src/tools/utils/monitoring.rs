//! Tool monitoring and metrics system

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Metrics for a specific tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetrics {
    /// Tool name
    pub tool_name: String,
    /// Total number of executions
    pub execution_count: u64,
    /// Total execution time in milliseconds
    pub total_execution_time_ms: u64,
    /// Average execution time in milliseconds
    pub average_execution_time_ms: f64,
    /// Number of successful executions
    pub success_count: u64,
    /// Number of failed executions
    pub error_count: u64,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    /// Last execution timestamp
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
    /// Most common error types
    pub error_types: HashMap<String, u64>,
}

impl ToolMetrics {
    pub fn new(tool_name: String) -> Self {
        Self {
            tool_name,
            execution_count: 0,
            total_execution_time_ms: 0,
            average_execution_time_ms: 0.0,
            success_count: 0,
            error_count: 0,
            success_rate: 0.0,
            last_execution: None,
            error_types: HashMap::new(),
        }
    }

    /// Update metrics after a successful execution
    pub fn record_success(&mut self, execution_time: Duration) {
        self.execution_count += 1;
        self.success_count += 1;
        let execution_time_ms = execution_time.as_millis() as u64;
        self.total_execution_time_ms += execution_time_ms;
        self.average_execution_time_ms =
            self.total_execution_time_ms as f64 / self.execution_count as f64;
        self.success_rate = self.success_count as f64 / self.execution_count as f64;
        self.last_execution = Some(chrono::Utc::now());
    }

    /// Update metrics after a failed execution
    pub fn record_error(&mut self, execution_time: Duration, error_type: String) {
        self.execution_count += 1;
        self.error_count += 1;
        let execution_time_ms = execution_time.as_millis() as u64;
        self.total_execution_time_ms += execution_time_ms;
        self.average_execution_time_ms =
            self.total_execution_time_ms as f64 / self.execution_count as f64;
        self.success_rate = self.success_count as f64 / self.execution_count as f64;
        self.last_execution = Some(chrono::Utc::now());

        // Track error types
        *self.error_types.entry(error_type).or_insert(0) += 1;
    }
}

/// Tool monitoring system
#[derive(Debug)]
pub struct ToolMonitor {
    metrics: Arc<Mutex<HashMap<String, ToolMetrics>>>,
    start_time: Instant,
}

impl ToolMonitor {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    /// Record a successful tool execution
    pub fn record_success(&self, tool_name: &str, execution_time: Duration) {
        let mut metrics = self.metrics.lock();
        let tool_metrics = metrics
            .entry(tool_name.to_string())
            .or_insert_with(|| ToolMetrics::new(tool_name.to_string()));
        tool_metrics.record_success(execution_time);
    }

    /// Record a failed tool execution
    pub fn record_error(&self, tool_name: &str, execution_time: Duration, error_type: String) {
        let mut metrics = self.metrics.lock();
        let tool_metrics = metrics
            .entry(tool_name.to_string())
            .or_insert_with(|| ToolMetrics::new(tool_name.to_string()));
        tool_metrics.record_error(execution_time, error_type);
    }

    /// Get metrics for a specific tool
    pub fn get_tool_metrics(&self, tool_name: &str) -> Option<ToolMetrics> {
        let metrics = self.metrics.lock();
        metrics.get(tool_name).cloned()
    }

    /// Get metrics for all tools
    pub fn get_all_metrics(&self) -> HashMap<String, ToolMetrics> {
        let metrics = self.metrics.lock();
        metrics.clone()
    }

    /// Get system uptime
    pub fn get_uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Generate a monitoring report
    pub fn generate_report(&self) -> MonitoringReport {
        let metrics = self.metrics.lock();
        let total_executions: u64 = metrics.values().map(|m| m.execution_count).sum();
        let total_successes: u64 = metrics.values().map(|m| m.success_count).sum();
        let total_errors: u64 = metrics.values().map(|m| m.error_count).sum();
        let overall_success_rate = if total_executions > 0 {
            total_successes as f64 / total_executions as f64
        } else {
            0.0
        };

        let mut tool_metrics: Vec<ToolMetrics> = metrics.values().cloned().collect();
        tool_metrics.sort_by(|a, b| b.execution_count.cmp(&a.execution_count));

        MonitoringReport {
            uptime: self.get_uptime(),
            total_executions,
            total_successes,
            total_errors,
            overall_success_rate,
            tool_metrics,
            generated_at: chrono::Utc::now(),
        }
    }

    /// Reset all metrics
    pub fn reset_metrics(&self) {
        let mut metrics = self.metrics.lock();
        metrics.clear();
    }
}

impl Default for ToolMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Monitoring report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringReport {
    /// System uptime
    pub uptime: Duration,
    /// Total number of tool executions
    pub total_executions: u64,
    /// Total number of successful executions
    pub total_successes: u64,
    /// Total number of failed executions
    pub total_errors: u64,
    /// Overall success rate
    pub overall_success_rate: f64,
    /// Metrics for each tool
    pub tool_metrics: Vec<ToolMetrics>,
    /// Report generation timestamp
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

impl MonitoringReport {
    /// Format the report as a human-readable string
    pub fn format(&self) -> String {
        let mut output = String::new();

        output.push_str("# Tool Monitoring Report\n\n");
        output.push_str(&format!(
            "Generated at: {}\n",
            self.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        output.push_str(&format!("System uptime: {:?}\n\n", self.uptime));

        output.push_str("## Overall Statistics\n\n");
        output.push_str(&format!("- Total executions: {}\n", self.total_executions));
        output.push_str(&format!(
            "- Successful executions: {}\n",
            self.total_successes
        ));
        output.push_str(&format!("- Failed executions: {}\n", self.total_errors));
        output.push_str(&format!(
            "- Overall success rate: {:.2}%\n\n",
            self.overall_success_rate * 100.0
        ));

        if !self.tool_metrics.is_empty() {
            output.push_str("## Tool-Specific Metrics\n\n");
            for metrics in &self.tool_metrics {
                output.push_str(&format!("### {}\n\n", metrics.tool_name));
                output.push_str(&format!("- Executions: {}\n", metrics.execution_count));
                output.push_str(&format!(
                    "- Success rate: {:.2}%\n",
                    metrics.success_rate * 100.0
                ));
                output.push_str(&format!(
                    "- Average execution time: {:.2}ms\n",
                    metrics.average_execution_time_ms
                ));
                if let Some(last_exec) = &metrics.last_execution {
                    output.push_str(&format!(
                        "- Last execution: {}\n",
                        last_exec.format("%Y-%m-%d %H:%M:%S UTC")
                    ));
                }
                if !metrics.error_types.is_empty() {
                    output.push_str("- Common errors:\n");
                    for (error_type, count) in &metrics.error_types {
                        output.push_str(&format!("  - {}: {} times\n", error_type, count));
                    }
                }
                output.push('\n');
            }
        }

        output
    }
}

// Global tool monitor instance
pub static GLOBAL_MONITOR: Lazy<ToolMonitor> = Lazy::new(ToolMonitor::new);

/// Helper function to record a successful execution
pub fn record_success(tool_name: &str, execution_time: Duration) {
    GLOBAL_MONITOR.record_success(tool_name, execution_time);
}

/// Helper function to record a failed execution
pub fn record_error(tool_name: &str, execution_time: Duration, error_type: String) {
    GLOBAL_MONITOR.record_error(tool_name, execution_time, error_type);
}

/// Helper function to get monitoring report
pub fn get_monitoring_report() -> MonitoringReport {
    GLOBAL_MONITOR.generate_report()
}
