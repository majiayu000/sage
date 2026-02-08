//! Session notes tool for viewing/managing all memories

use async_trait::async_trait;
use sage_core::memory::{MemoryId, MemoryType};
use sage_core::tools::{Tool, ToolCall, ToolError, ToolResult, ToolSchema};
use std::collections::HashMap;

use super::schema::session_notes_schema;
use super::types::ensure_memory_manager;

/// Session notes tool for viewing/managing all memories
#[derive(Debug, Clone)]
pub struct SessionNotesTool;

impl Default for SessionNotesTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionNotesTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for SessionNotesTool {
    fn name(&self) -> &str {
        "SessionNotes"
    }

    fn description(&self) -> &str {
        session_notes_schema().description
    }

    fn schema(&self) -> ToolSchema {
        session_notes_schema()
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let action = call
            .get_string("action")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'action' parameter".to_string()))?;

        let manager = ensure_memory_manager().await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to initialize memory manager: {}", e))
        })?;

        let response = match action.to_lowercase().as_str() {
            "list" => {
                let memory_type = call.get_string("memory_type");
                let memories = if let Some(type_str) = memory_type {
                    let mem_type = match type_str.to_lowercase().as_str() {
                        "fact" => MemoryType::Fact,
                        "preference" => MemoryType::Preference,
                        "lesson" => MemoryType::Lesson,
                        "note" | "custom" => MemoryType::Custom,
                        _ => MemoryType::Fact,
                    };
                    manager.find_by_type(mem_type).await
                } else {
                    // Get all memories
                    let mut all = Vec::new();
                    for mem_type in [
                        MemoryType::Fact,
                        MemoryType::Preference,
                        MemoryType::Lesson,
                        MemoryType::Custom,
                    ] {
                        all.extend(manager.find_by_type(mem_type).await.unwrap_or_default());
                    }
                    Ok(all)
                };

                let memories = memories.map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to list memories: {}", e))
                })?;

                if memories.is_empty() {
                    "No memories found.".to_string()
                } else {
                    let mut output = format!("Found {} memories:\n\n", memories.len());
                    for (i, mem) in memories.iter().enumerate() {
                        output.push_str(&format!(
                            "{}. [{}] {}\n   ID: {}, Tags: {}\n\n",
                            i + 1,
                            mem.memory_type.name(),
                            mem.content,
                            mem.id.as_str(),
                            if mem.metadata.tags.is_empty() {
                                "none".to_string()
                            } else {
                                mem.metadata.tags.join(", ")
                            }
                        ));
                    }
                    output
                }
            }

            "search" => {
                let query = call.get_string("query").ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'query' for search".to_string())
                })?;

                let memories = manager
                    .find(&query)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(format!("Search failed: {}", e)))?;

                if memories.is_empty() {
                    format!("No memories found matching '{}'.", query)
                } else {
                    let mut output = format!(
                        "Found {} memories matching '{}':\n\n",
                        memories.len(),
                        query
                    );
                    for (i, mem) in memories.iter().enumerate() {
                        output.push_str(&format!(
                            "{}. [{}] {}\n   ID: {}\n\n",
                            i + 1,
                            mem.memory_type.name(),
                            mem.content,
                            mem.id.as_str()
                        ));
                    }
                    output
                }
            }

            "delete" => {
                let memory_id = call.get_string("memory_id").ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'memory_id' for delete".to_string())
                })?;

                let id = MemoryId::from_string(memory_id.clone());
                manager
                    .delete(&id)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(format!("Delete failed: {}", e)))?;

                format!("Memory '{}' deleted.", memory_id)
            }

            "clear" => {
                manager
                    .clear()
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(format!("Clear failed: {}", e)))?;

                "All memories cleared.".to_string()
            }

            "stats" => {
                let stats = manager.stats().await.map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to get stats: {}", e))
                })?;

                format!(
                    "Memory Statistics:\n\
                     - Total memories: {}\n\
                     - Pinned: {}\n\
                     - Average relevance: {:.2}\n\
                     - Created (last 24h): {}\n\
                     - Accessed (last 24h): {}\n\n\
                     By Type:\n{}",
                    stats.total,
                    stats.pinned,
                    stats.avg_relevance,
                    stats.created_last_24h,
                    stats.accessed_last_24h,
                    stats
                        .by_type
                        .iter()
                        .map(|(k, v)| format!("  - {}: {}", k, v))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }

            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown action: '{}'. Valid actions: list, search, delete, clear, stats",
                    action
                )));
            }
        };

        Ok(ToolResult::success(&call.id, self.name(), response))
    }
}
