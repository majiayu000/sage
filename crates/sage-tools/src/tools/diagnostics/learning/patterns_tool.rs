//! LearningPatterns tool implementation for viewing and managing patterns

use async_trait::async_trait;
use sage_core::learning::PatternId;
use sage_core::tools::{Tool, ToolCall, ToolError, ToolResult, ToolSchema};

use super::analyzer::{
    format_pattern_list, format_search_results, get_all_patterns, parse_pattern_type,
    search_patterns,
};
use super::schema::learning_patterns_tool_schema;
use super::types::ensure_learning_engine;

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
        learning_patterns_tool_schema()
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let action = call
            .get_string("action")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'action' parameter".to_string()))?;

        let engine = ensure_learning_engine().await;

        let response = match action.to_lowercase().as_str() {
            "list" => {
                let pattern_type = call.get_string("pattern_type");
                let patterns = if let Some(pt) = pattern_type.as_ref() {
                    let pattern_type_filter = parse_pattern_type(pt);
                    engine.get_patterns_by_type(pattern_type_filter).await
                } else {
                    get_all_patterns(&engine).await
                };

                format_pattern_list(&patterns)
            }

            "search" => {
                let query = call.get_string("query").ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'query' for search".to_string())
                })?;

                let matches = search_patterns(&engine, &query).await;
                format_search_results(&matches, &query)
            }

            "delete" => {
                let pattern_id = call.get_string("pattern_id").ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'pattern_id' for delete".to_string())
                })?;

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

        Ok(ToolResult::success(&call.id, self.name(), response))
    }
}
