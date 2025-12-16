//! Enter plan mode tool

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};

/// Tool for entering plan mode to design implementation approaches
pub struct EnterPlanModeTool;

impl EnterPlanModeTool {
    /// Create a new enter plan mode tool
    pub fn new() -> Self {
        Self
    }
}

impl Default for EnterPlanModeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for EnterPlanModeTool {
    fn name(&self) -> &str {
        "enter_plan_mode"
    }

    fn description(&self) -> &str {
        "Enter plan mode to design and document implementation approaches. Use this tool when you need to create a detailed plan before implementing features or solving complex problems. In plan mode, focus on architecture, design decisions, and step-by-step implementation strategies."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                // No parameters required - empty parameters object
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let confirmation_message = r#"
╔══════════════════════════════════════════════════════════════╗
║                  PLAN MODE ACTIVATED                         ║
╚══════════════════════════════════════════════════════════════╝

You are now in PLAN MODE. Focus on:

✓ Analyzing requirements and constraints
✓ Designing architecture and system components
✓ Identifying key design decisions and trade-offs
✓ Creating step-by-step implementation strategies
✓ Documenting dependencies and prerequisites
✓ Planning testing and validation approaches

Take your time to think through the problem thoroughly before
implementing. When ready to exit plan mode, use the exit_plan_mode
tool to transition back to implementation mode.
"#;

        Ok(ToolResult::success(
            &call.id,
            self.name(),
            confirmation_message.trim(),
        ))
    }

    fn validate(&self, _call: &ToolCall) -> Result<(), ToolError> {
        // No parameters to validate - always valid
        Ok(())
    }

    fn max_execution_time(&self) -> Option<u64> {
        Some(5) // 5 seconds - this is a very lightweight operation
    }

    fn supports_parallel_execution(&self) -> bool {
        true // Mode transitions don't interfere with other operations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

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
    async fn test_enter_plan_mode_basic() {
        let tool = EnterPlanModeTool::new();
        let call = create_tool_call("test-1", "enter_plan_mode", json!({}));

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("PLAN MODE ACTIVATED"));
        assert!(output.contains("Analyzing requirements"));
        assert!(output.contains("exit_plan_mode"));
    }

    #[tokio::test]
    async fn test_enter_plan_mode_validation() {
        let tool = EnterPlanModeTool::new();
        let call = create_tool_call("test-2", "enter_plan_mode", json!({}));

        // Should always validate successfully
        let validation_result = tool.validate(&call);
        assert!(validation_result.is_ok());
    }

    #[tokio::test]
    async fn test_enter_plan_mode_with_extra_params() {
        let tool = EnterPlanModeTool::new();
        // Extra parameters should be ignored
        let call = create_tool_call(
            "test-3",
            "enter_plan_mode",
            json!({
                "extra_param": "should be ignored"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("PLAN MODE ACTIVATED"));
    }

    #[test]
    fn test_enter_plan_mode_schema() {
        let tool = EnterPlanModeTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "enter_plan_mode");
        assert!(!schema.description.is_empty());
    }

    #[test]
    fn test_enter_plan_mode_max_execution_time() {
        let tool = EnterPlanModeTool::new();
        assert_eq!(tool.max_execution_time(), Some(5));
    }

    #[test]
    fn test_enter_plan_mode_supports_parallel_execution() {
        let tool = EnterPlanModeTool::new();
        assert!(tool.supports_parallel_execution());
    }

    #[test]
    fn test_enter_plan_mode_name() {
        let tool = EnterPlanModeTool::new();
        assert_eq!(tool.name(), "enter_plan_mode");
    }

    #[test]
    fn test_enter_plan_mode_description() {
        let tool = EnterPlanModeTool::new();
        let desc = tool.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("plan mode"));
    }
}
