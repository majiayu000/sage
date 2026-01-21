//! Remember tool for storing memories

use async_trait::async_trait;
use sage_core::memory::{Memory, MemoryCategory, MemoryMetadata, MemoryType};
use sage_core::tools::{Tool, ToolCall, ToolError, ToolResult, ToolSchema};
use serde_json::json;
use std::collections::HashMap;

use super::schema::remember_schema;
use super::types::ensure_memory_manager;

/// Remember tool for storing memories
#[derive(Debug, Clone)]
pub struct RememberTool;

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
        remember_schema().description
    }

    fn schema(&self) -> ToolSchema {
        remember_schema()
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
        let manager = ensure_memory_manager().await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to initialize memory manager: {}", e))
        })?;

        // Create memory with metadata including tags
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
