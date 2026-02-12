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
        "EnterPlanMode"
    }

    fn description(&self) -> &str {
        r#"Use this tool proactively when you're about to start a non-trivial implementation task. Getting user sign-off on your approach before writing code prevents wasted effort and ensures alignment. This tool transitions you into plan mode where you can explore the codebase and design an implementation approach for user approval.

## When to Use This Tool

**Prefer using EnterPlanMode** for implementation tasks unless they're simple. Use it when ANY of these conditions apply:

1. **New Feature Implementation**: Adding meaningful new functionality
   - Example: "Add a logout button" - where should it go? What should happen on click?
   - Example: "Add form validation" - what rules? What error messages?

2. **Multiple Valid Approaches**: The task can be solved in several different ways
   - Example: "Add caching to the API" - could use Redis, in-memory, file-based, etc.
   - Example: "Improve performance" - many optimization strategies possible

3. **Code Modifications**: Changes that affect existing behavior or structure
   - Example: "Update the login flow" - what exactly should change?
   - Example: "Refactor this component" - what's the target architecture?

4. **Architectural Decisions**: The task requires choosing between patterns or technologies
   - Example: "Add real-time updates" - WebSockets vs SSE vs polling
   - Example: "Implement state management" - Redux vs Context vs custom solution

5. **Multi-File Changes**: The task will likely touch more than 2-3 files
   - Example: "Refactor the authentication system"
   - Example: "Add a new API endpoint with tests"

6. **Unclear Requirements**: You need to explore before understanding the full scope
   - Example: "Make the app faster" - need to profile and identify bottlenecks
   - Example: "Fix the bug in checkout" - need to investigate root cause

7. **User Preferences Matter**: The implementation could reasonably go multiple ways
   - If you would use ask_user_question to clarify the approach, use EnterPlanMode instead
   - Plan mode lets you explore first, then present options with context

## When NOT to Use This Tool

Only skip EnterPlanMode for simple tasks:
- Single-line or few-line fixes (typos, obvious bugs, small tweaks)
- Adding a single function with clear requirements
- Tasks where the user has given very specific, detailed instructions
- Pure research/exploration tasks (use the Task tool with explore agent instead)

## What Happens in Plan Mode

In plan mode, you'll:
1. Thoroughly explore the codebase using Glob, Grep, and Read tools
2. Understand existing patterns and architecture
3. Design an implementation approach
4. Present your plan to the user for approval
5. Use AskUserQuestion if you need to clarify approaches
6. Exit plan mode with ExitPlanMode when ready to implement

## Examples

### GOOD - Use EnterPlanMode:
User: "Add user authentication to the app"
- Requires architectural decisions (session vs JWT, where to store tokens, middleware structure)

User: "Optimize the database queries"
- Multiple approaches possible, need to profile first, significant impact

User: "Implement dark mode"
- Architectural decision on theme system, affects many components

### BAD - Don't use EnterPlanMode:
User: "Fix the typo in the README"
- Straightforward, no planning needed

User: "Add a console.log to debug this function"
- Simple, obvious implementation

User: "What files handle routing?"
- Research task, not implementation planning

## Important Notes

- This tool REQUIRES user approval - they must consent to entering plan mode
- If unsure whether to use it, err on the side of planning - it's better to get alignment upfront than to redo work
- Users appreciate being consulted before significant changes are made to their codebase"#
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

Use ExitPlanMode NOW and begin implementation.
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

    fn max_execution_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(5)) // 5 seconds - this is a very lightweight operation
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
        let call = create_tool_call("test-1", "EnterPlanMode", json!({}));

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("QUICK PLAN MODE"));
        assert!(output.contains("WRITE CODE"));
        assert!(output.contains("ExitPlanMode"));
    }

    #[tokio::test]
    async fn test_enter_plan_mode_validation() {
        let tool = EnterPlanModeTool::new();
        let call = create_tool_call("test-2", "EnterPlanMode", json!({}));

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
            "EnterPlanMode",
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
        assert_eq!(schema.name, "EnterPlanMode");
        assert!(!schema.description.is_empty());
    }

    #[test]
    fn test_enter_plan_mode_max_execution_duration() {
        use std::time::Duration;
        let tool = EnterPlanModeTool::new();
        assert_eq!(tool.max_execution_duration(), Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_enter_plan_mode_supports_parallel_execution() {
        let tool = EnterPlanModeTool::new();
        assert!(tool.supports_parallel_execution());
    }

    #[test]
    fn test_enter_plan_mode_name() {
        let tool = EnterPlanModeTool::new();
        assert_eq!(tool.name(), "EnterPlanMode");
    }

    #[test]
    fn test_enter_plan_mode_description() {
        let tool = EnterPlanModeTool::new();
        let desc = tool.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("plan mode"));
    }
}
