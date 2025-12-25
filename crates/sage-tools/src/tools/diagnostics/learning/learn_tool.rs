//! Learn tool implementation for recording user corrections and preferences

use async_trait::async_trait;
use sage_core::learning::{Pattern, PatternSource};
use sage_core::tools::{Tool, ToolCall, ToolError, ToolResult, ToolSchema};
use serde_json::json;
use std::collections::HashMap;

use super::analyzer::parse_pattern_type;
use super::schema::learn_tool_schema;
use super::types::ensure_learning_engine;

/// Learn tool for recording user corrections and preferences
#[derive(Debug, Clone)]
pub struct LearnTool;

impl Default for LearnTool {
    fn default() -> Self {
        Self::new()
    }
}

impl LearnTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for LearnTool {
    fn name(&self) -> &str {
        "Learn"
    }

    fn description(&self) -> &str {
        r#"Learn from user corrections and preferences to improve future interactions.

Use this tool when:
- User explicitly corrects your behavior ("don't do X, do Y instead")
- User states a preference ("I prefer X over Y")
- You discover a pattern in the user's workflow
- User teaches you something about their codebase or project

Pattern types:
- correction: User corrected something you did wrong
- preference: User preference for tool usage or workflow
- style: Coding style preference (formatting, naming)
- workflow: Workflow preference (commit frequency, testing approach)

Do NOT use for:
- One-off instructions that won't apply to future interactions
- Sensitive information
- Project-specific facts (use Remember tool instead)"#
    }

    fn schema(&self) -> ToolSchema {
        learn_tool_schema()
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let pattern_type_str = call.get_string("pattern_type").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'pattern_type' parameter".to_string())
        })?;

        let description = call.get_string("description").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'description' parameter".to_string())
        })?;

        let rule = call
            .get_string("rule")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'rule' parameter".to_string()))?;

        let context: Vec<String> = call
            .get_string("context")
            .map(|s| {
                s.split(',')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        // Parse pattern type
        let pattern_type = parse_pattern_type(&pattern_type_str);

        // Get or initialize learning engine
        let engine = ensure_learning_engine().await;

        // Create pattern
        let mut pattern = Pattern::new(
            pattern_type,
            description.clone(),
            rule.clone(),
            PatternSource::UserExplicit,
        )
        .with_confidence(0.8); // User-explicit patterns start with high confidence

        for ctx in &context {
            pattern = pattern.with_context(ctx.clone());
        }

        // Learn the pattern
        let pattern_id = engine
            .learn(pattern)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to learn pattern: {}", e)))?;

        // Get stats
        let stats = engine.stats().await;

        let response = format!(
            "Pattern learned successfully.\n\
             Type: {}\n\
             Description: {}\n\
             Rule: {}\n\
             Context: {}\n\
             Pattern ID: {}\n\n\
             Total patterns: {}, {} high-confidence",
            pattern_type_str,
            description,
            rule,
            if context.is_empty() {
                "none".to_string()
            } else {
                context.join(", ")
            },
            pattern_id,
            stats.total_patterns,
            stats.high_confidence_count
        );

        Ok(ToolResult {
            call_id: call.id.clone(),
            tool_name: self.name().to_string(),
            success: true,
            output: Some(response),
            error: None,
            exit_code: None,
            execution_time_ms: None,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("pattern_id".to_string(), json!(pattern_id.as_str()));
                meta.insert("pattern_type".to_string(), json!(pattern_type_str));
                meta
            },
        })
    }
}
