//! Learning mode tools
//!
//! Provides tools for learning from user corrections and preferences,
//! similar to Claude Code's learning capabilities.

use async_trait::async_trait;
use sage_core::learning::{
    create_learning_engine, LearningConfig, Pattern, PatternSource, PatternType,
    SharedLearningEngine,
};
use sage_core::tools::{Tool, ToolCall, ToolError, ToolParameter, ToolResult, ToolSchema};
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::OnceCell;

/// Global learning engine instance
static GLOBAL_LEARNING_ENGINE: OnceCell<SharedLearningEngine> = OnceCell::const_new();

/// Initialize the global learning engine
pub async fn init_global_learning_engine(config: Option<LearningConfig>) -> Result<(), String> {
    let config = config.unwrap_or_default();
    let engine = create_learning_engine(config);

    GLOBAL_LEARNING_ENGINE
        .set(engine)
        .map_err(|_| "Learning engine already initialized".to_string())
}

/// Get the global learning engine
pub fn get_global_learning_engine() -> Option<SharedLearningEngine> {
    GLOBAL_LEARNING_ENGINE.get().cloned()
}

/// Ensure learning engine is initialized (creates default if not)
async fn ensure_learning_engine() -> SharedLearningEngine {
    if let Some(engine) = GLOBAL_LEARNING_ENGINE.get() {
        return engine.clone();
    }

    // Initialize with default config
    let engine = create_learning_engine(LearningConfig::default());

    // Try to set, if fails (race condition), just get the existing one
    let _ = GLOBAL_LEARNING_ENGINE.set(engine.clone());
    GLOBAL_LEARNING_ENGINE.get().cloned().unwrap_or(engine)
}

/// Get patterns for system prompt injection
pub async fn get_learning_patterns_for_context(limit: usize) -> Vec<String> {
    let engine = match GLOBAL_LEARNING_ENGINE.get() {
        Some(e) => e,
        None => return Vec::new(),
    };

    engine.get_patterns_for_prompt(limit).await
}

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
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "pattern_type",
                    "Type of pattern: correction, preference, style, workflow",
                ),
                ToolParameter::string(
                    "description",
                    "Brief description of what was learned (1-2 sentences)",
                ),
                ToolParameter::string("rule", "The actual rule or behavior to follow"),
                ToolParameter::optional_string(
                    "context",
                    "Comma-separated context tags (e.g., 'rust,testing,bash')",
                ),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let pattern_type_str = call
            .get_string("pattern_type")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'pattern_type' parameter".to_string()))?;

        let description = call
            .get_string("description")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'description' parameter".to_string()))?;

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
        let pattern_type = match pattern_type_str.to_lowercase().as_str() {
            "correction" => PatternType::Correction,
            "preference" | "tool_preference" => PatternType::ToolPreference,
            "style" | "coding_style" => PatternType::CodingStyle,
            "workflow" | "workflow_preference" => PatternType::WorkflowPreference,
            _ => PatternType::Custom,
        };

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

/// Learning patterns tool for viewing and managing learned patterns
#[derive(Debug, Clone)]
pub struct LearningPatternsTool;

impl Default for LearningPatternsTool {
    fn default() -> Self {
        Self::new()
    }
}

impl LearningPatternsTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for LearningPatternsTool {
    fn name(&self) -> &str {
        "LearningPatterns"
    }

    fn description(&self) -> &str {
        r#"View, search, or manage learned patterns.

Actions:
- list: Show all patterns (optionally filtered by type)
- search: Search patterns by text
- delete: Delete a pattern by ID
- clear: Clear all patterns (use with caution)
- stats: Show learning statistics
- apply_decay: Apply time-based decay to patterns"#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "action",
                    "Action to perform: list, search, delete, clear, stats, apply_decay",
                ),
                ToolParameter::optional_string("query", "Search query (for 'search' action)"),
                ToolParameter::optional_string(
                    "pattern_type",
                    "Filter by type: correction, preference, style, workflow",
                ),
                ToolParameter::optional_string("pattern_id", "Pattern ID (for 'delete' action)"),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let action = call
            .get_string("action")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'action' parameter".to_string()))?;

        let engine = ensure_learning_engine().await;

        let response = match action.to_lowercase().as_str() {
            "list" => {
                let pattern_type = call.get_string("pattern_type");
                let pattern_type_filter = pattern_type.as_ref().map(|t| match t.to_lowercase().as_str() {
                    "correction" => PatternType::Correction,
                    "preference" | "tool_preference" => PatternType::ToolPreference,
                    "style" | "coding_style" => PatternType::CodingStyle,
                    "workflow" | "workflow_preference" => PatternType::WorkflowPreference,
                    _ => PatternType::Custom,
                });

                let patterns = if let Some(pt) = pattern_type_filter {
                    engine.get_patterns_by_type(pt).await
                } else {
                    // Get all patterns by iterating through types
                    let mut all = Vec::new();
                    for pt in [
                        PatternType::Correction,
                        PatternType::ToolPreference,
                        PatternType::CodingStyle,
                        PatternType::WorkflowPreference,
                        PatternType::Custom,
                    ] {
                        all.extend(engine.get_patterns_by_type(pt).await);
                    }
                    all
                };

                if patterns.is_empty() {
                    "No patterns found.".to_string()
                } else {
                    let mut output = format!("Found {} patterns:\n\n", patterns.len());
                    for (i, p) in patterns.iter().enumerate() {
                        output.push_str(&format!(
                            "{}. [{}] {}\n   Rule: {}\n   Confidence: {:.0}%, Context: {}\n   ID: {}\n\n",
                            i + 1,
                            p.pattern_type.name(),
                            p.description,
                            p.rule,
                            p.confidence.value() * 100.0,
                            if p.context.is_empty() {
                                "none".to_string()
                            } else {
                                p.context.join(", ")
                            },
                            p.id.as_str()
                        ));
                    }
                    output
                }
            }

            "search" => {
                let query = call
                    .get_string("query")
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'query' for search".to_string()))?;

                let query_lower = query.to_lowercase();

                // Search all pattern types
                let mut matches = Vec::new();
                for pt in [
                    PatternType::Correction,
                    PatternType::ToolPreference,
                    PatternType::CodingStyle,
                    PatternType::WorkflowPreference,
                    PatternType::Custom,
                ] {
                    for p in engine.get_patterns_by_type(pt).await {
                        if p.description.to_lowercase().contains(&query_lower)
                            || p.rule.to_lowercase().contains(&query_lower)
                            || p.context.iter().any(|c| c.to_lowercase().contains(&query_lower))
                        {
                            matches.push(p);
                        }
                    }
                }

                if matches.is_empty() {
                    format!("No patterns found matching '{}'.", query)
                } else {
                    let mut output = format!("Found {} patterns matching '{}':\n\n", matches.len(), query);
                    for (i, p) in matches.iter().enumerate() {
                        output.push_str(&format!(
                            "{}. [{}] {}\n   Rule: {}\n   ID: {}\n\n",
                            i + 1,
                            p.pattern_type.name(),
                            p.description,
                            p.rule,
                            p.id.as_str()
                        ));
                    }
                    output
                }
            }

            "delete" => {
                let pattern_id = call
                    .get_string("pattern_id")
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'pattern_id' for delete".to_string()))?;

                use sage_core::learning::PatternId;
                let id = PatternId::from_string(pattern_id.clone());
                engine
                    .remove_pattern(&id)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(format!("Delete failed: {}", e)))?;

                format!("Pattern '{}' deleted.", pattern_id)
            }

            "clear" => {
                engine.clear().await;
                "All patterns cleared.".to_string()
            }

            "stats" => {
                let stats = engine.stats().await;

                format!(
                    "Learning Statistics:\n\
                     - Total patterns: {}\n\
                     - High-confidence patterns: {}\n\
                     - Average confidence: {:.0}%\n\
                     - Patterns applied this session: {}\n\
                     - Learning events: {}\n\n\
                     By Type:\n{}",
                    stats.total_patterns,
                    stats.high_confidence_count,
                    stats.avg_confidence * 100.0,
                    stats.patterns_applied,
                    stats.events_count,
                    stats
                        .patterns_by_type
                        .iter()
                        .map(|(k, v)| format!("  - {}: {}", k, v))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }

            "apply_decay" => {
                engine.apply_decay().await;
                let stats = engine.stats().await;
                format!(
                    "Decay applied. Remaining patterns: {}, high-confidence: {}",
                    stats.total_patterns, stats.high_confidence_count
                )
            }

            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown action: '{}'. Valid actions: list, search, delete, clear, stats, apply_decay",
                    action
                )));
            }
        };

        Ok(ToolResult {
            call_id: call.id.clone(),
            tool_name: self.name().to_string(),
            success: true,
            output: Some(response),
            error: None,
            exit_code: None,
            execution_time_ms: None,
            metadata: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_learn_tool() {
        let tool = LearnTool::new();

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "Learn".to_string(),
            arguments: json!({
                "pattern_type": "correction",
                "description": "Avoid using grep -r",
                "rule": "Use ripgrep (rg) instead of grep -r for better performance",
                "context": "bash,search"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.unwrap().contains("Pattern learned"));
    }

    #[tokio::test]
    async fn test_learning_patterns_list() {
        let learn_tool = LearnTool::new();
        let patterns_tool = LearningPatternsTool::new();

        // Add a pattern first
        let add_call = ToolCall {
            id: "test-1".to_string(),
            name: "Learn".to_string(),
            arguments: json!({
                "pattern_type": "preference",
                "description": "User prefers 4-space indentation",
                "rule": "Use 4 spaces for indentation, not tabs"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
            call_id: None,
        };
        learn_tool.execute(&add_call).await.unwrap();

        // List patterns
        let list_call = ToolCall {
            id: "test-2".to_string(),
            name: "LearningPatterns".to_string(),
            arguments: json!({
                "action": "list"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
            call_id: None,
        };

        let result = patterns_tool.execute(&list_call).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_learning_patterns_stats() {
        let tool = LearningPatternsTool::new();

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "LearningPatterns".to_string(),
            arguments: json!({
                "action": "stats"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.unwrap().contains("Learning Statistics"));
    }
}
