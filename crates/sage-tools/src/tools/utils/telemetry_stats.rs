//! TelemetryStats tool - View tool usage statistics
//!
//! Provides access to telemetry data about tool usage, including
//! call counts, success rates, and execution times.

use async_trait::async_trait;
use sage_core::telemetry::global_telemetry;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use serde_json::json;

/// TelemetryStats tool - View tool usage statistics
pub struct TelemetryStatsTool;

impl Default for TelemetryStatsTool {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryStatsTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for TelemetryStatsTool {
    fn name(&self) -> &str {
        "TelemetryStats"
    }

    fn description(&self) -> &str {
        "View tool usage statistics and telemetry data. Shows call counts, success rates, and execution times for all tools used in the current session."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "view": {
                        "type": "string",
                        "description": "What to view: 'summary' (default), 'all', 'most_used', 'slowest', 'failures'",
                        "enum": ["summary", "all", "most_used", "slowest", "failures"],
                        "default": "summary"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Number of items to show for most_used/slowest/failures views (default: 5)",
                        "default": 5
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let view = call
            .arguments
            .get("view")
            .and_then(|v| v.as_str())
            .unwrap_or("summary");

        let limit = call
            .arguments
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as usize;

        let telemetry = global_telemetry();
        let mut output = String::new();

        match view {
            "summary" => {
                let summary = telemetry.get_summary();
                output.push_str("## Telemetry Summary\n\n");
                output.push_str(&format!(
                    "| Metric | Value |\n|--------|-------|\n\
                     | Total Tool Calls | {} |\n\
                     | Successful | {} |\n\
                     | Failed | {} |\n\
                     | Success Rate | {:.1}% |\n\
                     | Unique Tools | {} |\n\
                     | Total Duration | {:.2}s |\n\
                     | Avg Duration | {:.2}ms |\n\
                     | Session Uptime | {}s |\n",
                    summary.total_events,
                    summary.successful_events,
                    summary.failed_events,
                    if summary.total_events > 0 {
                        (summary.successful_events as f64 / summary.total_events as f64) * 100.0
                    } else {
                        0.0
                    },
                    summary.unique_tools,
                    summary.total_duration_ms as f64 / 1000.0,
                    summary.avg_duration_ms,
                    summary.uptime_secs,
                ));
            }
            "all" => {
                let stats = telemetry.get_all_stats();
                if stats.is_empty() {
                    output.push_str("No tool usage data yet.\n");
                } else {
                    output.push_str("## All Tool Statistics\n\n");
                    output.push_str("| Tool | Calls | Success | Failed | Avg Time |\n");
                    output.push_str("|------|-------|---------|--------|----------|\n");
                    for stat in stats {
                        output.push_str(&format!(
                            "| {} | {} | {} | {} | {:.2}ms |\n",
                            stat.tool_name,
                            stat.total_calls,
                            stat.successful_calls,
                            stat.failed_calls,
                            stat.avg_duration_ms
                        ));
                    }
                }
            }
            "most_used" => {
                let stats = telemetry.get_most_used_tools(limit);
                if stats.is_empty() {
                    output.push_str("No tool usage data yet.\n");
                } else {
                    output.push_str(&format!("## Top {} Most Used Tools\n\n", limit));
                    output.push_str("| Rank | Tool | Calls |\n");
                    output.push_str("|------|------|-------|\n");
                    for (i, stat) in stats.iter().enumerate() {
                        output.push_str(&format!(
                            "| {} | {} | {} |\n",
                            i + 1,
                            stat.tool_name,
                            stat.total_calls
                        ));
                    }
                }
            }
            "slowest" => {
                let stats = telemetry.get_slowest_tools(limit);
                if stats.is_empty() {
                    output.push_str("No tool usage data yet.\n");
                } else {
                    output.push_str(&format!("## Top {} Slowest Tools\n\n", limit));
                    output.push_str("| Rank | Tool | Avg Time | Total Time |\n");
                    output.push_str("|------|------|----------|------------|\n");
                    for (i, stat) in stats.iter().enumerate() {
                        output.push_str(&format!(
                            "| {} | {} | {:.2}ms | {:.2}s |\n",
                            i + 1,
                            stat.tool_name,
                            stat.avg_duration_ms,
                            stat.total_duration_ms as f64 / 1000.0
                        ));
                    }
                }
            }
            "failures" => {
                let failure_rates = telemetry.get_tools_by_failure_rate(limit);
                if failure_rates.is_empty() {
                    output.push_str("No tool usage data yet.\n");
                } else {
                    output.push_str(&format!("## Top {} Tools by Failure Rate\n\n", limit));
                    output.push_str("| Rank | Tool | Failure Rate |\n");
                    output.push_str("|------|------|-------------|\n");
                    for (i, (name, rate)) in failure_rates.iter().enumerate() {
                        output.push_str(&format!(
                            "| {} | {} | {:.1}% |\n",
                            i + 1,
                            name,
                            rate * 100.0
                        ));
                    }
                }
            }
            _ => {
                output.push_str("Unknown view type. Use: summary, all, most_used, slowest, failures\n");
            }
        }

        let summary = telemetry.get_summary();
        let mut result = ToolResult::success(&call.id, self.name(), output);
        result = result
            .with_metadata("total_events", json!(summary.total_events))
            .with_metadata("unique_tools", json!(summary.unique_tools))
            .with_metadata("uptime_secs", json!(summary.uptime_secs));

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_telemetry_stats_summary() {
        let tool = TelemetryStatsTool::new();

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "TelemetryStats".to_string(),
            arguments: HashMap::new(),
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("Telemetry Summary"));
    }

    #[tokio::test]
    async fn test_telemetry_stats_all() {
        let tool = TelemetryStatsTool::new();

        let mut args = HashMap::new();
        args.insert("view".to_string(), json!("all"));

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "TelemetryStats".to_string(),
            arguments: args,
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
    }
}
