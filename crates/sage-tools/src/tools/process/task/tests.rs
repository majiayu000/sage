//! Tests for task tool

#[cfg(test)]
mod suite {
    use serde_json::json;
    use std::sync::Arc;

    use sage_core::agent::subagent::{AgentPath, ChildAgentSpawnRecord, SubAgentGraph};
    use sage_core::thread_store::{SqliteThreadStore, ThreadRecord, ThreadStore};
    use sage_core::tools::base::Tool;
    use sage_core::tools::permission::ToolContext;
    use sage_core::tools::types::ToolCall;

    use super::super::tool::TaskTool;
    use super::super::types::{TaskRegistry, TaskStatus};

    async fn graph_with_parent(
        parent_thread_id: &str,
    ) -> Result<(Arc<SqliteThreadStore>, Arc<SubAgentGraph>), Box<dyn std::error::Error>> {
        let store = Arc::new(SqliteThreadStore::in_memory()?);
        store
            .create_thread(ThreadRecord::new(parent_thread_id))
            .await?;
        let graph_store: Arc<dyn ThreadStore> = store.clone();
        Ok((store, Arc::new(SubAgentGraph::new(graph_store))))
    }

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
            .collect(),
            call_id: None,
        };

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("background"));
        assert!(output.contains("TaskOutput"));

        let Some(task_id) = result
            .metadata
            .get("task_id")
            .and_then(|value| value.as_str())
        else {
            panic!("expected background task id");
        };
        let Some(task) = registry.get_task(task_id) else {
            panic!("expected registered background task");
        };
        assert_ne!(task.status, TaskStatus::Pending);
        assert!(task.run_in_background);
    }

    #[tokio::test]
    async fn test_task_tool_background_with_graph_records_child_edge()
    -> Result<(), Box<dyn std::error::Error>> {
        let registry = Arc::new(TaskRegistry::new());
        let (_store, graph) = graph_with_parent("parent-thread").await?;
        let tool = TaskTool::with_registry_and_graph(registry.clone(), graph.clone());
        let context = ToolContext::new(std::env::current_dir().unwrap_or_default())
            .with_session_id("parent-thread");

        let call = ToolCall {
            id: "spawn-item".to_string(),
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
            .collect(),
            call_id: None,
        };

        let result = tool.execute_with_context(&call, &context).await?;
        assert!(result.success);
        let task_id = result
            .metadata
            .get("task_id")
            .and_then(|value| value.as_str())
            .expect("expected task id");
        let agent_path = result
            .metadata
            .get("agent_path")
            .and_then(|value| value.as_str())
            .expect("expected agent path");
        assert_eq!(
            result
                .metadata
                .get("parent_thread_id")
                .and_then(|value| value.as_str()),
            Some("parent-thread")
        );
        assert_eq!(
            result
                .metadata
                .get("child_thread_id")
                .and_then(|value| value.as_str()),
            Some(task_id)
        );
        assert_eq!(
            result
                .metadata
                .get("spawn_item_id")
                .and_then(|value| value.as_str()),
            Some("spawn-item")
        );

        let summary = graph
            .read_child(&AgentPath::from_raw_path(agent_path)?)
            .await?;
        assert_eq!(summary.parent_thread_id, "parent-thread");
        assert_eq!(summary.child_thread_id, task_id);
        assert_eq!(summary.spawn_item_id, "spawn-item");
        assert!(registry.get_task(task_id).is_some());
        Ok(())
    }

    #[tokio::test]
    async fn test_task_tool_background_graph_missing_parent_fails_before_registering()
    -> Result<(), Box<dyn std::error::Error>> {
        let registry = Arc::new(TaskRegistry::new());
        let store = Arc::new(SqliteThreadStore::in_memory()?);
        let graph_store: Arc<dyn ThreadStore> = store;
        let graph = Arc::new(SubAgentGraph::new(graph_store));
        let tool = TaskTool::with_registry_and_graph(registry.clone(), graph);
        let context = ToolContext::new(std::env::current_dir().unwrap_or_default())
            .with_session_id("missing-parent");

        let call = ToolCall {
            id: "spawn-item".to_string(),
            name: "Task".to_string(),
            arguments: json!({
                "description": "Plan implementation",
                "prompt": "Design authentication system",
                "subagent_type": "Plan",
                "run_in_background": true,
                "resume": "task_missing_parent"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
            call_id: None,
        };

        let err = tool
            .execute_with_context(&call, &context)
            .await
            .expect_err("missing parent must fail");
        assert!(err.to_string().contains("missing-parent"));
        assert!(registry.get_task("task_missing_parent").is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_task_tool_background_with_graph_without_context_uses_task_registry()
    -> Result<(), Box<dyn std::error::Error>> {
        let registry = Arc::new(TaskRegistry::new());
        let (_store, graph) = graph_with_parent("parent-thread").await?;
        let tool = TaskTool::with_registry_and_graph(registry.clone(), graph.clone());

        let call = ToolCall {
            id: "spawn-item".to_string(),
            name: "Task".to_string(),
            arguments: json!({
                "description": "Plan implementation",
                "prompt": "Design authentication system",
                "subagent_type": "Plan",
                "run_in_background": true,
                "resume": "task_without_context"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
            call_id: None,
        };

        let result = tool.execute(&call).await?;
        assert!(result.success);
        assert_eq!(
            result
                .metadata
                .get("task_id")
                .and_then(|value| value.as_str()),
            Some("task_without_context")
        );
        assert!(!result.metadata.contains_key("agent_path"));
        assert!(registry.get_task("task_without_context").is_some());
        assert!(
            graph
                .read_child(&AgentPath::from_raw_path("agent://task_without_context")?)
                .await
                .is_err()
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_task_tool_background_with_graph_reuses_resume_edge()
    -> Result<(), Box<dyn std::error::Error>> {
        let registry = Arc::new(TaskRegistry::new());
        let (_store, graph) = graph_with_parent("parent-thread").await?;
        graph
            .record_child(ChildAgentSpawnRecord::new(
                "parent-thread",
                "task_existing",
                "original-spawn",
            ))
            .await?;
        let tool = TaskTool::with_registry_and_graph(registry.clone(), graph.clone());
        let context = ToolContext::new(std::env::current_dir().unwrap_or_default())
            .with_session_id("parent-thread");

        let call = ToolCall {
            id: "new-spawn".to_string(),
            name: "Task".to_string(),
            arguments: json!({
                "description": "Plan implementation",
                "prompt": "Design authentication system",
                "subagent_type": "Plan",
                "run_in_background": true,
                "resume": "task_existing"
            })
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
            call_id: None,
        };

        let result = tool.execute_with_context(&call, &context).await?;
        assert!(result.success);
        assert_eq!(
            result
                .metadata
                .get("task_id")
                .and_then(|value| value.as_str()),
            Some("task_existing")
        );
        assert_eq!(
            result
                .metadata
                .get("spawn_item_id")
                .and_then(|value| value.as_str()),
            Some("original-spawn")
        );

        let summary = graph
            .read_child(&AgentPath::from_raw_path("agent://task_existing")?)
            .await?;
        assert_eq!(summary.spawn_item_id, "original-spawn");
        assert!(registry.get_task("task_existing").is_some());
        Ok(())
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
