//! Output formatting for grep results

use sage_core::tools::base::ToolError;

/// Output mode for grep results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrepOutputMode {
    /// Show matching lines with content
    Content,
    /// Show only file paths with matches
    FilesWithMatches,
    /// Show match counts per file
    Count,
}

impl GrepOutputMode {
    /// Parse from string with ToolError
    #[allow(clippy::should_implement_trait)] // Uses ToolError instead of ParseError
    pub fn from_str(s: &str) -> Result<Self, ToolError> {
        match s {
            "content" => Ok(Self::Content),
            "files_with_matches" => Ok(Self::FilesWithMatches),
            "count" => Ok(Self::Count),
            _ => Err(ToolError::InvalidArguments(format!(
                "Invalid output_mode: {}. Use 'content', 'files_with_matches', or 'count'",
                s
            ))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Content => "content",
            Self::FilesWithMatches => "files_with_matches",
            Self::Count => "count",
        }
    }
}

impl Default for GrepOutputMode {
    fn default() -> Self {
        Self::FilesWithMatches
    }
}
