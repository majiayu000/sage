//! Skill execution tool
//!
//! Allows executing specialized skills within conversation context.
//! Skills provide domain-specific capabilities like PDF processing,
//! Excel manipulation, brainstorming, testing, etc.

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema, ToolParameter};

/// Tool for executing specialized skills
///
/// Skills are specialized capabilities that can be invoked within the conversation.
/// Each skill provides domain-specific functionality and expertise.
///
/// # Examples
///
/// - `skill: "pdf"` - Invoke PDF processing skill
/// - `skill: "xlsx"` - Invoke Excel/spreadsheet skill
/// - `skill: "brainstorming"` - Invoke collaborative brainstorming skill
/// - `skill: "comprehensive-testing"` - Invoke testing strategy skill
pub struct SkillTool;

impl Default for SkillTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillTool {
    /// Create a new SkillTool instance
    pub fn new() -> Self {
        Self
    }

    /// Get available skills
    fn get_available_skills() -> Vec<&'static str> {
        vec![
            "artifacts-builder",
            "brainstorming",
            "comprehensive-testing",
            "elegant-architecture",
            "frontend-design",
            "git-commit-smart",
            "playwright-automation",
            "product-manager",
            "project-health-auditor",
            "rust-best-practices",
            "systematic-debugging",
            "test-driven-development",
            "ui-designer",
        ]
    }

    /// Validate skill name
    fn validate_skill_name(&self, skill: &str) -> Result<(), ToolError> {
        if skill.is_empty() {
            return Err(ToolError::InvalidArguments(
                "Skill name cannot be empty".to_string()
            ));
        }

        // Allow any skill name for flexibility
        // The actual skill availability is determined by the runtime environment
        Ok(())
    }

    /// Execute the skill
    async fn execute_skill(&self, skill: &str) -> Result<String, ToolError> {
        // In a real implementation, this would invoke the skill system
        // For now, we return a message indicating the skill would be invoked
        let available_skills = Self::get_available_skills();

        if available_skills.contains(&skill) {
            Ok(format!(
                "Skill '{}' execution initiated. The skill will be invoked within the conversation context.",
                skill
            ))
        } else {
            Ok(format!(
                "Skill '{}' will be invoked. Note: This skill may not be in the standard library. Available skills: {}",
                skill,
                available_skills.join(", ")
            ))
        }
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str {
        "skill"
    }

    fn description(&self) -> &str {
        "Execute a specialized skill within the conversation. Skills provide domain-specific capabilities \
         and expertise for tasks like PDF processing, Excel manipulation, brainstorming, testing strategies, \
         architecture design, and more. Use when you need specialized functionality beyond standard tools."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "skill",
                    "The name of the skill to execute (e.g., 'pdf', 'xlsx', 'brainstorming', 'comprehensive-testing')"
                ),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        // Extract skill parameter
        let skill = call.get_string("skill")
            .ok_or_else(|| ToolError::InvalidArguments(
                "Missing required parameter: skill".to_string()
            ))?;

        // Validate skill name
        self.validate_skill_name(&skill)?;

        // Execute the skill
        let result = self.execute_skill(&skill).await?;

        Ok(ToolResult::success(&call.id, self.name(), result))
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let skill = call.get_string("skill")
            .ok_or_else(|| ToolError::InvalidArguments(
                "Missing required parameter: skill".to_string()
            ))?;

        self.validate_skill_name(&skill)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn create_tool_call(id: &str, name: &str, skill: &str) -> ToolCall {
        let mut arguments = HashMap::new();
        arguments.insert("skill".to_string(), json!(skill));

        ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments,
            call_id: None,
        }
    }

    #[tokio::test]
    async fn test_skill_execution() {
        let tool = SkillTool::new();
        let call = create_tool_call("test-1", "skill", "pdf");

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.as_ref().unwrap().contains("pdf"));
    }

    #[tokio::test]
    async fn test_skill_validation() {
        let tool = SkillTool::new();

        // Valid skill
        let call = create_tool_call("test-2", "skill", "brainstorming");
        assert!(tool.validate(&call).is_ok());

        // Empty skill name
        let call = create_tool_call("test-3", "skill", "");
        assert!(tool.validate(&call).is_err());
    }

    #[tokio::test]
    async fn test_missing_skill_parameter() {
        let tool = SkillTool::new();
        let call = ToolCall {
            id: "test-4".to_string(),
            name: "skill".to_string(),
            arguments: HashMap::new(),
            call_id: None,
        };

        let result = tool.execute(&call).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing required parameter"));
    }

    #[tokio::test]
    async fn test_available_skills() {
        let tool = SkillTool::new();

        // Test a few standard skills
        let skills = vec!["brainstorming", "comprehensive-testing", "rust-best-practices"];

        for skill in skills {
            let call = create_tool_call(&format!("test-{}", skill), "skill", skill);
            let result = tool.execute(&call).await.unwrap();
            assert!(result.success);
        }
    }

    #[tokio::test]
    async fn test_custom_skill() {
        let tool = SkillTool::new();
        let call = create_tool_call("test-5", "skill", "custom-skill");

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        // Should still work but indicate it's not in standard library
        assert!(result.output.as_ref().unwrap().contains("custom-skill"));
    }

    #[tokio::test]
    async fn test_tool_schema() {
        let tool = SkillTool::new();
        let schema = tool.schema();

        assert_eq!(schema.name, "skill");
        assert!(!schema.description.is_empty());

        // Check that the schema has the skill parameter
        let params = schema.parameters.as_object().unwrap();
        assert!(params.contains_key("properties"));

        let properties = params.get("properties").unwrap().as_object().unwrap();
        assert!(properties.contains_key("skill"));
    }
}
