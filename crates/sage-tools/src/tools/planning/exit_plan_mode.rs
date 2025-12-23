//! Exit plan mode tool

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};

/// Tool for exiting plan mode after creating a plan
pub struct ExitPlanModeTool;

impl ExitPlanModeTool {
    /// Create a new exit plan mode tool
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExitPlanModeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ExitPlanModeTool {
    fn name(&self) -> &str {
        "exit_plan_mode"
    }

    fn description(&self) -> &str {
        "Exit plan mode and IMMEDIATELY start writing code. You MUST begin implementation right after calling this. \
         Do NOT call task_done until you have actually created or modified code files. \
         Optionally launch a swarm of teammates to implement collaboratively."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::boolean(
                    "launchSwarm",
                    "Whether to launch a swarm of teammates to implement the plan",
                )
                .optional(),
                ToolParameter::number(
                    "teammateCount",
                    "Number of teammates in the swarm (1-10). Only used if launchSwarm is true",
                )
                .optional()
                .with_default(3),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let launch_swarm = call.get_bool("launchSwarm").unwrap_or(false);
        let teammate_count = call.get_number("teammateCount").unwrap_or(3.0) as u32;

        // Validate teammate count if swarm is being launched
        if launch_swarm && !(1..=10).contains(&teammate_count) {
            return Err(ToolError::InvalidArguments(
                "teammateCount must be between 1 and 10".to_string(),
            ));
        }

        let mut confirmation_message = String::from(
            r#"
╔══════════════════════════════════════════════════════════════╗
║                  PLAN MODE EXITED                            ║
╚══════════════════════════════════════════════════════════════╝

"#,
        );

        if launch_swarm {
            confirmation_message.push_str(&format!(
                r#"
🚀 SWARM MODE ACTIVATED

Launching swarm with {} teammates to implement the plan...

Swarm Configuration:
  • Teammates: {}
  • Collaboration: Enabled
  • Task Distribution: Automatic
  • Progress Tracking: Real-time

Your teammates will work collaboratively to implement the designed
plan. Each teammate will handle specific components while maintaining
communication and coordination with the swarm.

Ready to begin implementation!
"#,
                teammate_count, teammate_count
            ));
        } else {
            confirmation_message.push_str(
                r#"
⚡ IMPLEMENTATION MODE ACTIVATED ⚡

🚨 YOU MUST NOW START WRITING CODE IMMEDIATELY! 🚨

Do NOT:
  ✗ Call task_done without creating/modifying files
  ✗ Write more documentation
  ✗ Continue planning

START NOW:
  ✓ Create project structure
  ✓ Write actual code files
  ✓ Implement core functionality

NO MORE PLANNING - EXECUTE!
"#,
            );
        }

        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        confirmation_message.push_str(&format!("\nExited plan mode at: {}", timestamp));

        Ok(ToolResult::success(
            &call.id,
            self.name(),
            confirmation_message.trim(),
        ))
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let launch_swarm = call.get_bool("launchSwarm").unwrap_or(false);

        if launch_swarm {
            if let Some(count) = call.get_number("teammateCount") {
                let teammate_count = count as u32;
                if !(1..=10).contains(&teammate_count) {
                    return Err(ToolError::InvalidArguments(
                        "teammateCount must be between 1 and 10".to_string(),
                    ));
                }
            }
        }

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
    async fn test_exit_plan_mode_basic() {
        let tool = ExitPlanModeTool::new();
        let call = create_tool_call("test-1", "exit_plan_mode", json!({}));

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("PLAN MODE EXITED"));
        assert!(output.contains("IMPLEMENTATION MODE"));
        assert!(!output.contains("SWARM MODE"));
    }

    #[tokio::test]
    async fn test_exit_plan_mode_with_swarm() {
        let tool = ExitPlanModeTool::new();
        let call = create_tool_call(
            "test-2",
            "exit_plan_mode",
            json!({
                "launchSwarm": true,
                "teammateCount": 5
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("PLAN MODE EXITED"));
        assert!(output.contains("SWARM MODE ACTIVATED"));
        assert!(output.contains("5 teammates"));
        assert!(output.contains("Teammates: 5"));
    }

    #[tokio::test]
    async fn test_exit_plan_mode_swarm_default_count() {
        let tool = ExitPlanModeTool::new();
        let call = create_tool_call(
            "test-3",
            "exit_plan_mode",
            json!({
                "launchSwarm": true
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("SWARM MODE ACTIVATED"));
        assert!(output.contains("3 teammates")); // Default is 3
    }

    #[tokio::test]
    async fn test_exit_plan_mode_swarm_invalid_count_low() {
        let tool = ExitPlanModeTool::new();
        let call = create_tool_call(
            "test-4",
            "exit_plan_mode",
            json!({
                "launchSwarm": true,
                "teammateCount": 0
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("teammateCount must be between 1 and 10")
        );
    }

    #[tokio::test]
    async fn test_exit_plan_mode_swarm_invalid_count_high() {
        let tool = ExitPlanModeTool::new();
        let call = create_tool_call(
            "test-5",
            "exit_plan_mode",
            json!({
                "launchSwarm": true,
                "teammateCount": 11
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("teammateCount must be between 1 and 10")
        );
    }

    #[tokio::test]
    async fn test_exit_plan_mode_no_swarm_ignores_count() {
        let tool = ExitPlanModeTool::new();
        // When launchSwarm is false, teammateCount should be ignored
        let call = create_tool_call(
            "test-6",
            "exit_plan_mode",
            json!({
                "launchSwarm": false,
                "teammateCount": 100 // Invalid count, but should be ignored
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("IMPLEMENTATION MODE"));
        assert!(!output.contains("SWARM"));
    }

    #[tokio::test]
    async fn test_exit_plan_mode_validation_valid() {
        let tool = ExitPlanModeTool::new();
        let call = create_tool_call(
            "test-7",
            "exit_plan_mode",
            json!({
                "launchSwarm": true,
                "teammateCount": 5
            }),
        );

        let validation_result = tool.validate(&call);
        assert!(validation_result.is_ok());
    }

    #[tokio::test]
    async fn test_exit_plan_mode_validation_invalid() {
        let tool = ExitPlanModeTool::new();
        let call = create_tool_call(
            "test-8",
            "exit_plan_mode",
            json!({
                "launchSwarm": true,
                "teammateCount": 15
            }),
        );

        let validation_result = tool.validate(&call);
        assert!(validation_result.is_err());
    }

    #[tokio::test]
    async fn test_exit_plan_mode_timestamp() {
        let tool = ExitPlanModeTool::new();
        let call = create_tool_call("test-9", "exit_plan_mode", json!({}));

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("Exited plan mode at:"));
    }

    #[test]
    fn test_exit_plan_mode_schema() {
        let tool = ExitPlanModeTool::new();
        let schema = tool.schema();
        assert_eq!(schema.name, "exit_plan_mode");
        assert!(!schema.description.is_empty());
    }

    #[test]
    fn test_exit_plan_mode_max_execution_time() {
        let tool = ExitPlanModeTool::new();
        assert_eq!(tool.max_execution_time(), Some(5));
    }

    #[test]
    fn test_exit_plan_mode_supports_parallel_execution() {
        let tool = ExitPlanModeTool::new();
        assert!(tool.supports_parallel_execution());
    }

    #[test]
    fn test_exit_plan_mode_name() {
        let tool = ExitPlanModeTool::new();
        assert_eq!(tool.name(), "exit_plan_mode");
    }

    #[test]
    fn test_exit_plan_mode_description() {
        let tool = ExitPlanModeTool::new();
        let desc = tool.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("Exit plan mode"));
    }

    #[tokio::test]
    async fn test_exit_plan_mode_swarm_min_valid() {
        let tool = ExitPlanModeTool::new();
        let call = create_tool_call(
            "test-10",
            "exit_plan_mode",
            json!({
                "launchSwarm": true,
                "teammateCount": 1
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("1 teammates"));
    }

    #[tokio::test]
    async fn test_exit_plan_mode_swarm_max_valid() {
        let tool = ExitPlanModeTool::new();
        let call = create_tool_call(
            "test-11",
            "exit_plan_mode",
            json!({
                "launchSwarm": true,
                "teammateCount": 10
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("10 teammates"));
    }
}
