//! Skill execution tool
//!
//! Allows executing specialized skills within conversation context.
//! Skills provide domain-specific capabilities and can be defined as
//! markdown files with YAML frontmatter.
//!
//! ## Claude Code Compatible
//!
//! This tool is designed to work like Claude Code's skill system:
//! - Skills are loaded from `.sage/skills/` and `~/.config/sage/skills/`
//! - Skills can be defined as `skill-name.md` or `skill-name/SKILL.md`
//! - Supports `$ARGUMENTS` parameter substitution
//! - AI can auto-invoke skills based on `when_to_use` condition

use async_trait::async_trait;
use sage_core::skills::{SkillContext, SkillRegistry};
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tool for executing specialized skills
///
/// Skills are specialized capabilities that can be invoked within the conversation.
/// Each skill provides domain-specific functionality and expertise.
///
/// # Examples
///
/// - `skill: "commit"` - Invoke git commit skill
/// - `skill: "review-pr", args: "123"` - Review PR #123
/// - `skill: "code-review", args: "src/main.rs"` - Review specific file
pub struct SkillTool {
    /// Skill registry (shared)
    registry: Arc<RwLock<SkillRegistry>>,
    /// Current working directory
    working_dir: PathBuf,
}

impl Default for SkillTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillTool {
    /// Create a new SkillTool instance with default registry
    pub fn new() -> Self {
        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut registry = SkillRegistry::new(&working_dir);
        registry.register_builtins();

        Self {
            registry: Arc::new(RwLock::new(registry)),
            working_dir,
        }
    }

    /// Create with a specific working directory
    pub fn with_working_dir(working_dir: impl Into<PathBuf>) -> Self {
        let working_dir = working_dir.into();
        let mut registry = SkillRegistry::new(&working_dir);
        registry.register_builtins();

        Self {
            registry: Arc::new(RwLock::new(registry)),
            working_dir,
        }
    }

    /// Create with an existing registry
    pub fn with_registry(registry: Arc<RwLock<SkillRegistry>>) -> Self {
        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            registry,
            working_dir,
        }
    }

    /// Discover skills from the file system
    pub async fn discover_skills(&self) -> Result<usize, ToolError> {
        let mut registry = self.registry.write().await;
        registry
            .discover()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to discover skills: {}", e)))
    }

    /// Get a reference to the registry
    pub fn registry(&self) -> Arc<RwLock<SkillRegistry>> {
        Arc::clone(&self.registry)
    }

    /// Get the skill prompt with arguments substituted
    #[allow(dead_code)]
    async fn get_skill_prompt(&self, skill_name: &str, args: Option<&str>) -> Option<String> {
        let registry = self.registry.read().await;
        let skill = registry.get(skill_name)?;

        let context = SkillContext::new("")
            .with_working_dir(&self.working_dir);

        Some(skill.get_prompt_with_args(&context, args))
    }

    /// Validate skill name
    fn validate_skill_name(&self, skill: &str) -> Result<(), ToolError> {
        if skill.is_empty() {
            return Err(ToolError::InvalidArguments(
                "Skill name cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    /// Execute the skill
    async fn execute_skill(&self, skill_name: &str, args: Option<&str>) -> Result<String, ToolError> {
        let registry = self.registry.read().await;

        if let Some(skill) = registry.get(skill_name) {
            let context = SkillContext::new("")
                .with_working_dir(&self.working_dir);

            let prompt = skill.get_prompt_with_args(&context, args);

            // Format the response with skill metadata
            let mut result = format!("# Skill: {}\n\n", skill.user_facing_name());
            result.push_str(&format!("**Description:** {}\n\n", skill.description));

            if let Some(ref when) = skill.when_to_use {
                result.push_str(&format!("**When to use:** {}\n\n", when));
            }

            result.push_str("---\n\n");
            result.push_str(&prompt);

            Ok(result)
        } else {
            // List available skills
            let available: Vec<String> = registry
                .list_enabled()
                .iter()
                .map(|s| s.name.clone())
                .collect();

            Err(ToolError::ExecutionFailed(format!(
                "Skill '{}' not found. Available skills: {}",
                skill_name,
                available.join(", ")
            )))
        }
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str {
        "Skill"
    }

    fn description(&self) -> &str {
        "Execute a specialized skill within the conversation. Skills provide domain-specific capabilities \
         and expertise. Skills can be invoked by name with optional arguments. Use /skill-name or call \
         this tool with the skill name."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "skill",
                    "The name of the skill to execute (e.g., 'commit', 'review-pr', 'comprehensive-testing')",
                ),
                ToolParameter::string(
                    "args",
                    "Optional arguments to pass to the skill (replaces $ARGUMENTS in the skill prompt)",
                )
                .optional(),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        // Extract skill parameter
        let skill = call.get_string("skill").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: skill".to_string())
        })?;

        // Extract optional args
        let args = call.get_string("args");

        // Validate skill name
        self.validate_skill_name(&skill)?;

        // Execute the skill
        let result = self.execute_skill(&skill, args.as_deref()).await?;

        Ok(ToolResult::success(&call.id, self.name(), result))
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let skill = call.get_string("skill").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: skill".to_string())
        })?;

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

    fn create_tool_call_with_args(id: &str, name: &str, skill: &str, args: &str) -> ToolCall {
        let mut arguments = HashMap::new();
        arguments.insert("skill".to_string(), json!(skill));
        arguments.insert("args".to_string(), json!(args));

        ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments,
            call_id: None,
        }
    }

    #[tokio::test]
    async fn test_builtin_skill_execution() {
        let tool = SkillTool::new();
        // Test a builtin skill (rust-expert)
        let call = create_tool_call("test-1", "Skill", "rust-expert");

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.as_ref().unwrap().contains("rust-expert") ||
                result.output.as_ref().unwrap().contains("Rust"));
    }

    #[tokio::test]
    async fn test_skill_with_args() {
        let tool = SkillTool::new();
        let call = create_tool_call_with_args("test-args", "Skill", "comprehensive-testing", "src/lib.rs");

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_skill_validation() {
        let tool = SkillTool::new();

        // Valid skill
        let call = create_tool_call("test-2", "Skill", "rust-expert");
        assert!(tool.validate(&call).is_ok());

        // Empty skill name
        let call = create_tool_call("test-3", "Skill", "");
        assert!(tool.validate(&call).is_err());
    }

    #[tokio::test]
    async fn test_missing_skill_parameter() {
        let tool = SkillTool::new();
        let call = ToolCall {
            id: "test-4".to_string(),
            name: "Skill".to_string(),
            arguments: HashMap::new(),
            call_id: None,
        };

        let result = tool.execute(&call).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing required parameter")
        );
    }

    #[tokio::test]
    async fn test_builtin_skills_available() {
        let tool = SkillTool::new();

        // Test builtin skills that should exist
        let skills = vec![
            "rust-expert",
            "comprehensive-testing",
            "systematic-debugging",
        ];

        for skill in skills {
            let call = create_tool_call(&format!("test-{}", skill), "Skill", skill);
            let result = tool.execute(&call).await.unwrap();
            assert!(result.success, "Skill '{}' should be available", skill);
        }
    }

    #[tokio::test]
    async fn test_unknown_skill_error() {
        let tool = SkillTool::new();
        let call = create_tool_call("test-unknown", "Skill", "nonexistent-skill");

        let result = tool.execute(&call).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_tool_schema() {
        let tool = SkillTool::new();
        let schema = tool.schema();

        assert_eq!(schema.name, "Skill");
        assert!(!schema.description.is_empty());

        // Check that the schema has the skill parameter
        let params = schema.parameters.as_object().unwrap();
        assert!(params.contains_key("properties"));

        let properties = params.get("properties").unwrap().as_object().unwrap();
        assert!(properties.contains_key("skill"));
        assert!(properties.contains_key("args"));
    }

    #[tokio::test]
    async fn test_discover_skills() {
        let tool = SkillTool::new();

        // This should work even if no custom skills exist
        let count = tool.discover_skills().await;
        assert!(count.is_ok());
    }

    #[tokio::test]
    async fn test_registry_access() {
        let tool = SkillTool::new();
        let registry = tool.registry();

        let reg = registry.read().await;
        // Should have builtin skills
        assert!(reg.builtin_count() > 0);
    }
}
