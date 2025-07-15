//! Sequential thinking tool for agent reasoning

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};

/// Tool for sequential thinking and reasoning
pub struct SequentialThinkingTool;

impl SequentialThinkingTool {
    /// Create a new sequential thinking tool
    pub fn new() -> Self {
        Self
    }
}

impl Default for SequentialThinkingTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SequentialThinkingTool {
    fn name(&self) -> &str {
        "sequentialthinking"
    }

    fn description(&self) -> &str {
        "Use this tool to think through problems step by step. Provide your reasoning process and analysis."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![ToolParameter::string(
                "thinking",
                "Your step-by-step thinking process and analysis",
            )],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let thinking = call
            .get_string("thinking")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'thinking' parameter".to_string()))?;

        if thinking.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "Thinking content cannot be empty".to_string(),
            ));
        }

        // Process the thinking content
        let processed_thinking = self.process_thinking(&thinking);

        Ok(ToolResult::success(
            &call.id,
            self.name(),
            format!("Sequential thinking recorded:\n{}", processed_thinking),
        ))
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let thinking = call
            .get_string("thinking")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'thinking' parameter".to_string()))?;

        if thinking.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "Thinking content cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    fn max_execution_time(&self) -> Option<u64> {
        Some(10) // 10 seconds - this is a lightweight operation
    }

    fn supports_parallel_execution(&self) -> bool {
        true // Thinking operations don't interfere with each other
    }
}

impl SequentialThinkingTool {
    /// Process and format the thinking content
    fn process_thinking(&self, thinking: &str) -> String {
        let lines: Vec<&str> = thinking.lines().collect();
        let mut processed = Vec::new();
        let mut step_number = 1;

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                processed.push(String::new());
                continue;
            }

            // Check if line starts with a step indicator
            if trimmed.starts_with("Step ") || trimmed.starts_with(&format!("{}.", step_number)) {
                processed.push(format!("🤔 {}", trimmed));
                step_number += 1;
            } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                // Bullet points
                processed.push(format!("  • {}", &trimmed[2..]));
            } else if trimmed.starts_with("Question:") || trimmed.starts_with("Q:") {
                processed.push(format!("❓ {}", trimmed));
            } else if trimmed.starts_with("Answer:") || trimmed.starts_with("A:") {
                processed.push(format!("💡 {}", trimmed));
            } else if trimmed.starts_with("Conclusion:") || trimmed.starts_with("Summary:") {
                processed.push(format!("📝 {}", trimmed));
            } else if trimmed.starts_with("Problem:") || trimmed.starts_with("Issue:") {
                processed.push(format!("⚠️  {}", trimmed));
            } else if trimmed.starts_with("Solution:") || trimmed.starts_with("Approach:") {
                processed.push(format!("✅ {}", trimmed));
            } else {
                // Regular text
                processed.push(format!("   {}", trimmed));
            }
        }

        processed.join("\n")
    }
}
