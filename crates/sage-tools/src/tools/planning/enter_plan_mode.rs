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
        "Enter QUICK plan mode for brief analysis before coding. Use sparingly - most tasks should start with code immediately. \
         Plan mode is ONLY for complex multi-component tasks. Keep planning under 2 minutes, then exit and START WRITING CODE. \
         Do NOT use plan mode for simple features or bug fixes."
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
║              QUICK PLAN MODE - 2 MIN MAX                      ║
╚══════════════════════════════════════════════════════════════╝

⚠️  CRITICAL: This is for QUICK planning only. Do NOT:
  ✗ Spend more than 2 minutes planning
  ✗ Write detailed documentation
  ✗ Call task_done after planning without writing code

✓ Quickly identify key components (30 seconds)
✓ Note any critical dependencies (30 seconds)
✓ EXIT PLAN MODE and START CODING (immediately!)

REMEMBER: Plans without code are WORTHLESS.
Your job is to WRITE CODE, not documentation.

Use exit_plan_mode NOW and begin implementation.
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
        assert!(output.contains("QUICK PLAN MODE"));
        assert!(output.contains("WRITE CODE"));
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
        assert!(output.contains("QUICK PLAN MODE"));
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
