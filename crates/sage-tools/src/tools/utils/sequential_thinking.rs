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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use serde_json::json;

    fn create_tool_call(id: &str, name: &str, args: serde_json::Value) -> ToolCall {
        let arguments = if let serde_json::Value::Object(map) = args {
            map.into_iter().collect()
        } else {
            HashMap::new()
        };

        ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments,
            call_id: None,
        }
    }

    #[tokio::test]
    async fn test_sequential_thinking_basic() {
        let tool = SequentialThinkingTool::new();
        let call = create_tool_call("test-1", "sequentialthinking", json!({
            "thinking": "Let me think about this problem step by step:\n1. First, I need to understand the requirements\n2. Then, I'll analyze the constraints\n3. Finally, I'll propose a solution"
        }));

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("Sequential thinking recorded"));
        assert!(output.contains("First, I need to understand"));
        assert!(output.contains("Then, I'll analyze"));
        assert!(output.contains("Finally, I'll propose"));
    }

    #[tokio::test]
    async fn test_sequential_thinking_with_numbered_steps() {
        let tool = SequentialThinkingTool::new();
        let call = create_tool_call("test-2", "sequentialthinking", json!({
            "thinking": "1. Analyze the problem\n2. Consider alternatives\n3. Choose the best approach"
        }));

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        // Implementation uses 🤔 emoji for numbered steps
        assert!(output.contains("Analyze the problem"));
        assert!(output.contains("Consider alternatives"));
        assert!(output.contains("Choose the best approach"));
    }

    #[tokio::test]
    async fn test_sequential_thinking_with_bullet_points() {
        let tool = SequentialThinkingTool::new();
        let call = create_tool_call("test-3", "sequentialthinking", json!({
            "thinking": "* First consideration\n* Second point\n* Third aspect"
        }));

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        // Implementation uses • for bullet points
        assert!(output.contains("First consideration"));
        assert!(output.contains("Second point"));
        assert!(output.contains("Third aspect"));
    }

    #[tokio::test]
    async fn test_sequential_thinking_with_dashes() {
        let tool = SequentialThinkingTool::new();
        let call = create_tool_call("test-4", "sequentialthinking", json!({
            "thinking": "- Problem identification\n- Solution brainstorming\n- Implementation planning"
        }));

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        // Implementation uses • for dash bullet points
        assert!(output.contains("Problem identification"));
        assert!(output.contains("Solution brainstorming"));
        assert!(output.contains("Implementation planning"));
    }

    #[tokio::test]
    async fn test_sequential_thinking_empty_input() {
        let tool = SequentialThinkingTool::new();
        let call = create_tool_call("test-5", "sequentialthinking", json!({
            "thinking": ""
        }));

        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Thinking content cannot be empty"));
    }

    #[tokio::test]
    async fn test_sequential_thinking_whitespace_only() {
        let tool = SequentialThinkingTool::new();
        let call = create_tool_call("test-6", "sequentialthinking", json!({
            "thinking": "   \n\t  \n   "
        }));

        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Thinking content cannot be empty"));
    }

    #[tokio::test]
    async fn test_sequential_thinking_missing_parameter() {
        let tool = SequentialThinkingTool::new();
        let call = create_tool_call("test-7", "sequentialthinking", json!({}));

        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Missing 'thinking' parameter"));
    }

    #[tokio::test]
    async fn test_sequential_thinking_mixed_formatting() {
        let tool = SequentialThinkingTool::new();
        let call = create_tool_call("test-8", "sequentialthinking", json!({
            "thinking": "Let me analyze this:\n1. First step\n* Important point\n- Another consideration\nRegular text here"
        }));

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("First step"));
        assert!(output.contains("Important point"));
        assert!(output.contains("Another consideration"));
        assert!(output.contains("Regular text here"));
    }

    #[test]
    fn test_sequential_thinking_schema() {
        let tool = SequentialThinkingTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "sequentialthinking");
        assert!(!schema.description.is_empty());
    }
}
