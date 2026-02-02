//! Progress data types for tool execution tracking

use serde::{Deserialize, Serialize};

/// Progress data for tool execution updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressData {
    /// Progress type
    #[serde(rename = "type")]
    pub progress_type: ProgressType,

    /// Associated tool use ID
    #[serde(rename = "toolUseId")]
    pub tool_use_id: String,

    /// Tool name
    #[serde(rename = "toolName")]
    pub tool_name: String,

    /// Elapsed time in seconds
    #[serde(rename = "elapsedSeconds")]
    pub elapsed_seconds: u64,

    /// Progress output data
    pub output: ProgressOutput,
}

impl ProgressData {
    /// Create new progress data
    pub fn new(
        progress_type: ProgressType,
        tool_use_id: impl Into<String>,
        tool_name: impl Into<String>,
    ) -> Self {
        Self {
            progress_type,
            tool_use_id: tool_use_id.into(),
            tool_name: tool_name.into(),
            elapsed_seconds: 0,
            output: ProgressOutput::default(),
        }
    }

    /// Set elapsed seconds
    pub fn with_elapsed_seconds(mut self, seconds: u64) -> Self {
        self.elapsed_seconds = seconds;
        self
    }

    /// Set output
    pub fn with_output(mut self, output: ProgressOutput) -> Self {
        self.output = output;
        self
    }
}

/// Progress type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressType {
    /// Bash command execution progress
    BashProgress,
    /// File read progress
    ReadProgress,
    /// File write progress
    WriteProgress,
    /// Search operation progress
    SearchProgress,
    /// Generic progress for other tools
    GenericProgress,
}

impl Default for ProgressType {
    fn default() -> Self {
        Self::GenericProgress
    }
}

/// Progress output data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProgressOutput {
    /// Total lines of output so far
    #[serde(rename = "totalLines")]
    pub total_lines: usize,

    /// Last line of output
    #[serde(rename = "lastLine")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_line: Option<String>,

    /// Whether output was truncated
    pub truncated: bool,

    /// Completion percentage (0-100) if calculable
    #[serde(rename = "percentComplete")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent_complete: Option<u8>,
}

impl ProgressOutput {
    /// Create new progress output
    pub fn new() -> Self {
        Self::default()
    }

    /// Set total lines
    pub fn with_total_lines(mut self, lines: usize) -> Self {
        self.total_lines = lines;
        self
    }

    /// Set last line
    pub fn with_last_line(mut self, line: impl Into<String>) -> Self {
        self.last_line = Some(line.into());
        self
    }

    /// Mark as truncated
    pub fn truncated(mut self) -> Self {
        self.truncated = true;
        self
    }

    /// Set completion percentage
    pub fn with_percent_complete(mut self, percent: u8) -> Self {
        self.percent_complete = Some(percent.min(100));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_data() {
        let progress = ProgressData::new(ProgressType::BashProgress, "tool-123", "bash")
            .with_elapsed_seconds(5)
            .with_output(
                ProgressOutput::new()
                    .with_total_lines(100)
                    .with_last_line("Building...")
                    .with_percent_complete(50),
            );

        assert_eq!(progress.progress_type, ProgressType::BashProgress);
        assert_eq!(progress.tool_use_id, "tool-123");
        assert_eq!(progress.elapsed_seconds, 5);
        assert_eq!(progress.output.total_lines, 100);
        assert_eq!(progress.output.percent_complete, Some(50));
    }

    #[test]
    fn test_progress_type_serialization() {
        let progress_type = ProgressType::BashProgress;
        let json = serde_json::to_string(&progress_type).unwrap();
        assert_eq!(json, "\"bash_progress\"");

        let deserialized: ProgressType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ProgressType::BashProgress);
    }

    #[test]
    fn test_progress_output_truncated() {
        let output = ProgressOutput::new().with_total_lines(1000).truncated();

        assert!(output.truncated);
        assert_eq!(output.total_lines, 1000);
    }
}
