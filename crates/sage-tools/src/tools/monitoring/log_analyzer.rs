//! Log Analyzer Tool
//!
//! This tool provides log analysis capabilities including:
//! - Multi-format log parsing
//! - Error detection and anomaly analysis
//! - Log aggregation and filtering
//! - Performance metrics extraction

use async_trait::async_trait;
use regex::Regex;
use std::collections::HashMap;
use tokio::fs;
use tracing::info;

use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};

/// Log analyzer tool
#[derive(Debug, Clone)]
pub struct LogAnalyzerTool {
    name: String,
    description: String,
}

impl LogAnalyzerTool {
    /// Create a new log analyzer tool
    pub fn new() -> Self {
        Self {
            name: "log_analyzer".to_string(),
            description: "Log analysis tool for parsing, filtering, and analyzing log files with error detection and metrics extraction".to_string(),
        }
    }

    /// Analyze log file
    async fn analyze_logs(
        &self,
        file_path: &str,
        pattern: Option<&str>,
        lines: Option<usize>,
    ) -> Result<String, ToolError> {
        let content = fs::read_to_string(file_path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read log file: {}", e)))?;

        let log_lines: Vec<&str> = content.lines().collect();
        let total_lines = log_lines.len();

        // Take only the requested number of lines (from the end)
        let analyzed_lines = if let Some(limit) = lines {
            if limit < total_lines {
                &log_lines[total_lines - limit..]
            } else {
                &log_lines
            }
        } else {
            &log_lines
        };

        let mut result = String::new();
        result.push_str(&format!("Log Analysis Report for: {}\n", file_path));
        result.push_str(&format!("Total lines: {}\n", total_lines));
        result.push_str(&format!("Analyzed lines: {}\n\n", analyzed_lines.len()));

        // Error detection
        let mut error_count = 0;
        let mut warning_count = 0;
        let mut errors = Vec::new();

        for (i, line) in analyzed_lines.iter().enumerate() {
            let line_lower = line.to_lowercase();
            if line_lower.contains("error")
                || line_lower.contains("fatal")
                || line_lower.contains("exception")
            {
                error_count += 1;
                if errors.len() < 10 {
                    // Limit to first 10 errors
                    errors.push(format!(
                        "Line {}: {}",
                        total_lines - analyzed_lines.len() + i + 1,
                        line
                    ));
                }
            } else if line_lower.contains("warn") || line_lower.contains("warning") {
                warning_count += 1;
            }
        }

        result.push_str(&format!("Errors found: {}\n", error_count));
        result.push_str(&format!("Warnings found: {}\n\n", warning_count));

        if !errors.is_empty() {
            result.push_str("Recent Errors:\n");
            for error in &errors {
                result.push_str(&format!("  {}\n", error));
            }
            result.push_str("\n");
        }

        // Pattern matching if provided
        if let Some(pattern_str) = pattern {
            if let Ok(regex) = Regex::new(pattern_str) {
                let matches: Vec<_> = analyzed_lines
                    .iter()
                    .enumerate()
                    .filter_map(|(i, line)| {
                        if regex.is_match(line) {
                            Some(format!(
                                "Line {}: {}",
                                total_lines - analyzed_lines.len() + i + 1,
                                line
                            ))
                        } else {
                            None
                        }
                    })
                    .take(20) // Limit to 20 matches
                    .collect();

                result.push_str(&format!(
                    "Pattern '{}' matches: {}\n",
                    pattern_str,
                    matches.len()
                ));
                if !matches.is_empty() {
                    result.push_str("Sample matches:\n");
                    for match_line in matches {
                        result.push_str(&format!("  {}\n", match_line));
                    }
                }
            } else {
                result.push_str(&format!("Invalid regex pattern: {}\n", pattern_str));
            }
        }

        // Basic statistics
        let avg_line_length = analyzed_lines.iter().map(|line| line.len()).sum::<usize>() as f64
            / analyzed_lines.len() as f64;

        result.push_str(&format!("\nStatistics:\n"));
        result.push_str(&format!(
            "Average line length: {:.1} characters\n",
            avg_line_length
        ));

        if error_count > 0 {
            result.push_str(&format!(
                "Error rate: {:.2}%\n",
                (error_count as f64 / analyzed_lines.len() as f64) * 100.0
            ));
        }

        Ok(result)
    }

    /// Extract metrics from logs
    async fn extract_metrics(&self, file_path: &str) -> Result<String, ToolError> {
        let content = fs::read_to_string(file_path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read log file: {}", e)))?;

        let lines: Vec<&str> = content.lines().collect();

        // Look for common log patterns and extract metrics
        let mut response_times: Vec<f64> = Vec::new();
        let mut status_codes = HashMap::new();

        // Common patterns for different log formats
        let apache_pattern =
            Regex::new(r#"(\d+\.\d+\.\d+\.\d+).*?\[([^\]]+)\].*?"([^"]*)".*?(\d{3})\s+(\d+)"#)
                .unwrap();
        let response_time_pattern = Regex::new(r"(\d+(?:\.\d+)?)\s*ms").unwrap();

        for line in lines {
            // Extract status codes (HTTP logs)
            if let Some(caps) = apache_pattern.captures(line) {
                if let Some(status) = caps.get(4) {
                    *status_codes.entry(status.as_str().to_string()).or_insert(0) += 1;
                }
            }

            // Extract response times
            if let Some(caps) = response_time_pattern.captures(line) {
                if let Ok(time) = caps[1].parse::<f64>() {
                    response_times.push(time);
                }
            }
        }

        let mut result = String::new();
        result.push_str(&format!("Metrics extracted from: {}\n\n", file_path));

        if !status_codes.is_empty() {
            result.push_str("HTTP Status Codes:\n");
            for (status, count) in status_codes {
                result.push_str(&format!("  {}: {}\n", status, count));
            }
            result.push_str("\n");
        }

        if !response_times.is_empty() {
            let avg_time = response_times.iter().sum::<f64>() / response_times.len() as f64;
            let min_time = response_times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max_time = response_times
                .iter()
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));

            result.push_str("Response Times:\n");
            result.push_str(&format!("  Average: {:.2} ms\n", avg_time));
            result.push_str(&format!("  Min: {:.2} ms\n", min_time));
            result.push_str(&format!("  Max: {:.2} ms\n", max_time));
            result.push_str(&format!("  Total requests: {}\n", response_times.len()));
        }

        Ok(result)
    }

    /// Monitor log file for new entries
    async fn tail_logs(&self, file_path: &str, lines: usize) -> Result<String, ToolError> {
        let content = fs::read_to_string(file_path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read log file: {}", e)))?;

        let log_lines: Vec<&str> = content.lines().collect();
        let total_lines = log_lines.len();

        let tail_lines = if lines < total_lines {
            &log_lines[total_lines - lines..]
        } else {
            &log_lines
        };

        let mut result = String::new();
        result.push_str(&format!(
            "Last {} lines from {}:\n\n",
            tail_lines.len(),
            file_path
        ));

        for (i, line) in tail_lines.iter().enumerate() {
            result.push_str(&format!(
                "{}: {}\n",
                total_lines - tail_lines.len() + i + 1,
                line
            ));
        }

        Ok(result)
    }
}

impl Default for LogAnalyzerTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for LogAnalyzerTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string("command", "Log analysis command (analyze, metrics, tail)"),
                ToolParameter::string("file_path", "Path to the log file"),
                ToolParameter::optional_string("pattern", "Regex pattern to search for"),
                ToolParameter::number("lines", "Number of lines to analyze").optional(),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let command = call.get_string("command").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'command' parameter".to_string())
        })?;

        let file_path = call.get_string("file_path").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'file_path' parameter".to_string())
        })?;

        info!(
            "Executing log analysis command: {} on file: {}",
            command, file_path
        );

        let result = match command.as_str() {
            "analyze" => {
                let pattern = call.get_string("pattern");
                let lines = call.get_number("lines").map(|n| n as usize);
                self.analyze_logs(&file_path, pattern.as_deref(), lines)
                    .await?
            }
            "metrics" => self.extract_metrics(&file_path).await?,
            "tail" => {
                let lines = call.get_number("lines").unwrap_or(50.0) as usize;
                self.tail_logs(&file_path, lines).await?
            }
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown command: {}",
                    command
                )));
            }
        };

        Ok(ToolResult::success(call.id.clone(), self.name(), result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_log_analyzer_creation() {
        let tool = LogAnalyzerTool::new();
        assert_eq!(tool.name(), "log_analyzer");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_log_analyzer_schema() {
        let tool = LogAnalyzerTool::new();
        let schema = tool.schema();

        assert_eq!(schema.name, "log_analyzer");
        assert!(!schema.description.is_empty());
    }
}
