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
    assert!(
        result.output.as_ref().unwrap().contains("rust-expert")
            || result.output.as_ref().unwrap().contains("Rust")
    );
}

#[tokio::test]
async fn test_skill_with_args() {
    let tool = SkillTool::new();
    let call =
        create_tool_call_with_args("test-args", "Skill", "comprehensive-testing", "src/lib.rs");

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
