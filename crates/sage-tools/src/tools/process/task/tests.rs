//! Tests for task tool

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::sync::Arc;

    use sage_core::tools::base::Tool;
    use sage_core::tools::types::ToolCall;

    use super::super::tool::TaskTool;
    use super::super::types::{TaskRegistry, TaskStatus};

    #[tokio::test]
    async fn test_task_tool_basic() {
        let registry = Arc::new(TaskRegistry::new());
        let tool = TaskTool::with_registry(registry.clone());

        let call = ToolCall {
            id: "test-1".to_string(),
            name: "Task".to_string(),
            arguments: json!({
                "description": "Search codebase",
                "prompt": "Find all files related to authentication",
                "subagent_type": "Explore"
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

        // When global runner is not initialized, the task will fail with a helpful message
        // This is expected in test environments without full agent setup
        if !result.success {
            let error = result.error.unwrap();
            assert!(
                error.contains("not initialized") || error.contains("Runner not initialized"),
                "Expected 'not initialized' error, got: {}",
                error
            );

            // Get task_id from metadata
            if let Some(task_id) = result.metadata.get("task_id").and_then(|v| v.as_str()) {
                // Verify task was registered and marked as failed
                let task = registry.get_task(task_id);
                assert!(
                    task.map(|t| t.status == TaskStatus::Failed)
                        .unwrap_or(false)
                );
            }
        } else {
            // If runner is initialized, verify successful execution
            assert!(result.output.unwrap().contains("Explore"));
        }
    }

    #[tokio::test]
    async fn test_task_tool_background() {
        let registry = Arc::new(TaskRegistry::new());
        let tool = TaskTool::with_registry(registry.clone());

        let call = ToolCall {
            id: "test-2".to_string(),
            name: "Task".to_string(),
            arguments: json!({
                "description": "Plan implementation",
                "prompt": "Design authentication system",
                "subagent_type": "Plan",
                "run_in_background": true,
                "model": "opus"
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
        let output = result.output.unwrap();
        assert!(output.contains("background"));
        assert!(output.contains("TaskOutput"));
    }

    #[tokio::test]
    async fn test_task_registry() {
        use super::super::types::TaskRequest;

        let registry = TaskRegistry::new();

        let task = TaskRequest {
            id: "task-1".to_string(),
            description: "Test task".to_string(),
            prompt: "Do something".to_string(),
            subagent_type: "Explore".to_string(),
            model: None,
            run_in_background: false,
            resume: None,
            status: TaskStatus::Pending,
            result: None,
        };

        registry.add_task(task);

        // Get task
        let retrieved = registry.get_task("task-1").unwrap();
        assert_eq!(retrieved.description, "Test task");
        assert_eq!(retrieved.status, TaskStatus::Pending);

        // Update status
        registry.update_status("task-1", TaskStatus::Completed, Some("Done!".to_string()));

        let updated = registry.get_task("task-1").unwrap();
        assert_eq!(updated.status, TaskStatus::Completed);
        assert_eq!(updated.result, Some("Done!".to_string()));

        // Pending tasks should be empty now
        let pending = registry.get_pending_tasks();
        assert_eq!(pending.len(), 0);
    }
}
