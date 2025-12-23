//! Telemetry system for tracking tool usage and agent behavior

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Tool usage event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageEvent {
    /// Tool name
    pub tool_name: String,
    /// Timestamp when tool was called
    pub timestamp: u64,
    /// Duration of tool execution
    pub duration_ms: u64,
    /// Whether the tool call succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Agent type that called the tool
    pub agent_type: Option<String>,
}

/// Tool usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStats {
    /// Tool name
    pub tool_name: String,
    /// Total number of calls
    pub total_calls: u64,
    /// Successful calls
    pub successful_calls: u64,
    /// Failed calls
    pub failed_calls: u64,
    /// Average duration in milliseconds
    pub avg_duration_ms: f64,
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
}

/// Telemetry collector
#[derive(Debug)]
pub struct TelemetryCollector {
    events: Arc<RwLock<Vec<ToolUsageEvent>>>,
    start_time: Instant,
}

impl TelemetryCollector {
    /// Create a new telemetry collector
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
        }
    }

    /// Record a tool usage event
    pub fn record_tool_usage(
        &self,
        tool_name: impl Into<String>,
        duration: Duration,
        success: bool,
        error: Option<String>,
        agent_type: Option<String>,
    ) {
        let event = ToolUsageEvent {
            tool_name: tool_name.into(),
            timestamp: self.start_time.elapsed().as_secs(),
            duration_ms: duration.as_millis() as u64,
            success,
            error,
            agent_type,
        };

        let mut events = self.events.write();
        events.push(event);
    }

    /// Get all events
    pub fn get_events(&self) -> Vec<ToolUsageEvent> {
        let events = self.events.read();
        events.clone()
    }

    /// Get statistics for a specific tool
    pub fn get_tool_stats(&self, tool_name: &str) -> Option<ToolStats> {
        let events = self.events.read();
        let tool_events: Vec<_> = events.iter().filter(|e| e.tool_name == tool_name).collect();

        if tool_events.is_empty() {
            return None;
        }

        let total_calls = tool_events.len() as u64;
        let successful_calls = tool_events.iter().filter(|e| e.success).count() as u64;
        let failed_calls = total_calls - successful_calls;
        let total_duration_ms: u64 = tool_events.iter().map(|e| e.duration_ms).sum();
        let avg_duration_ms = total_duration_ms as f64 / total_calls as f64;

        Some(ToolStats {
            tool_name: tool_name.to_string(),
            total_calls,
            successful_calls,
            failed_calls,
            avg_duration_ms,
            total_duration_ms,
        })
    }

    /// Get statistics for all tools
    pub fn get_all_stats(&self) -> Vec<ToolStats> {
        let events = self.events.read();
        let mut tool_names: Vec<String> = events.iter().map(|e| e.tool_name.clone()).collect();
        tool_names.sort();
        tool_names.dedup();

        tool_names
            .iter()
            .filter_map(|name| self.get_tool_stats(name))
            .collect()
    }

    /// Get most used tools
    pub fn get_most_used_tools(&self, limit: usize) -> Vec<ToolStats> {
        let mut stats = self.get_all_stats();
        stats.sort_by(|a, b| b.total_calls.cmp(&a.total_calls));
        stats.truncate(limit);
        stats
    }

    /// Get slowest tools
    pub fn get_slowest_tools(&self, limit: usize) -> Vec<ToolStats> {
        let mut stats = self.get_all_stats();
        stats.sort_by(|a, b| {
            b.avg_duration_ms
                .partial_cmp(&a.avg_duration_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        stats.truncate(limit);
        stats
    }

    /// Get tools with highest failure rate
    pub fn get_tools_by_failure_rate(&self, limit: usize) -> Vec<(String, f64)> {
        let stats = self.get_all_stats();
        let mut failure_rates: Vec<(String, f64)> = stats
            .iter()
            .map(|s| {
                let rate = s.failed_calls as f64 / s.total_calls as f64;
                (s.tool_name.clone(), rate)
            })
            .collect();

        failure_rates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        failure_rates.truncate(limit);
        failure_rates
    }

    /// Get summary report
    pub fn get_summary(&self) -> TelemetrySummary {
        let events = self.events.read();
        let total_events = events.len();
        let successful_events = events.iter().filter(|e| e.success).count();
        let failed_events = total_events - successful_events;

        let total_duration_ms: u64 = events.iter().map(|e| e.duration_ms).sum();
        let avg_duration_ms = if total_events > 0 {
            total_duration_ms as f64 / total_events as f64
        } else {
            0.0
        };

        let unique_tools = {
            let mut tools: Vec<String> = events.iter().map(|e| e.tool_name.clone()).collect();
            tools.sort();
            tools.dedup();
            tools.len()
        };

        TelemetrySummary {
            total_events,
            successful_events,
            failed_events,
            unique_tools,
            total_duration_ms,
            avg_duration_ms,
            uptime_secs: self.start_time.elapsed().as_secs(),
        }
    }

    /// Clear all events
    pub fn clear(&self) {
        let mut events = self.events.write();
        events.clear();
    }
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Telemetry summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySummary {
    /// Total number of events
    pub total_events: usize,
    /// Successful events
    pub successful_events: usize,
    /// Failed events
    pub failed_events: usize,
    /// Number of unique tools used
    pub unique_tools: usize,
    /// Total duration of all tool calls
    pub total_duration_ms: u64,
    /// Average duration per tool call
    pub avg_duration_ms: f64,
    /// Uptime in seconds
    pub uptime_secs: u64,
}

/// Global telemetry collector
static GLOBAL_TELEMETRY: once_cell::sync::Lazy<TelemetryCollector> =
    once_cell::sync::Lazy::new(TelemetryCollector::new);

/// Get the global telemetry collector
pub fn global_telemetry() -> &'static TelemetryCollector {
    &GLOBAL_TELEMETRY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_collector() {
        let collector = TelemetryCollector::new();

        collector.record_tool_usage(
            "Read",
            Duration::from_millis(100),
            true,
            None,
            Some("GeneralPurpose".to_string()),
        );

        collector.record_tool_usage(
            "Read",
            Duration::from_millis(150),
            true,
            None,
            Some("GeneralPurpose".to_string()),
        );

        collector.record_tool_usage(
            "Write",
            Duration::from_millis(200),
            false,
            Some("Permission denied".to_string()),
            Some("GeneralPurpose".to_string()),
        );

        let events = collector.get_events();
        assert_eq!(events.len(), 3);

        let read_stats = collector.get_tool_stats("Read").unwrap();
        assert_eq!(read_stats.total_calls, 2);
        assert_eq!(read_stats.successful_calls, 2);
        assert_eq!(read_stats.failed_calls, 0);
        assert_eq!(read_stats.avg_duration_ms, 125.0);

        let write_stats = collector.get_tool_stats("Write").unwrap();
        assert_eq!(write_stats.total_calls, 1);
        assert_eq!(write_stats.successful_calls, 0);
        assert_eq!(write_stats.failed_calls, 1);

        let summary = collector.get_summary();
        assert_eq!(summary.total_events, 3);
        assert_eq!(summary.successful_events, 2);
        assert_eq!(summary.failed_events, 1);
        assert_eq!(summary.unique_tools, 2);
    }

    #[test]
    fn test_most_used_tools() {
        let collector = TelemetryCollector::new();

        for _ in 0..5 {
            collector.record_tool_usage("Read", Duration::from_millis(100), true, None, None);
        }

        for _ in 0..3 {
            collector.record_tool_usage("Write", Duration::from_millis(100), true, None, None);
        }

        for _ in 0..1 {
            collector.record_tool_usage("Edit", Duration::from_millis(100), true, None, None);
        }

        let most_used = collector.get_most_used_tools(2);
        assert_eq!(most_used.len(), 2);
        assert_eq!(most_used[0].tool_name, "Read");
        assert_eq!(most_used[0].total_calls, 5);
        assert_eq!(most_used[1].tool_name, "Write");
        assert_eq!(most_used[1].total_calls, 3);
    }

    #[test]
    fn test_failure_rate() {
        let collector = TelemetryCollector::new();

        // Read: 100% success
        collector.record_tool_usage("Read", Duration::from_millis(100), true, None, None);
        collector.record_tool_usage("Read", Duration::from_millis(100), true, None, None);

        // Write: 50% failure
        collector.record_tool_usage("Write", Duration::from_millis(100), true, None, None);
        collector.record_tool_usage("Write", Duration::from_millis(100), false, None, None);

        // Edit: 100% failure
        collector.record_tool_usage("Edit", Duration::from_millis(100), false, None, None);

        let failure_rates = collector.get_tools_by_failure_rate(3);
        assert_eq!(failure_rates[0].0, "Edit");
        assert_eq!(failure_rates[0].1, 1.0);
        assert_eq!(failure_rates[1].0, "Write");
        assert_eq!(failure_rates[1].1, 0.5);
        assert_eq!(failure_rates[2].0, "Read");
        assert_eq!(failure_rates[2].1, 0.0);
    }
}
