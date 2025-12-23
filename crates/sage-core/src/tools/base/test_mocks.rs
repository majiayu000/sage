//! Mock tools for testing

#![cfg(test)]

use super::command_tool::CommandTool;
use super::error::ToolError;
use super::filesystem_tool::FileSystemTool;
use super::tool_trait::Tool;
use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::path::PathBuf;

// Mock tool for testing
pub(super) struct MockTool {
    pub name: String,
    pub description: String,
    pub working_dir: PathBuf,
}

impl MockTool {
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            name: "mock_tool".to_string(),
            description: "A mock tool for testing".to_string(),
            working_dir,
        }
    }
}

#[async_trait]
impl Tool for MockTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(self.name(), self.description(), vec![])
    }

    async fn execute(&self, _call: &ToolCall) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success("test-id", self.name(), "success"))
    }
}

impl FileSystemTool for MockTool {
    fn working_directory(&self) -> &std::path::Path {
        &self.working_dir
    }
}

pub(super) struct MockCommandTool {
    pub allowed: Vec<String>,
    pub working_dir: PathBuf,
}

impl MockCommandTool {
    pub fn new(allowed: Vec<String>, working_dir: PathBuf) -> Self {
        Self {
            allowed,
            working_dir,
        }
    }
}

#[async_trait]
impl Tool for MockCommandTool {
    fn name(&self) -> &str {
        "mock_command_tool"
    }

    fn description(&self) -> &str {
        "A mock command tool for testing"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(self.name(), self.description(), vec![])
    }

    async fn execute(&self, _call: &ToolCall) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success("test-id", self.name(), "success"))
    }
}

impl CommandTool for MockCommandTool {
    fn allowed_commands(&self) -> Vec<&str> {
        self.allowed.iter().map(|s| s.as_str()).collect()
    }

    fn command_working_directory(&self) -> &std::path::Path {
        &self.working_dir
    }
}
