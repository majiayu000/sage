//! Memory and session notes tools
//!
//! Provides tools for storing and retrieving memories that persist across sessions.

use async_trait::async_trait;
use sage_core::memory::{
    Memory, MemoryCategory, MemoryConfig, MemoryManager, MemoryType, SharedMemoryManager,
};
use sage_core::tools::{Tool, ToolCall, ToolError, ToolParameter, ToolResult, ToolSchema};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OnceCell;

/// Global memory manager instance
static GLOBAL_MEMORY_MANAGER: OnceCell<SharedMemoryManager> = OnceCell::const_new();

/// Initialize the global memory manager
pub async fn init_global_memory_manager(storage_path: Option<PathBuf>) -> Result<(), String> {
    let config = if let Some(path) = storage_path {
        MemoryConfig::with_file_storage(path)
    } else {
        MemoryConfig::default()
    };

    let manager = MemoryManager::new(config)
        .await
        .map_err(|e| format!("Failed to create memory manager: {}", e))?;

    GLOBAL_MEMORY_MANAGER
        .set(Arc::new(manager))
        .map_err(|_| "Memory manager already initialized".to_string())
}

/// Get the global memory manager
pub fn get_global_memory_manager() -> Option<SharedMemoryManager> {
    GLOBAL_MEMORY_MANAGER.get().cloned()
}

/// Ensure memory manager is initialized (creates in-memory if not)
async fn ensure_memory_manager() -> SharedMemoryManager {
    if let Some(manager) = GLOBAL_MEMORY_MANAGER.get() {
        return manager.clone();
    }

    // Initialize with default in-memory storage
    let config = MemoryConfig::default();
    let manager = MemoryManager::new(config)
        .await
        .expect("Failed to create default memory manager");
    let shared = Arc::new(manager);

    // Try to set, if fails (race condition), just get the existing one
    let _ = GLOBAL_MEMORY_MANAGER.set(shared.clone());
    GLOBAL_MEMORY_MANAGER.get().cloned().unwrap_or(shared)
}

/// Remember tool for storing memories
#[derive(Debug, Clone)]
pub struct RememberTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct RememberInput {
    pub memory: String,
    #[serde(default)]
    pub memory_type: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Default for RememberTool {
    fn default() -> Self {
        Self::new()
    }
}

impl RememberTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for RememberTool {
    fn name(&self) -> &str {
        "Remember"
    }

    fn description(&self) -> &str {
        r#"Store information in long-term memory that persists across sessions.

Use this tool when:
- User explicitly asks you to remember something
- You learn an important fact about the user's preferences
- You discover something important about the codebase or project
- You learn lessons from mistakes or successes

Memory types:
- fact: General facts about the user, project, or codebase
- preference: User preferences for coding style, tools, etc.
- lesson: Lessons learned from tasks
- note: General session notes

Do NOT use for:
- Temporary information that's only relevant to the current task
- Information that's already in files (use the codebase instead)
- Sensitive information (passwords, secrets, etc.)"#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string("memory", "The concise (1-2 sentences) memory to store."),
                ToolParameter::optional_string(
                    "memory_type",
                    "Type of memory: fact, preference, lesson, note. Defaults to 'fact'.",
                ),
                ToolParameter::optional_string(
                    "tags",
                    "Comma-separated tags to categorize the memory (e.g., 'rust,coding,preference').",
                ),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let memory_content = call
            .get_string("memory")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'memory' parameter".to_string()))?;

        let memory_type_str = call.get_string("memory_type").unwrap_or("fact".to_string());
        let tags: Vec<String> = call
            .get_string("tags")
            .map(|s| {
                s.split(',')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        // Parse memory type
        let memory_type = match memory_type_str.to_lowercase().as_str() {
            "fact" => MemoryType::Fact,
            "preference" => MemoryType::Preference,
            "lesson" => MemoryType::Lesson,
            "note" | "custom" => MemoryType::Custom,
            _ => MemoryType::Fact,
        };

        // Get or initialize memory manager
        let manager = ensure_memory_manager().await;

        // Create memory with metadata including tags
        use sage_core::memory::MemoryMetadata;
        let metadata = MemoryMetadata::default().with_tags(tags.clone());
        let memory = Memory::new(memory_type, MemoryCategory::Session, memory_content.clone())
            .with_metadata(metadata);

        let id = manager
            .store(memory)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to store memory: {}", e)))?;

        // Get stats for response
        let stats = manager
            .stats()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get stats: {}", e)))?;

        let response = format!(
            "Memory stored successfully.\n\
             Content: {}\n\
             Type: {}\n\
             ID: {}\n\
             Tags: {}\n\n\
             Total memories: {}, {} pinned",
            memory_content,
            memory_type_str,
            id.as_str(),
            if tags.is_empty() {
                "none".to_string()
            } else {
                tags.join(", ")
            },
            stats.total,
            stats.pinned
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
                meta.insert("memory_id".to_string(), json!(id.as_str()));
                meta.insert("memory_type".to_string(), json!(memory_type_str));
                meta
            },
        })
    }
}

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
        r#"View, search, or manage session notes and memories.

Actions:
- list: Show all memories (optionally filtered by type)
- search: Search memories by text
- delete: Delete a memory by ID
- clear: Clear all memories (use with caution)
- stats: Show memory statistics"#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "action",
                    "Action to perform: list, search, delete, clear, stats",
                ),
                ToolParameter::string("query", "Search query (for 'search' action)"),
                ToolParameter::string(
                    "memory_type",
                    "Filter by type: fact, preference, lesson, note",
                ),
                ToolParameter::string("memory_id", "Memory ID (for 'delete' action)"),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let action = call
            .get_string("action")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'action' parameter".to_string()))?;

        let manager = ensure_memory_manager().await;

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

                use sage_core::memory::MemoryId;
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

/// Get memories for context injection (used by system prompt builder)
pub async fn get_memories_for_context(limit: usize) -> Vec<String> {
    let manager = match GLOBAL_MEMORY_MANAGER.get() {
        Some(m) => m,
        None => return Vec::new(),
    };

    // Get pinned memories first
    let mut memories = Vec::new();

    if let Ok(pinned) = manager.pinned().await {
        for mem in pinned.iter().take(limit) {
            memories.push(format!("[{}] {}", mem.memory_type.name(), mem.content));
        }
    }

    // If we have room, add recent non-pinned memories
    if memories.len() < limit {
        let remaining = limit - memories.len();
        for mem_type in [MemoryType::Fact, MemoryType::Preference, MemoryType::Lesson] {
            if memories.len() >= limit {
                break;
            }
            if let Ok(mems) = manager.find_by_type(mem_type).await {
                for mem in mems.iter().filter(|m| !m.metadata.pinned).take(remaining) {
                    memories.push(format!("[{}] {}", mem.memory_type.name(), mem.content));
                    if memories.len() >= limit {
                        break;
                    }
                }
            }
        }
    }

    memories
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_remember_tool() {
        let tool = RememberTool::new();

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "Remember".to_string(),
            arguments: json!({
                "memory": "User prefers tabs over spaces",
                "memory_type": "preference"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .map(|(k, v)| (k, v))
            .collect(),
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.unwrap().contains("Memory stored"));
    }

    #[tokio::test]
    async fn test_session_notes_list() {
        let remember_tool = RememberTool::new();
        let notes_tool = SessionNotesTool::new();

        // Add a memory first
        let add_call = ToolCall {
            id: "test-1".to_string(),
            name: "Remember".to_string(),
            arguments: json!({
                "memory": "Test memory for listing",
                "memory_type": "fact"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .map(|(k, v)| (k, v))
            .collect(),
            call_id: None,
        };
        remember_tool.execute(&add_call).await.unwrap();

        // List memories
        let list_call = ToolCall {
            id: "test-2".to_string(),
            name: "SessionNotes".to_string(),
            arguments: json!({
                "action": "list"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .map(|(k, v)| (k, v))
            .collect(),
            call_id: None,
        };

        let result = notes_tool.execute(&list_call).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_session_notes_stats() {
        let tool = SessionNotesTool::new();

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "SessionNotes".to_string(),
            arguments: json!({
                "action": "stats"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .map(|(k, v)| (k, v))
            .collect(),
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.unwrap().contains("Memory Statistics"));
    }
}
